use crate::{
    ArtifactReader, ArtifactVerificationError, RpcLaunchQueryRepository, VerifiedArtifactContent,
    verify_artifact,
};
use graph_domain::{RpcLaunchSpec, RpcTerminalReceipt};

/// Fresh byte observations only; not RPC success, cleanup or retry authority.
pub struct RpcOutputObservation {
    receipt: RpcTerminalReceipt,
    stdout: Option<VerifiedArtifactContent>,
    stderr: Option<VerifiedArtifactContent>,
}
impl std::fmt::Debug for RpcOutputObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RpcOutputObservation { .. }")
    }
}
impl RpcOutputObservation {
    pub fn receipt(&self) -> &RpcTerminalReceipt {
        &self.receipt
    }
    pub fn stdout(&self) -> Option<&VerifiedArtifactContent> {
        self.stdout.as_ref()
    }
    pub fn stderr(&self) -> Option<&VerifiedArtifactContent> {
        self.stderr.as_ref()
    }
}

#[derive(thiserror::Error)]
pub enum RpcOutputError<E: std::error::Error + Send + Sync + 'static> {
    #[error("RPC launch or terminal receipt unavailable")]
    Unavailable,
    #[error("RPC launch differs from host-selected expectation")]
    ExpectationMismatch,
    #[error("RPC outputs exceed total byte budget")]
    TooLarge,
    #[error("RPC inspection failed")]
    Repository(#[source] E),
    #[error("RPC output content verification failed")]
    Content(#[source] ArtifactVerificationError),
    #[error("RPC snapshot changed while verifying outputs")]
    Changed,
}
impl<E: std::error::Error + Send + Sync + 'static> std::fmt::Debug for RpcOutputError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// Host selects expected launch and scoped reader root; never derive authority
/// from the returned receipt. Budget excludes one sentinel byte per stream.
/// No writes, process control or post-return lock; host bounds blocking reader time.
pub fn verify_rpc_outputs<R: RpcLaunchQueryRepository, B: ArtifactReader>(
    repository: &R,
    reader: &B,
    expected: &RpcLaunchSpec,
    max_total_bytes: u64,
) -> Result<RpcOutputObservation, RpcOutputError<R::Error>> {
    let before = repository
        .rpc_launch_snapshot(expected.id(), expected.task())
        .map_err(RpcOutputError::Repository)?
        .ok_or(RpcOutputError::Unavailable)?;
    if before.spec() != expected {
        return Err(RpcOutputError::ExpectationMismatch);
    }
    let receipt = before
        .terminal_receipt()
        .ok_or(RpcOutputError::Unavailable)?;
    let total = [receipt.stdout(), receipt.stderr()]
        .into_iter()
        .flatten()
        .try_fold(0_u64, |sum, a| sum.checked_add(a.byte_length()))
        .ok_or(RpcOutputError::TooLarge)?;
    if total > max_total_bytes {
        return Err(RpcOutputError::TooLarge);
    }
    let stdout = receipt
        .stdout()
        .map(|a| verify_artifact(reader, a, max_total_bytes))
        .transpose()
        .map_err(RpcOutputError::Content)?;
    let stderr = receipt
        .stderr()
        .map(|a| verify_artifact(reader, a, max_total_bytes))
        .transpose()
        .map_err(RpcOutputError::Content)?;
    if repository
        .rpc_launch_snapshot(expected.id(), expected.task())
        .map_err(RpcOutputError::Repository)?
        .as_ref()
        != Some(&before)
    {
        return Err(RpcOutputError::Changed);
    }
    Ok(RpcOutputObservation {
        receipt: receipt.clone(),
        stdout,
        stderr,
    })
}
