use crate::BoundRpcTerminal;
use graph_application::{
    AnalysisRepository, ArtifactIngestion, ArtifactIngestionError, ArtifactRepository,
    ArtifactWriter, RpcSpawnObservationRepository, RpcTerminalReceiptRepository, ingest_artifact,
};
use graph_domain::{
    AnalysisRun, Artifact, ArtifactProtection, ArtifactRetention, DomainError, RpcInputProgress,
    RpcTerminalReceipt, UncertainRpc,
};
use sha2::{Digest, Sha256};

/// Reopens historical journal accounting and verifies its referenced bytes.
/// Host owns the manifest handle, scoped reader and I/O deadline. No process
/// control or ledger write is performed; files may change after this returns.
pub fn reopen_rpc_journal<B: graph_application::ArtifactReader>(
    reader: &B,
    manifest: &Artifact,
    expected: &graph_domain::RpcLaunchSpec,
    max_manifest_bytes: usize,
    max_output_bytes: u64,
) -> Result<RpcTerminalReceipt, Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Read;
    if manifest.kind() != "rpc_journal_manifest"
        || manifest.project() != expected.execution_snapshot()
        || manifest.graph_version() != expected.task().graph_version()
        || manifest.byte_length() > max_manifest_bytes as u64
    {
        return Err(DomainError::Invalid("journal handle scope or size mismatch").into());
    }
    let mut bytes = Vec::new();
    reader
        .open_artifact(manifest)?
        .take(manifest.byte_length() + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != manifest.byte_length()
        || format!("{:x}", Sha256::digest(&bytes)) != manifest.content_sha256()
    {
        return Err(DomainError::Invalid("journal manifest content mismatch").into());
    }
    let receipt = graph_protocol::rpc_journal::decode(
        &bytes,
        expected,
        max_manifest_bytes,
        max_output_bytes,
    )?;
    if manifest.analysis_run() != receipt.output_run().id() {
        return Err(DomainError::Invalid("journal manifest run mismatch").into());
    }
    for artifact in [receipt.stdout(), receipt.stderr()].into_iter().flatten() {
        graph_application::verify_artifact(reader, artifact, max_output_bytes)?;
    }
    Ok(receipt)
}

/// Replays journal accounting without claiming or spawning a process.
/// Writes are staged and may partially commit; retry only the same journal.
pub fn replay_rpc_journal<R, B>(
    repository: &mut R,
    reader: &B,
    manifest: &Artifact,
    expected: &graph_domain::RpcLaunchSpec,
    max_manifest_bytes: usize,
    max_output_bytes: u64,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>
where
    R: ArtifactRepository
        + AnalysisRepository<Error = <R as ArtifactRepository>::Error>
        + RpcSpawnObservationRepository<Error = <R as ArtifactRepository>::Error>
        + RpcTerminalReceiptRepository<Error = <R as ArtifactRepository>::Error>,
    B: graph_application::ArtifactReader,
{
    let receipt = reopen_rpc_journal(
        reader,
        manifest,
        expected,
        max_manifest_bytes,
        max_output_bytes,
    )?;
    if repository.rpc_spawn_observation(expected)?.as_ref() != Some(receipt.spawn())
        || repository
            .rpc_terminal_receipt(expected)?
            .is_some_and(|old| old != receipt)
    {
        return Err(DomainError::Invalid("journal replay prerequisite mismatch").into());
    }
    repository.record_analysis_run(receipt.output_run())?;
    for artifact in [receipt.stdout(), receipt.stderr()].into_iter().flatten() {
        repository.record_artifact(artifact)?;
    }
    Ok(repository.record_rpc_terminal_receipt(&receipt)?)
}

/// Owns immutable descriptors, borrows original evidence and any unreaped child.
pub struct PreparedRpcReceipt<'a> {
    terminal: &'a BoundRpcTerminal,
    receipt: RpcTerminalReceipt,
}
impl std::fmt::Debug for PreparedRpcReceipt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedRpcReceipt { .. }")
    }
}

#[derive(Debug)]
pub struct RpcReceiptPublication {
    pub receipt_inserted: bool,
    pub stdout: ArtifactIngestion,
    pub stderr: ArtifactIngestion,
}

