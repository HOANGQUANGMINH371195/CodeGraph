//! Descriptions for piped JSONL RPC, never executable host authorization.
use crate::{DomainError, Lease, ProjectRef, TaskSpec, command::validate_descriptor};

/// Unlike CheckCommand, stdin carries RPC frames. Environment values remain host-owned.
#[derive(Clone, PartialEq, Eq)]
pub struct RpcProcessSpec {
    program: String,
    args: Vec<String>,
    cwd: String,
    executable_sha256: String,
    environment_sha256: String,
    timeout_ms: u64,
    cleanup_timeout_ms: u64,
    stdout_max_bytes: u64,
    stderr_max_bytes: u64,
}
impl std::fmt::Debug for RpcProcessSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcProcessSpec")
            .field("argument_count", &self.args.len())
            .finish_non_exhaustive()
    }
}
impl RpcProcessSpec {
    /// Constructs a bounded RPC process descriptor; host policy resolves it.
    ///
    /// # Errors
    /// Returns an error when command text, paths, digests, timeouts, or output
    /// limits fail the shared descriptor validation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        program: String,
        args: Vec<String>,
        cwd: String,
        executable_sha256: String,
        environment_sha256: String,
        timeout_ms: u64,
        cleanup_timeout_ms: u64,
        stdout_max_bytes: u64,
        stderr_max_bytes: u64,
    ) -> Result<Self, DomainError> {
        validate_descriptor(
            &program,
            &args,
            &cwd,
            &executable_sha256,
            &environment_sha256,
            timeout_ms,
            cleanup_timeout_ms,
            stdout_max_bytes,
            stderr_max_bytes,
        )?;
        Ok(Self {
            program,
            args,
            cwd,
            executable_sha256,
            environment_sha256,
            timeout_ms,
            cleanup_timeout_ms,
            stdout_max_bytes,
            stderr_max_bytes,
        })
    }
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }
    #[must_use]
    pub fn cwd(&self) -> &str {
        &self.cwd
    }
    #[must_use]
    pub fn executable_sha256(&self) -> &str {
        &self.executable_sha256
    }
    #[must_use]
    pub fn environment_sha256(&self) -> &str {
        &self.environment_sha256
    }
    #[must_use]
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
    #[must_use]
    pub fn cleanup_timeout_ms(&self) -> u64 {
        self.cleanup_timeout_ms
    }
    #[must_use]
    pub fn stdout_max_bytes(&self) -> u64 {
        self.stdout_max_bytes
    }
    #[must_use]
    pub fn stderr_max_bytes(&self) -> u64 {
        self.stderr_max_bytes
    }
}

/// Protocol bounds, not platform capability. Adapter checks actual frame fit and caps.
#[derive(Clone, PartialEq, Eq)]
pub struct RpcConnectionSpec {
    epoch: String,
    client_name: String,
    client_version: String,
    experimental: bool,
    max_pending: u32,
    max_frame: u32,
}
impl std::fmt::Debug for RpcConnectionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcConnectionSpec")
            .field("max_pending", &self.max_pending)
            .field("max_frame", &self.max_frame)
            .finish_non_exhaustive()
    }
}
impl RpcConnectionSpec {
    /// Constructs protocol framing and connection limits.
    ///
    /// # Errors
    /// Returns an error when identity text is invalid or limits exceed the
    /// protocol bounds.
    pub fn new(
        epoch: String,
        client_name: String,
        client_version: String,
        experimental: bool,
        max_pending: u32,
        max_frame: u32,
    ) -> Result<Self, DomainError> {
        for label in [&epoch, &client_name, &client_version] {
            validate_label(label)?;
        }
        if !(1..=4096).contains(&max_pending) || !(1..=16 * 1024 * 1024).contains(&max_frame) {
            return Err(DomainError::Invalid("invalid RPC connection limits"));
        }
        Ok(Self {
            epoch,
            client_name,
            client_version,
            experimental,
            max_pending,
            max_frame,
        })
    }
    pub fn epoch(&self) -> &str {
        &self.epoch
    }
    pub fn client_name(&self) -> &str {
        &self.client_name
    }
    pub fn client_version(&self) -> &str {
        &self.client_version
    }
    pub fn experimental(&self) -> bool {
        self.experimental
    }
    pub fn max_pending(&self) -> u32 {
        self.max_pending
    }
    pub fn max_frame(&self) -> u32 {
        self.max_frame
    }
}

