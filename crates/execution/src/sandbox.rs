//! Bounded Linux sandbox admission and launch planning.
//!
//! This module owns only the execution-boundary adapter. It does not write the
//! graph, issue a lease, or authenticate an execution receipt. Bubblewrap is a
//! useful namespace backend, but its process/group lifecycle is still reported
//! separately by the supervisor and is not a proof against namespace escape,
//! host crash, or PID reuse.

use graph_domain::CheckCommand;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MOUNT_ROOT: &str = "/mnt";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SandboxEgressPolicy {
    /// A private network namespace is sufficient for this bounded adapter.
    DenyAll,
    /// Requires a network firewall/DNS gateway which bwrap alone does not own.
    AllowList(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SandboxEgressDisposition {
    PrivateNetworkRequested,
    /// Kept as a typed state for callers that want to surface a degraded mode.
    UnverifiableAllowList,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxCapabilities {
    pub backend: String,
    pub version: String,
    pub private_network: bool,
    pub declared_root_bind: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxRuntime {
    executable: PathBuf,
    capabilities: SandboxCapabilities,
}

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("sandbox backend is unsupported or unavailable")]
    Unsupported,
    #[error("sandbox egress policy is unverifiable by the selected backend")]
    UnverifiableEgress,
    #[error("sandbox request was rejected before spawn")]
    Rejected,
    #[error("sandbox backend probe failed")]
    ProbeFailed,
    #[error("sandbox path preflight failed")]
    PathPreflight,
    #[error("sandbox I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxPlan {
    backend: PathBuf,
    args: Vec<OsString>,
    egress: SandboxEgressDisposition,
    capabilities: SandboxCapabilities,
}

impl SandboxRuntime {
    /// Discover a pinned absolute bwrap path, then probe namespace creation.
    /// A mutable PATH is not accepted as an execution identity.
    pub fn discover() -> Result<Self, SandboxError> {
        let candidates = [
            PathBuf::from("/usr/bin/bwrap"),
            PathBuf::from("/usr/local/bin/bwrap"),
            PathBuf::from("/opt/opensandbox/bwrap"),
        ];
        candidates
            .iter()
            .find(|path| path.is_file())
            .map_or(Err(SandboxError::Unsupported), |path| Self::from_path(path))
    }

    /// Construct a runtime only after probing the exact backend binary.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SandboxError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = path;
            Err(SandboxError::Unsupported)
        }
        #[cfg(target_os = "linux")]
        {
            let path = path
                .as_ref()
                .canonicalize()
                .map_err(|_| SandboxError::Unsupported)?;
            if !path.is_absolute() || !path.is_file() {
                return Err(SandboxError::Unsupported);
            }
            let version = probe_version(&path)?;
            probe_namespace(&path)?;
            Ok(Self {
                executable: path.to_path_buf(),
                capabilities: SandboxCapabilities {
                    backend: "bubblewrap".into(),
                    version,
                    private_network: true,
                    declared_root_bind: true,
                },
            })
        }
    }

    #[must_use]
    pub fn capabilities(&self) -> &SandboxCapabilities {
        &self.capabilities
    }

    /// Validate the target and build an argv vector without starting a process.
    /// The target executable must live under the declared root so the child can
    /// execute the same file through the `/mnt` bind inside the namespace.
    pub fn plan(
        &self,
        command: &CheckCommand,
        executable: &Path,
        root: &Path,
        environment: &BTreeMap<String, String>,
        egress: &SandboxEgressPolicy,
    ) -> Result<SandboxPlan, SandboxError> {
        if !matches!(egress, SandboxEgressPolicy::DenyAll) {
            return Err(SandboxError::UnverifiableEgress);
        }
        let root = root
            .canonicalize()
            .map_err(|_| SandboxError::PathPreflight)?;
        let executable = executable
            .canonicalize()
            .map_err(|_| SandboxError::PathPreflight)?;
        let cwd = root
            .join(command.cwd())
            .canonicalize()
            .map_err(|_| SandboxError::PathPreflight)?;
        if !root.is_dir()
            || !cwd.is_dir()
            || !cwd.starts_with(&root)
            || !executable.is_file()
            || !executable.starts_with(&root)
            || executable.to_str() != Some(command.program())
        {
            return Err(SandboxError::Rejected);
        }
        if environment.iter().any(|(key, value)| {
            key.is_empty() || key.contains('=') || key.contains('\0') || value.contains('\0')
        }) {
            return Err(SandboxError::Rejected);
        }
        let executable_rel = executable
            .strip_prefix(&root)
            .map_err(|_| SandboxError::PathPreflight)?;
        let cwd_rel = cwd
            .strip_prefix(&root)
            .map_err(|_| SandboxError::PathPreflight)?;
        let sandbox_executable = sandbox_path(executable_rel)?;
        let sandbox_cwd = sandbox_path(cwd_rel)?;

        let mut args = vec![
            "--unshare-pid".into(),
            "--unshare-uts".into(),
            "--hostname".into(),
            "graph-sandbox".into(),
            "--unshare-ipc".into(),
            "--unshare-cgroup".into(),
            "--unshare-net".into(),
            "--ro-bind".into(),
            "/".into(),
            "/".into(),
            "--tmpfs".into(),
            "/tmp".into(),
            "--tmpfs".into(),
            "/run".into(),
            "--dev".into(),
            "/dev".into(),
            "--proc".into(),
            "/proc".into(),
            // The product workspace is supplied explicitly at /mnt. Mask the
            // common host-owned roots in this host-level fixture adapter.
            "--tmpfs".into(),
            "/home".into(),
            "--tmpfs".into(),
            "/root".into(),
            "--bind".into(),
            root.as_os_str().to_owned(),
            MOUNT_ROOT.into(),
            "--chdir".into(),
            sandbox_cwd,
            "--clearenv".into(),
        ];
        for (key, value) in environment {
            args.extend([
                OsString::from("--setenv"),
                OsString::from(key),
                OsString::from(value),
            ]);
        }
        args.extend(["--die-with-parent".into(), "--".into(), sandbox_executable]);
        args.extend(command.args().iter().map(OsString::from));

        Ok(SandboxPlan {
            backend: self.executable.clone(),
            args,
            egress: SandboxEgressDisposition::PrivateNetworkRequested,
            capabilities: self.capabilities.clone(),
        })
    }
}

