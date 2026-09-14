#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use graph_application::environment_fingerprint;
use graph_domain::execution::{ChildCompletion, ScopeCleanup, StopReason, StreamCompletion};
use graph_domain::{CheckCommand, CheckOutcome};
use graph_execution::{FixtureError, FixtureRun, run_trusted_fixture};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn executable() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .canonicalize()
        .unwrap()
}

fn command(mode: &str, environment: &BTreeMap<String, String>) -> CheckCommand {
    let exe = executable();
    CheckCommand::new(
        exe.to_str().unwrap().to_owned(),
        vec![mode.to_owned()],
        ".".to_owned(),
        format!("{:x}", Sha256::digest(std::fs::read(&exe).unwrap())),
        environment_fingerprint(environment).unwrap(),
        5000,
        1000,
        4096,
        4096,
    )
    .unwrap()
}

fn assert_reaped(run: &FixtureRun) {
    assert!(run.unreaped_child.is_none(), "owned child left unreaped");
    assert!(matches!(
        run.completion.child,
        ChildCompletion::Reaped { .. }
    ));
    assert_eq!(run.completion.cleanup, ScopeCleanup::Unverifiable);
}

#[test]
fn exact_and_zero_byte_caps_distinguish_eof_from_overflow() {
    let environment = BTreeMap::new();
    for (mode, stdout_cap, stderr_cap, truncated) in [
        ("binary", 23, 8, false),
        ("binary", 22, 8, true),
        ("binary", 0, 0, true),
        ("unknown-mode", 0, 0, false),
    ] {
        let root = tempfile::tempdir().unwrap();
        let base = command(mode, &environment);
        let command = CheckCommand::new(
            base.program().to_owned(),
            base.args().to_vec(),
            ".".to_owned(),
            base.executable_sha256().to_owned(),
            base.environment_sha256().to_owned(),
            5000,
            1000,
            stdout_cap,
            stderr_cap,
        )
        .unwrap();
        let run = run_trusted_fixture(
            &command,
            &executable(),
            root.path(),
            &environment,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_reaped(&run);
        assert!(run.stdout.len() as u64 <= stdout_cap);
        assert!(run.stderr.len() as u64 <= stderr_cap);
        if truncated {
            assert_eq!(run.completion.reason, StopReason::OutputLimit);
            assert_eq!(run.completion.stdout, StreamCompletion::Truncated);
        } else {
            assert_eq!(run.completion.reason, StopReason::Exited);
            assert_eq!(run.completion.stdout, StreamCompletion::Complete);
            assert_eq!(run.completion.stderr, StreamCompletion::Complete);
            if mode == "binary" {
                assert_eq!(run.stdout, b"\0\xffout\r\nno-final-newline");
                assert_eq!(run.stderr, b"\xfe\0err\n\r\x80");
            }
        }
    }
}

#[test]
fn pre_cancel_and_escaping_cwd_never_spawn() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    let base = command("sleep", &environment);
    let run = run_trusted_fixture(
        &base,
        &executable(),
        root.path(),
        &environment,
        &AtomicBool::new(true),
    )
    .unwrap();
    assert_eq!(run.completion.reason, StopReason::Cancelled);
    assert_eq!(run.completion.child, ChildCompletion::NotSpawned);
    assert!(run.unreaped_child.is_none());
    assert!(!root.path().join("ready").exists());

    std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
    let command = CheckCommand::new(
        base.program().to_owned(),
        base.args().to_vec(),
        "escape".to_owned(),
        base.executable_sha256().to_owned(),
        base.environment_sha256().to_owned(),
        5000,
        1000,
        4096,
        4096,
    )
    .unwrap();
    assert!(matches!(
        run_trusted_fixture(
            &command,
            &executable(),
            root.path(),
            &environment,
            &AtomicBool::new(false)
        ),
        Err(FixtureError::Preflight)
    ));
    assert!(!outside.path().join("ready").exists());
}