#[derive(thiserror::Error)]
pub enum RpcPublicationError<E: std::error::Error + Send + Sync + 'static> {
    #[error("RPC prerequisite lookup failed")]
    Lookup(#[source] E),
    #[error("stored spawn or terminal report differs; no output publication attempted")]
    Mismatch,
    #[error("RPC output run registration failed")]
    Run(#[source] E),
    #[error("RPC stdout publication failed; retry same prepared receipt")]
    Stdout(#[source] ArtifactIngestionError<E>),
    #[error("RPC stdout verified; stderr publication failed; retry same prepared receipt")]
    Stderr {
        stdout: ArtifactIngestion,
        #[source]
        source: ArtifactIngestionError<E>,
    },
    #[error("RPC outputs verified; receipt recording failed and may have committed")]
    Receipt {
        stdout: ArtifactIngestion,
        stderr: ArtifactIngestion,
        #[source]
        source: E,
    },
}
impl<E: std::error::Error + Send + Sync + 'static> std::fmt::Debug for RpcPublicationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl BoundRpcTerminal {
    /// No storage side effects. Time/analysis identity are host-selected claims;
    /// raw output is never advertised as redacted or as RPC task success.
    pub fn prepare_receipt(
        &self,
        run: AnalysisRun,
        stdout_id: String,
        stderr_id: String,
        finished_at_ms: i64,
        max_total_bytes: u64,
    ) -> Result<PreparedRpcReceipt<'_>, DomainError> {
        let terminal = self.terminal();
        let launch = self.spawn_observation().launch();
        if terminal.requests.epoch() != launch.connection().epoch()
            || terminal
                .output
                .stdout
                .len()
                .checked_add(terminal.output.stderr.len())
                .is_none_or(|n| n as u64 > max_total_bytes)
            || terminal.output.stdout_state != terminal.completion.stdout
            || terminal.output.stderr_state != terminal.completion.stderr
        {
            return Err(DomainError::Invalid(
                "RPC terminal binding or output budget mismatch",
            ));
        }
        let output = |id: String, bytes: &[u8], kind: &str| {
            Artifact::new(
                id,
                run.project().clone(),
                run.graph_version().into(),
                run.id().into(),
                format!("{:x}", Sha256::digest(bytes)),
                bytes.len() as u64,
                kind.into(),
                ArtifactRetention::Evidence,
                ArtifactProtection::Unreviewed,
            )
        };
        let stdout = output(stdout_id, &terminal.output.stdout, "stdout")?;
        let stderr = output(stderr_id, &terminal.output.stderr, "stderr")?;
        let pending = terminal
            .requests
            .unresolved()
            .map(|(id, pending)| {
                if !pending.uncertain() {
                    return Err(DomainError::Invalid("unsealed terminal request"));
                }
                UncertainRpc::new(id, pending.method().into())
            })
            .collect::<Result<_, _>>()?;
        let progress = terminal
            .input_progress
            .map(|p| RpcInputProgress::new(p.total as u64, p.written as u64, p.failed))
            .transpose()?;
        let receipt = RpcTerminalReceipt::new(
            self.spawn_observation().clone(),
            run,
            finished_at_ms,
            terminal.elapsed_ms,
            Some(stdout),
            Some(stderr),
            terminal.completion,
            pending,
            progress,
        )?;
        Ok(PreparedRpcReceipt {
            terminal: self,
            receipt,
        })
    }
}

