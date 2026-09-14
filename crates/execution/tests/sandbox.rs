#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use graph_application::environment_fingerprint;
use graph_domain::{CheckCommand, execution::ChildCompletion};
use graph_execution::{
    SandboxEgressDisposition, SandboxEgressPolicy, SandboxError, run_trusted_fixture_in_sandbox,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn fixture_source() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .canonicalize()
        .unwrap()
}

fn copy_fixture(root: &Path) -> PathBuf {
    let target = root.join("fixture-bin");
    fs::copy(fixture_source(), &target).unwrap();
    let mut permissions = fs::metadata(&target).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o755);
        fs::set_permissions(&target, permissions).unwrap();
    }
    target.canonicalize().unwrap()
}

fn command(
    executable: &Path,
    args: Vec<String>,
    environment: &BTreeMap<String, String>,
) -> CheckCommand {
    CheckCommand::new(
        executable.to_str().unwrap().into(),
        args,
        ".".into(),
        format!("{:x}", Sha256::digest(fs::read(executable).unwrap())),
        environment_fingerprint(environment).unwrap(),
        5000,
        1000,
        4096,
        4096,
    )
    .unwrap()
}

#[path = "support/sandbox_requirement.rs"]
mod sandbox_requirement;
use sandbox_requirement::runtime_or_skip;

#[test]
fn sandbox_plan_is_private_and_allow_list_fails_closed() {
    let Some(runtime) = runtime_or_skip() else {
        return;
    };
    let root = tempfile::tempdir().unwrap();
    let executable = copy_fixture(root.path());
    let environment = BTreeMap::from([("GRAPH_FIXTURE_VALUE".into(), "bounded".into())]);
    let command = command(&executable, vec!["visibility".into()], &environment);

    let plan = runtime
        .plan(
            &command,
            &executable,
            root.path(),
            &environment,
            &SandboxEgressPolicy::DenyAll,
        )
        .unwrap();
    assert!(plan.args().iter().any(|arg| arg == "--unshare-net"));
    assert!(plan.args().iter().any(|arg| arg == "--clearenv"));
    assert_eq!(
        plan.egress(),
        &SandboxEgressDisposition::PrivateNetworkRequested
    );

    let allow_list = SandboxEgressPolicy::AllowList(vec!["example.com".into()]);
    assert!(matches!(
        runtime.plan(
            &command,
            &executable,
            root.path(),
            &environment,
            &allow_list
        ),
        Err(SandboxError::UnverifiableEgress)
    ));
}

#[test]
fn sandbox_fixture_sees_declared_mount_and_not_masked_host_temp() {
    let Some(runtime) = runtime_or_skip() else {
        return;
    };
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let executable = copy_fixture(root.path());
    fs::write(root.path().join("root-sentinel"), b"inside").unwrap();
    fs::write(outside.path().join("host-sentinel"), b"outside").unwrap();
    let environment = BTreeMap::new();
    let command = command(
        &executable,
        vec![
            "visibility".into(),
            "/mnt/root-sentinel".into(),
            outside
                .path()
                .join("host-sentinel")
                .to_str()
                .unwrap()
                .into(),
        ],
        &environment,
    );

    let run = run_trusted_fixture_in_sandbox(
        &command,
        &executable,
        root.path(),
        &environment,
        &SandboxEgressPolicy::DenyAll,
        &runtime,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        run.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        format!(
            "/mnt/root-sentinel=visible\n{}=hidden\n",
            outside.path().join("host-sentinel").display()
        )
    );
    assert!(run.unreaped_child.is_none());
}

#[test]
fn sandbox_rejects_executable_outside_declared_root_before_spawn() {
    let Some(runtime) = runtime_or_skip() else {
        return;
    };
    let root = tempfile::tempdir().unwrap();
    let outside = fixture_source();
    let environment = BTreeMap::new();
    let command = command(&outside, vec!["sleep".into()], &environment);

    assert!(matches!(
        run_trusted_fixture_in_sandbox(
            &command,
            &outside,
            root.path(),
            &environment,
            &SandboxEgressPolicy::DenyAll,
            &runtime,
            &AtomicBool::new(false),
        ),
        Err(graph_execution::FixtureError::Sandbox(
            SandboxError::Rejected
        ))
    ));
}
