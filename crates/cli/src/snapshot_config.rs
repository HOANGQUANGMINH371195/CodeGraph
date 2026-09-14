use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{Map, Value};

const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COMMAND_TIMEOUT_MS: u64 = 60_000;

pub(crate) struct SnapshotAuthorityConfig {
    pub(crate) root: PathBuf,
    pub(crate) repository_id: String,
    pub(crate) worktree_id: String,
    pub(crate) config_hash: String,
    pub(crate) ignore_policy_version: String,
    pub(crate) max_total_bytes: u64,
    pub(crate) command_timeout: Duration,
}

pub(crate) fn load(path: &Path) -> Result<SnapshotAuthorityConfig, Box<dyn Error>> {
    let value: Value = serde_json::from_slice(&crate::input::read_json_bytes(path)?)?;
    let object = value.as_object().ok_or_else(invalid_config)?;
    let expected = [
        "schema_version",
        "root",
        "repository_id",
        "worktree_id",
        "config_hash",
        "ignore_policy_version",
        "max_total_bytes",
        "command_timeout_ms",
    ];
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid_config().into());
    }

    let schema_version = number(object, "schema_version")?;
    if schema_version != 1 {
        return Err(invalid_config().into());
    }
    let root = text(object, "root")?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err(invalid_config().into());
    }
    let max_total_bytes = number(object, "max_total_bytes")?;
    if max_total_bytes == 0 || max_total_bytes > MAX_SOURCE_BYTES {
        return Err(invalid_config().into());
    }
    let command_timeout_ms = number(object, "command_timeout_ms")?;
    if command_timeout_ms == 0 || command_timeout_ms > MAX_COMMAND_TIMEOUT_MS {
        return Err(invalid_config().into());
    }

    Ok(SnapshotAuthorityConfig {
        root,
        repository_id: text(object, "repository_id")?,
        worktree_id: text(object, "worktree_id")?,
        config_hash: text(object, "config_hash")?,
        ignore_policy_version: text(object, "ignore_policy_version")?,
        max_total_bytes,
        command_timeout: Duration::from_millis(command_timeout_ms),
    })
}

fn text(object: &Map<String, Value>, key: &str) -> Result<String, Box<dyn Error>> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty() && !value.chars().any(char::is_control))
        .ok_or_else(invalid_config)?;
    Ok(value.to_owned())
}

fn number(object: &Map<String, Value>, key: &str) -> Result<u64, Box<dyn Error>> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_config().into())
}

fn invalid_config() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid snapshot authority config",
    )
}