/// Immutable requested launch. Every identity/approval/snapshot is a claim.
/// The host must resolve current authority and consume a durable one-shot claim.
/// Task/lease identify the launch's origin, not one process per RPC request.
/// Reusing a connection for other work still needs separate per-request authority.
#[derive(Clone, PartialEq, Eq)]
pub struct RpcLaunchSpec {
    id: String,
    task: TaskSpec,
    origin_lease: Lease,
    host_id: String,
    approval_id: String,
    execution_snapshot: ProjectRef,
    process: RpcProcessSpec,
    connection: RpcConnectionSpec,
}
impl std::fmt::Debug for RpcLaunchSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcLaunchSpec").finish_non_exhaustive()
    }
}
impl RpcLaunchSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        task: TaskSpec,
        origin_lease: Lease,
        host_id: String,
        approval_id: String,
        execution_snapshot: ProjectRef,
        process: RpcProcessSpec,
        connection: RpcConnectionSpec,
    ) -> Result<Self, DomainError> {
        for label in [&id, &host_id, &approval_id] {
            validate_label(label)?;
        }
        task.validate()?;
        execution_snapshot.validate()?;
        if origin_lease.task_id() != task.id() || origin_lease.expires_at_ms() <= 0 {
            return Err(DomainError::Invalid(
                "RPC launch lease does not bind the task",
            ));
        }
        let source = task.project();
        if execution_snapshot.repository_id != source.repository_id
            || execution_snapshot.config_hash != source.config_hash
            || execution_snapshot.ignore_policy_version != source.ignore_policy_version
        {
            return Err(DomainError::Invalid(
                "RPC execution snapshot differs from task policy",
            ));
        }
        Ok(Self {
            id,
            task,
            origin_lease,
            host_id,
            approval_id,
            execution_snapshot,
            process,
            connection,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn task(&self) -> &TaskSpec {
        &self.task
    }
    pub fn origin_lease(&self) -> &Lease {
        &self.origin_lease
    }
    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    pub fn approval_id(&self) -> &str {
        &self.approval_id
    }
    pub fn execution_snapshot(&self) -> &ProjectRef {
        &self.execution_snapshot
    }
    pub fn process(&self) -> &RpcProcessSpec {
        &self.process
    }
    pub fn connection(&self) -> &RpcConnectionSpec {
        &self.connection
    }
}

fn validate_label(value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        Err(DomainError::Invalid("invalid RPC launch label"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckCommand, TaskId, WorkerId};

    fn process() -> RpcProcessSpec {
        RpcProcessSpec::new(
            "/owned/fixture".into(),
            vec![
                "".into(),
                "a b".into(),
                "$(literal);*".into(),
                "tiếng Việt".into(),
            ],
            ".".into(),
            "a".repeat(64),
            "b".repeat(64),
            1000,
            500,
            4096,
            4096,
        )
        .unwrap()
    }
    fn rebuild_process(p: RpcProcessSpec) -> Result<RpcProcessSpec, DomainError> {
        RpcProcessSpec::new(
            p.program,
            p.args,
            p.cwd,
            p.executable_sha256,
            p.environment_sha256,
            p.timeout_ms,
            p.cleanup_timeout_ms,
            p.stdout_max_bytes,
            p.stderr_max_bytes,
        )
    }
    fn connection() -> RpcConnectionSpec {
        RpcConnectionSpec::new("epoch".into(), "client".into(), "1".into(), false, 2, 4096).unwrap()
    }
    fn launch() -> RpcLaunchSpec {
        let snapshot = ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "abc".into(),
            working_tree_fingerprint: "dirty".into(),
            config_hash: "cfg".into(),
            ignore_policy_version: "1".into(),
        };
        let task = TaskSpec::new(
            TaskId::new("task").unwrap(),
            snapshot.clone(),
            "graph".into(),
            "reader".into(),
            "native".into(),
            vec!["src".into()],
            vec![],
            "context".into(),
            vec!["report".into()],
            100,
        )
        .unwrap();
        let lease =
            Lease::issue(task.id().clone(), WorkerId::new("worker").unwrap(), 1, 1000).unwrap();
        RpcLaunchSpec::new(
            "launch".into(),
            task,
            lease,
            "host".into(),
            "approval-reference".into(),
            snapshot,
            process(),
            connection(),
        )
        .unwrap()
    }
    fn rebuild_launch(s: RpcLaunchSpec) -> Result<RpcLaunchSpec, DomainError> {
        RpcLaunchSpec::new(
            s.id,
            s.task,
            s.origin_lease,
            s.host_id,
            s.approval_id,
            s.execution_snapshot,
            s.process,
            s.connection,
        )
    }

    #[test]
    fn process_validation_is_shared_without_changing_closed_stdin_checks() {
        let p = process();
        let check = CheckCommand::new(
            p.program.clone(),
            p.args.clone(),
            p.cwd.clone(),
            p.executable_sha256.clone(),
            p.environment_sha256.clone(),
            p.timeout_ms,
            p.cleanup_timeout_ms,
            p.stdout_max_bytes,
            p.stderr_max_bytes,
        )
        .unwrap();
        assert_eq!(p.args(), check.args());
        assert!(!format!("{p:?}").contains("literal"));
        for case in 0..11 {
            let mut p = process();
            match case {
                0 => p.program = " ".into(),
                1 => p.args.push("bad\0argument".into()),
                2 => p.cwd = "../outside".into(),
                3 => p.executable_sha256 = "A".repeat(64),
                4 => p.environment_sha256 = "a".repeat(63),
                5 => p.timeout_ms = 0,
                6 => p.cleanup_timeout_ms = 0,
                7 => p.timeout_ms = u64::MAX,
                8 => p.stdout_max_bytes = u64::MAX,
                9 => p.stderr_max_bytes = u64::MAX,
                _ => {
                    p.timeout_ms = i64::MAX as u64;
                    p.cleanup_timeout_ms = 1;
                }
            }
            assert!(rebuild_process(p).is_err(), "case {case}");
        }
        let mut p = process();
        p.stdout_max_bytes = 0;
        p.stderr_max_bytes = 0;
        assert!(rebuild_process(p).is_ok());
    }

    #[test]
    fn connection_rejects_bad_labels_and_limits_but_not_adapter_specific_caps() {
        for bad in [
            "".into(),
            " ".into(),
            "a".repeat(257),
            "a\0b".into(),
            "a\nb".into(),
        ] {
            for position in 0..3 {
                let mut labels = ["epoch".to_owned(), "client".to_owned(), "1".to_owned()];
                labels[position] = bad.clone();
                let [epoch, name, version] = labels;
                assert!(RpcConnectionSpec::new(epoch, name, version, false, 2, 4096).is_err());
            }
        }
        for (pending, frame) in [(0, 4096), (4097, 4096), (2, 0), (2, 16 * 1024 * 1024 + 1)] {
            assert!(
                RpcConnectionSpec::new("e".into(), "c".into(), "1".into(), false, pending, frame)
                    .is_err()
            );
        }
        let spec = RpcConnectionSpec::new(
            "e".repeat(256),
            "c".into(),
            "1".into(),
            true,
            4096,
            16 * 1024 * 1024,
        )
        .unwrap();
        assert!(spec.experimental());
        assert_eq!(spec.max_frame(), 16 * 1024 * 1024);
        assert_eq!(spec.max_pending(), 4096);
    }

    #[test]
    fn launch_rejects_mismatched_task_lease_and_repository_policy() {
        for case in 0..9 {
            let mut s = launch();
            match case {
                0 => s.id.clear(),
                1 => s.host_id = "host\n".into(),
                2 => s.approval_id.clear(),
                3 => {
                    s.origin_lease = Lease::issue(
                        TaskId::new("other-task").unwrap(),
                        WorkerId::new("worker").unwrap(),
                        1,
                        1000,
                    )
                    .unwrap()
                }
                4 => {
                    s.origin_lease =
                        Lease::issue(s.task.id().clone(), WorkerId::new("worker").unwrap(), 1, 0)
                            .unwrap()
                }
                5 => s.execution_snapshot.repository_id = "other".into(),
                6 => s.execution_snapshot.config_hash = "other".into(),
                7 => s.execution_snapshot.ignore_policy_version = "other".into(),
                _ => s.execution_snapshot.worktree_id.clear(),
            }
            assert!(rebuild_launch(s).is_err(), "case {case}");
        }
    }

    #[test]
    fn complete_description_identity_is_preserved_without_minting_authority() {
        let original = launch();
        assert_eq!(rebuild_launch(original.clone()).unwrap(), original);
        for case in 0..9 {
            let mut s = original.clone();
            match case {
                0 => s.id = "other-launch".into(),
                1 => s.host_id = "other-host".into(),
                2 => s.approval_id = "unverified-other-approval".into(),
                3 => s.execution_snapshot.worktree_id = "isolated".into(),
                4 => s.execution_snapshot.git_head = "different".into(),
                5 => s.connection.epoch = "other-epoch".into(),
                6 => s.connection.max_frame = 8192,
                7 => s.process.args.push("more".into()),
                _ => s.process.environment_sha256 = "c".repeat(64),
            }
            assert_ne!(rebuild_launch(s).unwrap(), original, "case {case}");
        }
        let debug = format!("{original:?}");
        for private in ["literal", "approval-reference", "worker", "dirty"] {
            assert!(!debug.contains(private));
        }
    }
}
