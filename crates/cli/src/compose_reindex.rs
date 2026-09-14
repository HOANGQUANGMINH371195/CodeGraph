//! Sequential single-owner reconciliation. No source I/O inside DB transactions.
use graph_application::{
    DeploymentRepository, SourceLimits, SourceReader, capture_source, verify_source,
};
use graph_domain::{SourceEvidence, deployment::DeploymentScope};
use graph_protocol::{SCHEMA_VERSION, deployment::ComposeReindex};
use graph_source::DirectorySource;
use graph_store::Store;
use std::{error::Error, io::Write, path::Path, time::Duration};

pub struct ReindexOptions<'a> {
    pub id: &'a str,
    pub task_spec: &'a Path,
    pub root: &'a Path,
    pub analysis_run: &'a str,
    pub expected: u64,
    pub max_source_bytes: u32,
    pub max_output_bytes: u32,
}

fn locator(store: &Store, options: &ReindexOptions<'_>) -> Result<SourceEvidence, Box<dyn Error>> {
    if options.analysis_run.trim().is_empty()
        || options.analysis_run.len() > 128
        || options.analysis_run.chars().any(char::is_control)
    {
        return Err(std::io::Error::other(
            "analysis run must be 1 to 128 bytes without control characters",
        )
        .into());
    }
    crate::compose::selected_evidence(store, options.id, options.task_spec)
}

fn analyze_observed(
    reader: &impl SourceReader,
    locator: &SourceEvidence,
    run: &str,
    limits: SourceLimits,
) -> Result<graph_system::DeploymentGraph, &'static str> {
    let source =
        capture_source(reader, locator, run, limits).map_err(|_| "source_capture_failed")?;
    let graph =
        graph_system::analyze_compose_graph(&source).map_err(|_| "compose_analysis_failed")?;
    verify_source(reader, graph.evidence(), limits)
        .map_err(|_| "source_changed_during_analysis")?;
    Ok(graph)
}

fn cycle(
    store: &mut Store,
    locator: &SourceEvidence,
    options: &ReindexOptions<'_>,
    expected: u64,
) -> Result<(Vec<u8>, u64, bool), Box<dyn Error>> {
    let scope = DeploymentScope::new("compose".into(), locator)?;
    let before = store.deployment(&scope)?;
    if before.as_ref().map_or(0, |s| s.generation) != expected {
        return Err(std::io::Error::other(
            "deployment generation conflict; inspect deployment before retrying",
        )
        .into());
    }
    let limits = SourceLimits::new(
        options.max_source_bytes as usize,
        options.max_source_bytes as usize,
    )?;
    // Errors here refer to this explicitly selected source; never turn a task,
    // database or output-encoding error into successful invalidation.
    let result = DirectorySource::open(
        options.root,
        locator.project().clone(),
        locator.graph_version().into(),
    )
    .map_err(|_| "source_unavailable")
    .and_then(|reader| analyze_observed(&reader, locator, options.analysis_run, limits));
    let source_valid = result.is_ok();
    let reason = result.as_ref().err().copied();
    let graph = result.ok();
    let unchanged = before.as_ref().is_some_and(|s| s.graph == graph);
    let generation = if unchanged {
        expected
    } else {
        expected
            .checked_add(1)
            .filter(|n| i64::try_from(*n).is_ok())
            .ok_or_else(|| std::io::Error::other("deployment generation exhausted"))?
    };
    let bytes = crate::output::json_line(
        &ComposeReindex {
            schema_version: SCHEMA_VERSION,
            kind: "compose_reindex",
            status: if !source_valid {
                "invalidated"
            } else if unchanged {
                "unchanged"
            } else {
                "published"
            },
            generation,
            source_valid,
            changed: !unchanged,
            historical: true,
            snapshot_binding: "caller_supplied",
            relationship_verified: false,
            reason,
        },
        options.max_output_bytes as usize,
    )?;
    if unchanged {
        // Recheck after source work so an observed competing commit is not
        // reported as a successful no-op against the caller's old generation.
        if store
            .deployment(&scope)?
            .as_ref()
            .map_or(0, |s| s.generation)
            != expected
        {
            return Err(std::io::Error::other(
                "deployment generation conflict; inspect deployment before retrying",
            )
            .into());
        }
    } else if let Some(graph) = &graph {
        store.replace_deployment(graph, expected)?;
    } else {
        store.invalidate_deployment(&scope, expected)?;
    }
    Ok((bytes, generation, source_valid))
}

pub fn reindex(
    store: &mut Store,
    options: &ReindexOptions<'_>,
) -> Result<(Vec<u8>, bool), Box<dyn Error>> {
    let locator = locator(store, options)?;
    let (bytes, _, valid) = cycle(store, &locator, options, options.expected)?;
    Ok((bytes, valid))
}

pub fn watch(
    store: &mut Store,
    options: &ReindexOptions<'_>,
    interval_ms: u64,
    max_cycles: Option<u32>,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let locator = locator(store, options)?;
    let mut generation = options.expected;
    let mut cycles = 0_u64;
    loop {
        let (bytes, next, _) = cycle(store, &locator, options, generation)?;
        output.write_all(&bytes)?;
        output.flush()?;
        generation = next;
        cycles += 1;
        if max_cycles.is_some_and(|max| cycles >= u64::from(max)) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(interval_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct ChangingReader {
        reads: Cell<usize>,
        change: bool,
    }
    impl SourceReader for ChangingReader {
        type Error = std::io::Error;
        fn read_source(&self, _: &SourceEvidence, _: usize) -> Result<Vec<u8>, Self::Error> {
            let count = self.reads.get();
            self.reads.set(count + 1);
            Ok(if self.change && count > 0 {
                b"services: {new: {}}\n".to_vec()
            } else {
                b"services: {old: {}}\n".to_vec()
            })
        }
    }

    #[test]
    fn changed_bytes_between_capture_and_verification_never_return_publishable_graph() {
        let locator = SourceEvidence::new(
            "old".into(),
            graph_domain::ProjectRef {
                repository_id: "r".into(),
                worktree_id: "w".into(),
                git_head: "h".into(),
                working_tree_fingerprint: "f".into(),
                config_hash: "c".into(),
                ignore_policy_version: "1".into(),
            },
            "g".into(),
            "compose.yaml".into(),
            "a".repeat(64),
            1,
            1,
            "old-run".into(),
        )
        .unwrap();
        let limits = SourceLimits::new(1024, 1024).unwrap();
        for change in [false, true] {
            let reader = ChangingReader {
                reads: Cell::new(0),
                change,
            };
            let result = analyze_observed(&reader, &locator, "run", limits);
            assert_eq!(reader.reads.get(), 2);
            if change {
                assert_eq!(result.unwrap_err(), "source_changed_during_analysis");
            } else {
                assert_eq!(result.unwrap().nodes()[0].name, "old");
            }
        }
    }
}