#[test]
fn simultaneous_pipe_pressure_preserves_both_streams_at_exact_caps() {
    let root = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    let base = command("flood-both", &environment);
    let cap = 64 * 8192;
    let command = CheckCommand::new(
        base.program().to_owned(),
        base.args().to_vec(),
        ".".to_owned(),
        base.executable_sha256().to_owned(),
        base.environment_sha256().to_owned(),
        5000,
        1000,
        cap,
        cap,
    )
    .unwrap();
    let run = run_trusted_fixture(
        &command,
        &executable(),
        root.path(),
        &environment,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_reaped(&run);
    assert_eq!(run.completion.reason, StopReason::Exited);
    assert_eq!(
        run.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(run.completion.stdout, StreamCompletion::Complete);
    assert_eq!(run.completion.stderr, StreamCompletion::Complete);
    assert_eq!(run.stdout, vec![0xa5; cap as usize]);
    assert_eq!(run.stderr, vec![0x5a; cap as usize]);
}

#[test]
fn exact_argv_canonical_cwd_explicit_environment_and_closed_stdin() {
    use std::os::unix::ffi::OsStrExt;

    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("work")).unwrap();
    std::os::unix::fs::symlink("work", root.path().join("inside")).unwrap();
    let environment = BTreeMap::from([
        (
            "GRAPH_FIXTURE_VALUE".to_owned(),
            "literal $HOME\n雪".to_owned(),
        ),
        ("GRAPH_FIXTURE_EMPTY".to_owned(), String::new()),
    ]);
    let base = command("context", &environment);
    let args = [
        "context",
        "",
        "two words",
        "'quoted'",
        "$HOME;*",
        "line\nbreak",
        "雪",
    ];
    let command = CheckCommand::new(
        base.program().to_owned(),
        args.iter().map(|arg| (*arg).to_owned()).collect(),
        "inside".to_owned(),
        base.executable_sha256().to_owned(),
        base.environment_sha256().to_owned(),
        base.timeout_ms(),
        base.cleanup_timeout_ms(),
        base.stdout_max_bytes(),
        base.stderr_max_bytes(),
    )
    .unwrap();
    let run = run_trusted_fixture(
        &command,
        &executable(),
        root.path(),
        &environment,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_reaped(&run);
    let mut expected = Vec::new();
    for field in std::iter::once(command.program()).chain(args) {
        expected.extend_from_slice(field.as_bytes());
        expected.push(0);
    }
    expected.extend_from_slice(
        root.path()
            .join("work")
            .canonicalize()
            .unwrap()
            .as_os_str()
            .as_bytes(),
    );
    expected.extend_from_slice(b"\0");
    expected.extend_from_slice("2\0literal $HOME\n雪\0\0stdin=0\0".as_bytes());
    assert_eq!(run.stdout, expected);
    assert!(run.stderr.is_empty());
    assert_eq!(run.completion.reason, StopReason::Exited);
    assert_eq!(
        run.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(run.completion.stdout, StreamCompletion::Complete);
    assert_eq!(run.completion.stderr, StreamCompletion::Complete);
    assert_eq!(
        run.completion.reported_check_outcome(),
        CheckOutcome::Unknown
    );
}

#[test]
fn binary_streams_and_nonzero_status_remain_exact() {
    let root = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    let run = run_trusted_fixture(
        &command("binary", &environment),
        &executable(),
        root.path(),
        &environment,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_reaped(&run);
    assert_eq!(run.stdout, b"\0\xffout\r\nno-final-newline");
    assert_eq!(run.stderr, b"\xfe\0err\n\r\x80");
    assert_eq!(run.completion.reason, StopReason::Exited);
    assert_eq!(
        run.completion.child,
        ChildCompletion::Reaped {
            exit_code: Some(23)
        }
    );
    assert_eq!(run.completion.stdout, StreamCompletion::Complete);
    assert_eq!(run.completion.stderr, StreamCompletion::Complete);
    assert_eq!(
        run.completion.reported_check_outcome(),
        CheckOutcome::Unknown
    );
}

#[test]
fn either_output_stream_enforces_its_cap() {
    for mode in ["flood-stdout", "flood-stderr"] {
        let root = tempfile::tempdir().unwrap();
        let environment = BTreeMap::new();
        let command = command(mode, &environment);
        let run = run_trusted_fixture(
            &command,
            &executable(),
            root.path(),
            &environment,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_reaped(&run);
        assert_eq!(run.completion.reason, StopReason::OutputLimit, "{mode}");
        assert!(run.stdout.len() as u64 <= command.stdout_max_bytes());
        assert!(run.stderr.len() as u64 <= command.stderr_max_bytes());
        let (flooded, other, completion) = if mode == "flood-stdout" {
            (&run.stdout, &run.stderr, run.completion.stdout)
        } else {
            (&run.stderr, &run.stdout, run.completion.stderr)
        };
        assert_eq!(flooded, &vec![0xa5; 4096]);
        assert!(other.is_empty());
        assert_eq!(completion, StreamCompletion::Truncated);
    }
}

#[test]
fn finite_sleep_times_out_and_reaps_the_direct_child() {
    let root = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    let base = command("sleep", &environment);
    let command = CheckCommand::new(
        base.program().to_owned(),
        base.args().to_vec(),
        base.cwd().to_owned(),
        base.executable_sha256().to_owned(),
        base.environment_sha256().to_owned(),
        100,
        1000,
        4096,
        4096,
    )
    .unwrap();
    let start = Instant::now();
    let run = run_trusted_fixture(
        &command,
        &executable(),
        root.path(),
        &environment,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_reaped(&run);
    assert_eq!(run.completion.reason, StopReason::TimedOut);
    assert!(run.elapsed_ms >= command.timeout_ms());
    // Scheduling tolerance is separate from the runner's configured budgets.
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(run.stdout.is_empty());
    assert!(run.stderr.is_empty());
}

#[test]
fn cancellation_after_fixture_readiness_reaps_the_direct_child() {
    let root = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    let command = command("sleep", &environment);
    let cancelled = AtomicBool::new(false);
    let ready = root.path().join("ready");
    let start = Instant::now();
    let (run, saw_ready) = std::thread::scope(|scope| {
        let trigger = scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !ready.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
            let saw_ready = ready.exists();
            cancelled.store(true, Ordering::SeqCst);
            saw_ready
        });
        let run = run_trusted_fixture(
            &command,
            &executable(),
            root.path(),
            &environment,
            &cancelled,
        );
        (run.unwrap(), trigger.join().unwrap())
    });
    assert_reaped(&run);
    assert!(saw_ready, "cancellation must exercise a spawned fixture");
    assert_eq!(run.completion.reason, StopReason::Cancelled);
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(run.stdout.is_empty());
    assert!(run.stderr.is_empty());
}

#[test]
fn preflight_rejects_digest_environment_and_exact_program_mismatches() {
    let root = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    let base = command("sleep", &environment);
    for mismatch in ["digest", "environment", "program", "absolute-program"] {
        let command = CheckCommand::new(
            if mismatch == "program" {
                "graph-execution-fixture".to_owned()
            } else if mismatch == "absolute-program" {
                // Same file, different absolute spelling: equality must be exact.
                format!("/./{}", base.program().trim_start_matches('/'))
            } else {
                base.program().to_owned()
            },
            base.args().to_vec(),
            base.cwd().to_owned(),
            if mismatch == "digest" {
                let mut digest = base.executable_sha256().to_owned();
                digest.replace_range(..1, if digest.starts_with('0') { "1" } else { "0" });
                digest
            } else {
                base.executable_sha256().to_owned()
            },
            base.environment_sha256().to_owned(),
            5000,
            1000,
            4096,
            4096,
        )
        .unwrap();
        let supplied_environment = if mismatch == "environment" {
            BTreeMap::from([("GRAPH_FIXTURE_VALUE".to_owned(), "changed".to_owned())])
        } else {
            environment.clone()
        };
        let result = run_trusted_fixture(
            &command,
            &executable(),
            root.path(),
            &supplied_environment,
            &AtomicBool::new(false),
        );
        assert!(
            matches!(result, Err(FixtureError::Preflight)),
            "expected preflight rejection for {mismatch}, got {result:?}"
        );
        assert!(
            !root.path().join("ready").exists(),
            "preflight spawned the fixture"
        );
    }
}
