#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_application::environment_fingerprint;
use graph_domain::{
    CheckCommand,
    execution::{ChildCompletion, ScopeCleanup, StopReason, StreamCompletion},
};
use graph_execution::run_trusted_fixture;
use rustix::process::{Pid, WaitOptions};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

#[test]
#[ignore = "isolated subreaper helper; invoked by descendant_pipe_deadline_is_bounded"]
fn descendant_subreaper_helper() {
    assert_eq!(
        std::env::var("GRAPH_RUN_SUBREAPER_HELPER").as_deref(),
        Ok("1")
    );
    rustix::process::set_child_subreaper(Some(Pid::INIT)).unwrap();
    let root = tempfile::tempdir().unwrap();
    let exe = Path::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .canonicalize()
        .unwrap();
    let environment = BTreeMap::from([("GRAPH_FIXTURE_SUBREAPER".into(), "1".into())]);
    let command = CheckCommand::new(
        exe.to_str().unwrap().into(),
        vec!["spawn-holder".into()],
        ".".into(),
        format!("{:x}", Sha256::digest(std::fs::read(&exe).unwrap())),
        environment_fingerprint(&environment).unwrap(),
        5000,
        100,
        4096,
        4096,
    )
    .unwrap();
    let result = run_trusted_fixture(
        &command,
        &exe,
        root.path(),
        &environment,
        &AtomicBool::new(false),
    );
    // Capture observations before waiting; assert only after all adopted children reap.
    let pid_text = std::fs::read_to_string(root.path().join("holder-pid"));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut reaped = Vec::new();
    loop {
        match rustix::process::wait(WaitOptions::NOHANG) {
            Ok(Some((pid, status))) => reaped.push((pid, status.exit_status())),
            Err(rustix::io::Errno::CHILD) => break,
            Err(rustix::io::Errno::INTR) => continue,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(2)),
            other => panic!("owned descendant reap failed: {other:?}"),
        }
    }
    let run = result.unwrap();
    assert!(run.unreaped_child.is_none());
    assert_eq!(
        run.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(run.completion.reason, StopReason::Exited);
    assert_eq!(run.completion.stdout, StreamCompletion::Complete);
    assert_eq!(run.completion.stderr, StreamCompletion::Complete);
    assert_eq!(run.completion.cleanup, ScopeCleanup::Unverifiable);
    assert!(run.stdout.is_empty() && run.stderr.is_empty());
    // The holder inherits the owned process group and keeps both pipes open.
    // Group teardown must stop it before the one-second fixture sleep ends.
    assert!(
        run.elapsed_ms < 900,
        "descendant was not torn down: {}ms",
        run.elapsed_ms
    );
    assert_eq!(reaped.len(), 1);
    assert_eq!(
        reaped[0].0.as_raw_nonzero().get(),
        pid_text.unwrap().parse::<i32>().unwrap()
    );
    assert_eq!(
        reaped[0].1, None,
        "descendant should be terminated by scope"
    );
}

#[test]
fn descendant_pipe_deadline_is_bounded() {
    let mut helper = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "descendant_subreaper_helper",
            "--nocapture",
        ])
        .env_clear()
        .env("GRAPH_RUN_SUBREAPER_HELPER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .process_group(0)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = helper.try_wait().unwrap() {
            assert!(status.success(), "subreaper helper failed: {status}");
            break;
        }
        if Instant::now() >= deadline {
            // PID cannot be recycled while this owned group leader remains unreaped.
            let pid = Pid::from_raw(helper.id().try_into().unwrap()).unwrap();
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            let _ = helper.kill();
            let _ = helper.wait();
            panic!("owned subreaper helper exceeded deadline");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