impl SandboxPlan {
    #[must_use]
    pub fn backend(&self) -> &Path {
        &self.backend
    }

    #[must_use]
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    #[must_use]
    pub fn egress(&self) -> &SandboxEgressDisposition {
        &self.egress
    }

    #[must_use]
    pub fn capabilities(&self) -> &SandboxCapabilities {
        &self.capabilities
    }

    /// Apply this immutable plan to a command. The caller still owns pipes,
    /// process-group supervision, cancellation and reaping.
    pub fn configure(&self, command: &mut Command) {
        command
            .arg(self.backend.as_os_str())
            .args(&self.args)
            .stdin(Stdio::null());
    }
}

fn sandbox_path(relative: &Path) -> Result<OsString, SandboxError> {
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(SandboxError::PathPreflight);
    }
    let mut path = PathBuf::from(MOUNT_ROOT);
    if !relative.as_os_str().is_empty() {
        path.push(relative);
    }
    Ok(path.into_os_string())
}

fn probe_version(path: &Path) -> Result<String, SandboxError> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|_| SandboxError::ProbeFailed)?;
    if !output.status.success() {
        return Err(SandboxError::ProbeFailed);
    }
    let version = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find(|token| token.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or(SandboxError::ProbeFailed)?;
    Ok(version)
}

fn probe_namespace(path: &Path) -> Result<(), SandboxError> {
    let status = Command::new(path)
        .args([
            "--unshare-pid",
            "--unshare-uts",
            "--unshare-ipc",
            "--unshare-cgroup",
            "--unshare-net",
            "--ro-bind",
            "/",
            "/",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--die-with-parent",
            "--",
            "/usr/bin/true",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|_| SandboxError::ProbeFailed)?;
    if status.success() {
        Ok(())
    } else {
        Err(SandboxError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_path_rejects_parent_escape() {
        assert!(sandbox_path(Path::new("../escape")).is_err());
        assert_eq!(
            sandbox_path(Path::new("nested/cwd")).unwrap(),
            "/mnt/nested/cwd"
        );
    }
}
