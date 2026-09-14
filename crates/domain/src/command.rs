//! Noninteractive check command description, not approval to execute.
use crate::DomainError;

/// Stdin is closed. Host must explicitly construct environment and verify both
/// digests; program is resolved by host policy, never implicit PATH fallback.
#[derive(Clone, PartialEq, Eq)]
pub struct CheckCommand {
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
impl std::fmt::Debug for CheckCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckCommand")
            .field("argument_count", &self.args.len())
            .finish_non_exhaustive()
    }
}
impl CheckCommand {
    /// Constructs a command descriptor without resolving paths or checking bytes.
    ///
    /// # Errors
    /// Returns an error for blank program text, NUL in program/arguments, an
    /// unnormalized relative cwd, or digests other than lowercase SHA-256 hex.
    /// Timeouts must be positive with a combined value at most `i64::MAX`;
    /// each output limit must also fit that range. Zero output limits are valid.
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

/// Shared syntax checks only; no stdin semantics or authorization.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_descriptor(
    program: &str,
    args: &[String],
    cwd: &str,
    executable_sha256: &str,
    environment_sha256: &str,
    timeout_ms: u64,
    cleanup_timeout_ms: u64,
    stdout_max_bytes: u64,
    stderr_max_bytes: u64,
) -> Result<(), DomainError> {
    if program.trim().is_empty()
        || program.contains('\0')
        || args.iter().any(|arg| arg.contains('\0'))
    {
        return Err(DomainError::Invalid(
            "program/argv contain invalid command text",
        ));
    }
    // Wire paths use portable slash separators. Actual root/symlink authority
    // cannot be established here in the filesystem-free domain.
    if cwd != "."
        && (cwd.is_empty()
            || cwd.contains(['\\', ':', '\0'])
            || cwd
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".."))
    {
        return Err(DomainError::Invalid(
            "check cwd must be a normalized relative path",
        ));
    }
    for digest in [executable_sha256, environment_sha256] {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(DomainError::Invalid(
                "command digests must be lowercase SHA-256 hex",
            ));
        }
    }
    if timeout_ms == 0
        || cleanup_timeout_ms == 0
        || timeout_ms.checked_add(cleanup_timeout_ms).is_none()
        || timeout_ms + cleanup_timeout_ms > i64::MAX as u64
        || stdout_max_bytes > i64::MAX as u64
        || stderr_max_bytes > i64::MAX as u64
    {
        return Err(DomainError::Invalid("invalid check time/output limits"));
    }
    Ok(())
}
