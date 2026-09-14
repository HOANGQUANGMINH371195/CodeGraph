use crate::{ProtocolError, SCHEMA_VERSION, require_current_schema};
use serde::{Deserialize, Serialize};

/// Untrusted descriptor; no environment values, host approval or verified status.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckCommand {
    pub schema_version: u32,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub executable_sha256: String,
    pub environment_sha256: String,
    pub timeout_ms: u64,
    pub cleanup_timeout_ms: u64,
    pub stdout_max_bytes: u64,
    pub stderr_max_bytes: u64,
}
impl std::fmt::Debug for CheckCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckCommand")
            .field("argument_count", &self.args.len())
            .finish_non_exhaustive()
    }
}
impl CheckCommand {
    pub fn try_into_domain(self) -> Result<graph_domain::CheckCommand, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::CheckCommand::new(
            self.program,
            self.args,
            self.cwd,
            self.executable_sha256,
            self.environment_sha256,
            self.timeout_ms,
            self.cleanup_timeout_ms,
            self.stdout_max_bytes,
            self.stderr_max_bytes,
        )?)
    }
}
impl From<&graph_domain::CheckCommand> for CheckCommand {
    fn from(value: &graph_domain::CheckCommand) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            program: value.program().into(),
            args: value.args().to_vec(),
            cwd: value.cwd().into(),
            executable_sha256: value.executable_sha256().into(),
            environment_sha256: value.environment_sha256().into(),
            timeout_ms: value.timeout_ms(),
            cleanup_timeout_ms: value.cleanup_timeout_ms(),
            stdout_max_bytes: value.stdout_max_bytes(),
            stderr_max_bytes: value.stderr_max_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> CheckCommand {
        CheckCommand {
            schema_version: 1,
            program: "fixture-runner".into(),
            args: vec![
                "".into(),
                "a b".into(),
                "$(not-a-shell-command);*".into(),
                "tiếng Việt".into(),
            ],
            cwd: ".".into(),
            executable_sha256: "a".repeat(64),
            environment_sha256: "b".repeat(64),
            timeout_ms: 1000,
            cleanup_timeout_ms: 500,
            stdout_max_bytes: 1024,
            stderr_max_bytes: 1024,
        }
    }

    #[test]
    fn exact_argv_roundtrip_and_debug_do_not_expose_arguments() {
        let wire = command();
        let domain = wire.clone().try_into_domain().unwrap();
        assert_eq!(domain.args(), wire.args);
        assert_eq!(CheckCommand::from(&domain), wire);
        let json = serde_json::to_vec(&wire).unwrap();
        let decoded: CheckCommand = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.try_into_domain().unwrap(), domain);
        assert!(!format!("{domain:?}").contains("not-a-shell-command"));
        assert!(!format!("{wire:?}").contains("not-a-shell-command"));
    }

    #[test]
    fn invalid_paths_hashes_nul_and_limits_are_rejected() {
        for cwd in [
            "", "/tmp", "../src", "a/../b", "a//b", "a/./b", "a/", "C:/repo", "a\\b", "a\0b",
        ] {
            let mut wire = command();
            wire.cwd = cwd.into();
            assert!(wire.try_into_domain().is_err(), "{cwd:?}");
        }
        for cwd in [".", "src", "packages/a", "thư mục"] {
            let mut wire = command();
            wire.cwd = cwd.into();
            assert!(wire.try_into_domain().is_ok());
        }
        for program in ["", " ", "a\0b"] {
            let mut wire = command();
            wire.program = program.into();
            assert!(wire.try_into_domain().is_err());
        }
        let mut wire = command();
        wire.args.push("bad\0arg".into());
        assert!(wire.try_into_domain().is_err());
        for hash in ["A".repeat(64), "g".repeat(64), "a".repeat(63)] {
            let mut wire = command();
            wire.executable_sha256 = hash.clone();
            assert!(wire.try_into_domain().is_err());
            let mut wire = command();
            wire.environment_sha256 = hash;
            assert!(wire.try_into_domain().is_err());
        }
        for field in 0..5 {
            let mut wire = command();
            match field {
                0 => wire.timeout_ms = 0,
                1 => wire.cleanup_timeout_ms = 0,
                2 => wire.timeout_ms = u64::MAX,
                3 => wire.stdout_max_bytes = u64::MAX,
                _ => wire.stderr_max_bytes = u64::MAX,
            }
            assert!(wire.try_into_domain().is_err());
        }
        let mut wire = command();
        wire.stdout_max_bytes = 0;
        wire.stderr_max_bytes = 0;
        assert!(wire.try_into_domain().is_ok());
    }

    #[test]
    fn wire_requires_explicit_fields_and_supported_schema() {
        let valid = serde_json::to_value(command()).unwrap();
        for key in valid.as_object().unwrap().keys() {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<CheckCommand>(missing).is_err(),
                "{key}"
            );
        }
        let mut extra = valid;
        extra["approved"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CheckCommand>(extra).is_err());
        let mut wire = command();
        wire.schema_version = 2;
        assert!(wire.try_into_domain().is_err());
    }
}