impl PreparedRpcReceipt<'_> {
    /// Stages the journal and retains its handle in the artifact ledger.
    /// Host must select a stable manifest ID for scoped lookup after restart.
    /// Failure may leave CAS blobs or a run row; retry the identical preparation.
    pub fn stage_registered_journal<R, W>(
        &self,
        repository: &mut R,
        writer: &W,
        manifest_id: String,
        max_manifest_bytes: usize,
        max_output_bytes: u64,
    ) -> Result<Artifact, Box<dyn std::error::Error + Send + Sync>>
    where
        R: ArtifactRepository
            + AnalysisRepository<Error = <R as ArtifactRepository>::Error>
            + RpcSpawnObservationRepository<Error = <R as ArtifactRepository>::Error>
            + RpcTerminalReceiptRepository<Error = <R as ArtifactRepository>::Error>,
        W: ArtifactWriter,
    {
        let receipt = &self.receipt;
        let launch = receipt.spawn().launch();
        if repository.rpc_spawn_observation(launch)?.as_ref() != Some(receipt.spawn())
            || repository
                .rpc_terminal_receipt(launch)?
                .is_some_and(|old| old != *receipt)
        {
            return Err(DomainError::Invalid("journal registration prerequisite mismatch").into());
        }
        let manifest =
            self.stage_journal(writer, manifest_id, max_manifest_bytes, max_output_bytes)?;
        repository.record_analysis_run(receipt.output_run())?;
        repository.record_artifact(&manifest)?;
        Ok(manifest)
    }

    /// Stages outputs then manifest through the host-selected CAS writer.
    /// Caller must retain the returned descriptor; no ledger or process action
    /// occurs. A failed operation may leave verified blobs for later replay.
    pub fn stage_journal<W: ArtifactWriter>(
        &self,
        writer: &W,
        manifest_id: String,
        max_manifest_bytes: usize,
        max_output_bytes: u64,
    ) -> Result<Artifact, Box<dyn std::error::Error + Send + Sync>> {
        let receipt = &self.receipt;
        let bytes = graph_protocol::rpc_journal::encode(
            receipt,
            receipt.spawn().launch(),
            max_manifest_bytes,
            max_output_bytes,
        )?;
        let run = receipt.output_run();
        let manifest = Artifact::new(
            manifest_id,
            run.project().clone(),
            run.graph_version().into(),
            run.id().into(),
            format!("{:x}", Sha256::digest(&bytes)),
            bytes.len() as u64,
            "rpc_journal_manifest".into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )?;
        let output = &self.terminal.terminal().output;
        let (Some(stdout), Some(stderr)) = (receipt.stdout(), receipt.stderr()) else {
            return Err(DomainError::Invalid("journal requires both streams").into());
        };
        for (artifact, mut input) in [
            (stdout, output.stdout.as_slice()),
            (stderr, output.stderr.as_slice()),
            (&manifest, bytes.as_slice()),
        ] {
            writer.write_artifact(artifact, &mut input, artifact.byte_length())?;
            graph_application::verify_artifact(writer, artifact, artifact.byte_length())?;
        }
        Ok(manifest)
    }

    pub fn receipt(&self) -> &RpcTerminalReceipt {
        &self.receipt
    }
    /// Replayable staged writes, not an atomic blob+database transaction.
    /// Caller owns root, adapter deadlines and cleanup; no retry/spawn is implicit.
    pub fn publish<R, W>(
        &self,
        repository: &mut R,
        writer: &W,
    ) -> Result<RpcReceiptPublication, RpcPublicationError<<R as ArtifactRepository>::Error>>
    where
        R: ArtifactRepository
            + AnalysisRepository<Error = <R as ArtifactRepository>::Error>
            + RpcSpawnObservationRepository<Error = <R as ArtifactRepository>::Error>
            + RpcTerminalReceiptRepository<Error = <R as ArtifactRepository>::Error>,
        W: ArtifactWriter,
    {
        let r = &self.receipt;
        if repository
            .rpc_spawn_observation(r.spawn().launch())
            .map_err(RpcPublicationError::Lookup)?
            .as_ref()
            != Some(r.spawn())
        {
            return Err(RpcPublicationError::Mismatch);
        }
        if repository
            .rpc_terminal_receipt(r.spawn().launch())
            .map_err(RpcPublicationError::Lookup)?
            .is_some_and(|old| old != *r)
        {
            return Err(RpcPublicationError::Mismatch);
        }
        repository
            .record_analysis_run(r.output_run())
            .map_err(RpcPublicationError::Run)?;
        // Private preparation always supplies both descriptors, even empty streams.
        let (Some(out), Some(err)) = (r.stdout(), r.stderr()) else {
            return Err(RpcPublicationError::Mismatch);
        };
        let output = &self.terminal.terminal().output;
        let stdout = ingest_artifact(
            repository,
            writer,
            out,
            &mut output.stdout.as_slice(),
            out.byte_length(),
        )
        .map_err(RpcPublicationError::Stdout)?;
        let stderr = match ingest_artifact(
            repository,
            writer,
            err,
            &mut output.stderr.as_slice(),
            err.byte_length(),
        ) {
            Ok(value) => value,
            Err(source) => return Err(RpcPublicationError::Stderr { stdout, source }),
        };
        match repository.record_rpc_terminal_receipt(r) {
            Ok(receipt_inserted) => Ok(RpcReceiptPublication {
                receipt_inserted,
                stdout,
                stderr,
            }),
            Err(source) => Err(RpcPublicationError::Receipt {
                stdout,
                stderr,
                source,
            }),
        }
    }
}
