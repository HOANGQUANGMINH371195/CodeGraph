//! Host-owned Git/worktree snapshot authority.
//!
//! This adapter identifies the exact worktree selected by the host and hashes
//! the complete Git-visible source set. It is deliberately separate from
//! `DirectorySource`: access to a directory and proof that it is the requested
//! snapshot are different capabilities.

use std::{
    collections::BTreeSet,
    fs::{self, File, Metadata, Permissions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use graph_application::{SourceSnapshotAuthority, SourceSnapshotBinding};
use graph_domain::ProjectRef;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

const FINGERPRINT_VERSION: &[u8] = b"project-graph/git-snapshot/v1\0";
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_GIT_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
const TEMP_DIR_CREATE_ATTEMPTS: u8 = 64;

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Owns a private temporary directory without making the runtime depend on a
/// test-oriented helper crate. The directory is removed when the last source
/// capability owning it is dropped.
struct OwnedTempDir {
    path: PathBuf,
}

impl OwnedTempDir {
    fn new() -> io::Result<Self> {
        let base = std::env::temp_dir();
        let process_id = std::process::id();
        for _ in 0..TEMP_DIR_CREATE_ATTEMPTS {
            let sequence = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("project-graph-source-{process_id}-{sequence}"));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            builder.mode(0o700);
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique temporary source directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        thaw_tree(&self.path);
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug, Error)]
pub enum GitSnapshotAuthorityError {
    #[error("snapshot root is unavailable")]
    RootUnavailable(#[source] io::Error),
    #[error("requested root is not the authority's configured worktree")]
    RootMismatch,
    #[error("Git command could not be started")]
    GitStart(#[source] io::Error),
    #[error("Git command failed")]
    GitFailed,
    #[error("Git command exceeded its time limit")]
    GitTimeout,
    #[error("snapshot observation exceeded its time limit")]
    SnapshotTimeout,
    #[error("Git output exceeded the authority limit")]
    GitOutputTooLarge,
    #[error("Git returned invalid text or path data")]
    InvalidGitOutput,
    #[error("Git returned a duplicate source path")]
    DuplicatePath,
    #[error("source entry is not a regular file or symlink")]
    UnsupportedEntry,
    #[error("source bytes exceeded the authority limit")]
    SourceTooLarge,
    #[error("source filesystem access failed")]
    SourceIo(#[source] io::Error),
    #[error("snapshot metadata is invalid: {0}")]
    InvalidMetadata(&'static str),
    #[error("requested project does not match the observed Git snapshot")]
    ProjectMismatch,
    #[error("source snapshot changed while it was being observed")]
    SnapshotChanged,
}

/// A host-configured authority for one canonical Git worktree.
///
/// Repository/worktree/config/ignore identifiers are supplied by the host and
/// are compared exactly. Git supplies `HEAD` and the working-tree fingerprint;
/// no identity field is silently invented from a caller-provided path.
pub struct GitSnapshotAuthority {
    root: PathBuf,
    repository_id: String,
    worktree_id: String,
    config_hash: String,
    ignore_policy_version: String,
    max_total_bytes: u64,
    command_timeout: Duration,
}

impl GitSnapshotAuthority {
    /// Construct an authority with the default five-second observation limit.
    pub fn new(
        root: impl AsRef<Path>,
        repository_id: String,
        worktree_id: String,
        config_hash: String,
        ignore_policy_version: String,
        max_total_bytes: u64,
    ) -> Result<Self, GitSnapshotAuthorityError> {
        Self::with_command_timeout(
            root,
            repository_id,
            worktree_id,
            config_hash,
            ignore_policy_version,
            max_total_bytes,
            DEFAULT_COMMAND_TIMEOUT,
        )
    }

    /// Variant used by a host that needs a shorter bounded observation.
    pub fn with_command_timeout(
        root: impl AsRef<Path>,
        repository_id: String,
        worktree_id: String,
        config_hash: String,
        ignore_policy_version: String,
        max_total_bytes: u64,
        command_timeout: Duration,
    ) -> Result<Self, GitSnapshotAuthorityError> {
        if max_total_bytes == 0
            || max_total_bytes > MAX_SOURCE_BYTES
            || command_timeout.is_zero()
            || command_timeout > MAX_COMMAND_TIMEOUT
        {
            return Err(GitSnapshotAuthorityError::InvalidMetadata(
                "snapshot limits are outside the supported bounds",
            ));
        }
        for value in [
            &repository_id,
            &worktree_id,
            &config_hash,
            &ignore_policy_version,
        ] {
            if value.trim().is_empty() || value.chars().any(char::is_control) {
                return Err(GitSnapshotAuthorityError::InvalidMetadata(
                    "host snapshot identity contains invalid text",
                ));
            }
        }
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(GitSnapshotAuthorityError::RootUnavailable)?;
        if !root.is_dir() {
            return Err(GitSnapshotAuthorityError::InvalidMetadata(
                "snapshot root must be a directory",
            ));
        }
        Ok(Self {
            root,
            repository_id,
            worktree_id,
            config_hash,
            ignore_policy_version,
            max_total_bytes,
            command_timeout,
        })
    }

    /// Observe the current project identity without creating a graph fact.
    pub fn current_project(&self) -> Result<ProjectRef, GitSnapshotAuthorityError> {
        let observed = self.observe_stable()?;
        Ok(self.project_from_observation(&observed))
    }

    fn project_from_observation(&self, observed: &Observation) -> ProjectRef {
        ProjectRef {
            repository_id: self.repository_id.clone(),
            worktree_id: self.worktree_id.clone(),
            git_head: observed.head.clone(),
            working_tree_fingerprint: observed.fingerprint.clone(),
            config_hash: self.config_hash.clone(),
            ignore_policy_version: self.ignore_policy_version.clone(),
        }
    }

    fn observe_with_deadline(
        &self,
        deadline: &ObservationDeadline,
    ) -> Result<Observation, GitSnapshotAuthorityError> {
        deadline.check()?;
        let actual_root = canonical_root(&self.root)?;
        if actual_root != self.root {
            return Err(GitSnapshotAuthorityError::RootMismatch);
        }
        let git_root = git_text(
            &run_git(
                &self.root,
                &["rev-parse", "--show-toplevel"],
                deadline,
                MAX_GIT_OUTPUT_BYTES,
            )?,
            false,
            deadline,
        )?;
        let git_root = canonical_root(Path::new(&git_root))?;
        deadline.check()?;
        if git_root != actual_root {
            return Err(GitSnapshotAuthorityError::RootMismatch);
        }

        let common_dir = git_text(
            &run_git(
                &self.root,
                &["rev-parse", "--git-common-dir"],
                deadline,
                MAX_GIT_OUTPUT_BYTES,
            )?,
            false,
            deadline,
        )?;
        let common_dir_path = Path::new(&common_dir);
        let common_dir_path = if common_dir_path.is_absolute() {
            common_dir_path.to_path_buf()
        } else {
            self.root.join(common_dir_path)
        };
        let common_dir = canonical_root(&common_dir_path)?;
        deadline.check()?;

        let head = git_text(
            &run_git(
                &self.root,
                &["rev-parse", "HEAD"],
                deadline,
                MAX_GIT_OUTPUT_BYTES,
            )?,
            true,
            deadline,
        )?;
        let status = run_git(
            &self.root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            deadline,
            MAX_GIT_OUTPUT_BYTES,
        )?;
        let paths = parse_paths(
            &run_git(
                &self.root,
                &[
                    "ls-files",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                    "-z",
                ],
                deadline,
                MAX_GIT_OUTPUT_BYTES,
            )?,
            deadline,
        )?;
        let files = self.inspect_files(&paths, deadline)?;
        deadline.check()?;
        let fingerprint = fingerprint(&head, &status, &files);
        deadline.check()?;
        Ok(Observation {
            head,
            fingerprint,
            common_dir,
            root: actual_root,
            files,
        })
    }

    /// Observe the complete Git-visible source set twice and reject a changed
    /// result. This narrows the race window around a mutable worktree while
    /// remaining deliberately weaker than an atomic filesystem snapshot.
    fn observe_stable(&self) -> Result<Observation, GitSnapshotAuthorityError> {
        let deadline = ObservationDeadline::new(self.command_timeout);
        self.observe_stable_with_deadline(&deadline)
    }

    fn observe_stable_with_deadline(
        &self,
        deadline: &ObservationDeadline,
    ) -> Result<Observation, GitSnapshotAuthorityError> {
        let first = self.observe_with_deadline(deadline)?;
        let second = self.observe_with_deadline(deadline)?;
        deadline.check()?;
        stable_observation(&first, second)
    }

    /// Materialize the observed source set into a private, per-call tree.
    ///
    /// The live worktree is observed before and after the copy. The returned
    /// source owns a directory capability into the temporary tree, while the
    /// owner-scoped temporary directory keeps that tree alive until all
    /// consumers are done. This is a stable, immutable-by-ownership read
    /// surface; it is intentionally not described as an atomic filesystem
    /// snapshot.
    pub fn materialize_source_snapshot(
        &self,
        root: &Path,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<MaterializedSource, GitSnapshotAuthorityError> {
        let deadline = ObservationDeadline::new(self.command_timeout);
        let requested_root = canonical_root(root)?;
        let configured_root = canonical_root(&self.root)?;
        if requested_root != configured_root {
            return Err(GitSnapshotAuthorityError::RootMismatch);
        }

        let observed = self.observe_stable_with_deadline(&deadline)?;
        let actual = self.project_from_observation(&observed);
        if project != &actual {
            return Err(GitSnapshotAuthorityError::ProjectMismatch);
        }
        let binding_id = binding_id(&observed, &actual);
        let binding =
            SourceSnapshotBinding::new(binding_id, actual.clone(), graph_version.to_owned())
                .map_err(|_| GitSnapshotAuthorityError::InvalidMetadata("invalid graph version"))?;

        let temp_dir = OwnedTempDir::new().map_err(GitSnapshotAuthorityError::SourceIo)?;
        let source_directory =
            cap_std::fs::Dir::open_ambient_dir(&self.root, cap_std::ambient_authority())
                .map_err(GitSnapshotAuthorityError::SourceIo)?;
        let destination_directory =
            cap_std::fs::Dir::open_ambient_dir(temp_dir.path(), cap_std::ambient_authority())
                .map_err(GitSnapshotAuthorityError::SourceIo)?;
        copy_observed_files(
            &observed.files,
            &source_directory,
            &destination_directory,
            self.max_total_bytes,
            &deadline,
        )?;

        let source =
            super::DirectorySource::open(temp_dir.path(), actual, graph_version.to_owned())
                .map_err(map_source_open_error)?;
        freeze_tree(temp_dir.path(), &deadline)?;

        let after = self.observe_stable_with_deadline(&deadline)?;
        if observed != after {
            return Err(GitSnapshotAuthorityError::SnapshotChanged);
        }

        Ok(MaterializedSource {
            temp_dir,
            source,
            binding,
        })
    }

    fn inspect_files(
        &self,
        paths: &[String],
        deadline: &ObservationDeadline,
    ) -> Result<Vec<FileObservation>, GitSnapshotAuthorityError> {
        let mut remaining = self.max_total_bytes;
        paths
            .iter()
            .map(|relative| {
                deadline.check()?;
                let path = self.root.join(relative);
                let metadata = match fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        return Ok(FileObservation::deleted(relative.clone()));
                    }
                    Err(error) => return Err(GitSnapshotAuthorityError::SourceIo(error)),
                };
                if metadata.file_type().is_symlink() {
                    let target =
                        fs::read_link(&path).map_err(GitSnapshotAuthorityError::SourceIo)?;
                    deadline.check()?;
                    let target = os_bytes(&target);
                    if target.len() as u64 > remaining {
                        return Err(GitSnapshotAuthorityError::SourceTooLarge);
                    }
                    remaining -= target.len() as u64;
                    let digest = hash(&target);
                    deadline.check()?;
                    return Ok(FileObservation::symlink(relative.clone(), digest));
                }
                if !metadata.is_file() {
                    return Err(GitSnapshotAuthorityError::UnsupportedEntry);
                }
                let (digest, length) = digest_file(&path, &metadata, remaining, deadline)?;
                remaining -= length;
                Ok(FileObservation::file(
                    relative.clone(),
                    executable(&metadata),
                    digest,
                    length,
                ))
            })
            .collect()
    }
}

impl SourceSnapshotAuthority for GitSnapshotAuthority {
    type Error = GitSnapshotAuthorityError;

    fn bind_source_snapshot(
        &self,
        root: &Path,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<SourceSnapshotBinding, Self::Error> {
        let requested_root = canonical_root(root)?;
        let configured_root = canonical_root(&self.root)?;
        if requested_root != configured_root {
            return Err(GitSnapshotAuthorityError::RootMismatch);
        }
        let observed = self.observe_stable()?;
        let actual = self.project_from_observation(&observed);
        if project != &actual {
            return Err(GitSnapshotAuthorityError::ProjectMismatch);
        }
        let binding_id = binding_id(&observed, &actual);
        SourceSnapshotBinding::new(binding_id, actual, graph_version.into())
            .map_err(|_| GitSnapshotAuthorityError::InvalidMetadata("invalid graph version"))
    }
}

fn stable_observation(
    first: &Observation,
    second: Observation,
) -> Result<Observation, GitSnapshotAuthorityError> {
    if *first == second {
        Ok(second)
    } else {
        Err(GitSnapshotAuthorityError::SnapshotChanged)
    }
}

struct ObservationDeadline {
    deadline: Instant,
}

impl ObservationDeadline {
    fn new(timeout: Duration) -> Self {
        Self {
            deadline: Instant::now() + timeout,
        }
    }

    fn remaining(&self) -> Result<Duration, GitSnapshotAuthorityError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            Err(GitSnapshotAuthorityError::SnapshotTimeout)
        } else {
            Ok(remaining)
        }
    }

    fn check(&self) -> Result<(), GitSnapshotAuthorityError> {
        self.remaining().map(|_| ())
    }
}

#[derive(PartialEq, Eq)]
struct Observation {
    head: String,
    fingerprint: String,
    common_dir: PathBuf,
    root: PathBuf,
    files: Vec<FileObservation>,
}

#[derive(Clone, PartialEq, Eq)]
struct FileObservation {
    path: String,
    kind: FileKind,
    executable: bool,
    digest: Option<String>,
    bytes: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Deleted,
    File,
    Symlink,
}

impl FileObservation {
    fn deleted(path: String) -> Self {
        Self {
            path,
            kind: FileKind::Deleted,
            executable: false,
            digest: None,
            bytes: 0,
        }
    }

    fn file(path: String, executable: bool, digest: String, bytes: u64) -> Self {
        Self {
            path,
            kind: FileKind::File,
            executable,
            digest: Some(digest),
            bytes,
        }
    }

    fn symlink(path: String, digest: String) -> Self {
        Self {
            path,
            kind: FileKind::Symlink,
            executable: false,
            digest: Some(digest),
            bytes: 0,
        }
    }
}

fn canonical_root(path: &Path) -> Result<PathBuf, GitSnapshotAuthorityError> {
    fs::canonicalize(path).map_err(GitSnapshotAuthorityError::RootUnavailable)
}

fn git_text(
    bytes: &[u8],
    is_head: bool,
    deadline: &ObservationDeadline,
) -> Result<String, GitSnapshotAuthorityError> {
    deadline.check()?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| GitSnapshotAuthorityError::InvalidGitOutput)?
        .trim_end_matches(['\r', '\n'])
        .to_owned();
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(GitSnapshotAuthorityError::InvalidGitOutput);
    }
    if is_head
        && (text.len() != 40
            || !text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(GitSnapshotAuthorityError::InvalidGitOutput);
    }
    deadline.check()?;
    Ok(text)
}

fn parse_paths(
    bytes: &[u8],
    deadline: &ObservationDeadline,
) -> Result<Vec<String>, GitSnapshotAuthorityError> {
    let mut seen = BTreeSet::new();
    let mut paths = Vec::new();
    for raw in bytes.split(|byte| *byte == 0).filter(|raw| !raw.is_empty()) {
        deadline.check()?;
        let path = std::str::from_utf8(raw)
            .map_err(|_| GitSnapshotAuthorityError::InvalidGitOutput)?
            .to_owned();
        validate_relative_path(&path)?;
        if !seen.insert(path.clone()) {
            return Err(GitSnapshotAuthorityError::DuplicatePath);
        }
        paths.push(path);
    }
    paths.sort();
    deadline.check()?;
    Ok(paths)
}

fn validate_relative_path(path: &str) -> Result<(), GitSnapshotAuthorityError> {
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::RootDir
            )
        })
    {
        return Err(GitSnapshotAuthorityError::InvalidGitOutput);
    }
    Ok(())
}

fn digest_file(
    path: &Path,
    metadata: &Metadata,
    remaining: u64,
    deadline: &ObservationDeadline,
) -> Result<(String, u64), GitSnapshotAuthorityError> {
    deadline.check()?;
    if metadata.len() > remaining {
        return Err(GitSnapshotAuthorityError::SourceTooLarge);
    }
    let mut file = File::open(path).map_err(GitSnapshotAuthorityError::SourceIo)?;
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        deadline.check()?;
        let count = Read::by_ref(&mut file)
            .take(remaining.saturating_sub(bytes).saturating_add(1))
            .read(&mut buffer)
            .map_err(GitSnapshotAuthorityError::SourceIo)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > remaining {
            return Err(GitSnapshotAuthorityError::SourceTooLarge);
        }
        digest.update(&buffer[..count]);
    }
    deadline.check()?;
    Ok((format_digest(digest.finalize()), bytes))
}

fn fingerprint(head: &str, status: &[u8], files: &[FileObservation]) -> String {
    let mut digest = Sha256::new();
    digest.update(FINGERPRINT_VERSION);
    push_text(&mut digest, head);
    push_bytes(&mut digest, status);
    digest.update((files.len() as u64).to_be_bytes());
    for file in files {
        push_text(&mut digest, &file.path);
        digest.update([match file.kind {
            FileKind::Deleted => 0,
            FileKind::File => 1,
            FileKind::Symlink => 2,
        }]);
        digest.update([u8::from(file.executable)]);
        if let Some(digest_hex) = &file.digest {
            push_text(&mut digest, digest_hex);
        }
    }
    format_digest(digest.finalize())
}

fn binding_id(observed: &Observation, project: &ProjectRef) -> String {
    let mut digest = Sha256::new();
    digest.update(b"project-graph/source-snapshot-binding/v1\0");
    push_text(&mut digest, &observed.root.to_string_lossy());
    push_text(&mut digest, &observed.common_dir.to_string_lossy());
    for value in [
        &project.repository_id,
        &project.worktree_id,
        &project.git_head,
        &project.working_tree_fingerprint,
        &project.config_hash,
        &project.ignore_policy_version,
    ] {
        push_text(&mut digest, value);
    }
    format!("git:{:x}", digest.finalize())
}

fn push_text(digest: &mut Sha256, value: &str) {
    push_bytes(digest, value.as_bytes());
}

fn push_bytes(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

fn hash(bytes: &[u8]) -> String {
    format_digest(Sha256::digest(bytes))
}

fn format_digest(digest: impl std::fmt::LowerHex) -> String {
    format!("{digest:x}")
}

#[cfg(unix)]
fn os_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

#[cfg(unix)]
fn executable(metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_: &Metadata) -> bool {
    false
}

fn run_git(
    root: &Path,
    args: &[&str],
    deadline: &ObservationDeadline,
    max_output: usize,
) -> Result<Vec<u8>, GitSnapshotAuthorityError> {
    deadline.check()?;
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(GitSnapshotAuthorityError::GitStart)?;
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(GitSnapshotAuthorityError::GitFailed);
        }
    };
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || sender.send(read_bounded(stdout, max_output)));
    let mut received_output: Option<Result<Vec<u8>, ReadLimit>> = None;
    let result = loop {
        if received_output.is_none() {
            match receiver.try_recv() {
                Ok(Err(ReadLimit::Exceeded)) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    break Err(GitSnapshotAuthorityError::GitOutputTooLarge);
                }
                Ok(Err(ReadLimit::Io(error))) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    break Err(GitSnapshotAuthorityError::SourceIo(error));
                }
                Ok(Ok(bytes)) => received_output = Some(Ok(bytes)),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    break Err(GitSnapshotAuthorityError::GitFailed);
                }
            }
        }
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                break Err(GitSnapshotAuthorityError::GitStart(error));
            }
        };
        match status {
            Some(status) => {
                let read_result = match received_output.take() {
                    Some(read_result) => {
                        if reader.join().is_err() {
                            break Err(GitSnapshotAuthorityError::GitFailed);
                        }
                        read_result
                    }
                    None => match reader.join() {
                        Ok(Ok(())) => match receiver.recv() {
                            Ok(read_result) => read_result,
                            Err(_) => break Err(GitSnapshotAuthorityError::GitFailed),
                        },
                        Ok(Err(_)) => {
                            break Err(GitSnapshotAuthorityError::GitFailed);
                        }
                        Err(_) => break Err(GitSnapshotAuthorityError::GitFailed),
                    },
                };
                let bytes = read_result.map_err(map_read_limit)?;
                deadline.check()?;
                if status.success() {
                    break Ok(bytes);
                }
                break Err(GitSnapshotAuthorityError::GitFailed);
            }
            None if deadline.remaining().is_err() => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                break Err(GitSnapshotAuthorityError::SnapshotTimeout);
            }
            None => {
                let remaining = deadline.remaining()?;
                thread::sleep(std::cmp::min(remaining, Duration::from_millis(5)));
            }
        }
    };
    result
}

fn map_read_limit(error: ReadLimit) -> GitSnapshotAuthorityError {
    match error {
        ReadLimit::Exceeded => GitSnapshotAuthorityError::GitOutputTooLarge,
        ReadLimit::Io(error) => GitSnapshotAuthorityError::SourceIo(error),
    }
}

/// A self-contained source reader backed by a private materialized tree.
///
/// The path is useful only while this value is alive; callers must not retain
/// it after drop. The source reader itself uses a capability directory handle,
/// and the tree is made read-only after materialization on Unix.
pub struct MaterializedSource {
    source: super::DirectorySource,
    temp_dir: OwnedTempDir,
    binding: SourceSnapshotBinding,
}

impl MaterializedSource {
    pub fn source(&self) -> &super::DirectorySource {
        &self.source
    }

    pub fn snapshot(&self) -> &SourceSnapshotBinding {
        &self.binding
    }

    pub fn root(&self) -> &Path {
        self.temp_dir.path()
    }
}

fn copy_observed_files(
    files: &[FileObservation],
    source_directory: &cap_std::fs::Dir,
    destination_directory: &cap_std::fs::Dir,
    max_total_bytes: u64,
    deadline: &ObservationDeadline,
) -> Result<(), GitSnapshotAuthorityError> {
    deadline.check()?;
    let mut remaining = max_total_bytes;
    for file in files {
        deadline.check()?;
        if matches!(file.kind, FileKind::Deleted) {
            continue;
        }
        let path = Path::new(&file.path);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            destination_directory
                .create_dir_all(parent)
                .map_err(GitSnapshotAuthorityError::SourceIo)?;
        }
        let resolved;
        let (content, source_path) = if matches!(file.kind, FileKind::Symlink) {
            resolved = source_directory
                .canonicalize(path)
                .map_err(GitSnapshotAuthorityError::SourceIo)?;
            let target = files
                .iter()
                .find(|candidate| {
                    Path::new(&candidate.path) == resolved
                        && matches!(candidate.kind, FileKind::File)
                })
                .ok_or(GitSnapshotAuthorityError::UnsupportedEntry)?;
            (target, resolved.as_path())
        } else {
            (file, path)
        };
        let copied = copy_observed_file(
            content,
            source_path,
            path,
            source_directory,
            destination_directory,
            remaining,
            deadline,
        )?;
        remaining = remaining
            .checked_sub(copied)
            .ok_or(GitSnapshotAuthorityError::SourceTooLarge)?;
    }
    Ok(())
}

fn copy_observed_file(
    observation: &FileObservation,
    source_path: &Path,
    path: &Path,
    source_directory: &cap_std::fs::Dir,
    destination_directory: &cap_std::fs::Dir,
    remaining: u64,
    deadline: &ObservationDeadline,
) -> Result<u64, GitSnapshotAuthorityError> {
    deadline.check()?;
    let follow_final = matches!(observation.kind, FileKind::Symlink);
    let mut source_options = cap_std::fs::OpenOptions::new();
    source_options.read(true);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        let mut flags = rustix::fs::OFlags::NONBLOCK;
        if !follow_final {
            flags |= rustix::fs::OFlags::NOFOLLOW;
        }
        source_options.custom_flags(flags.bits() as i32);
    }
    let source = source_directory
        .open_with(source_path, &source_options)
        .map_err(GitSnapshotAuthorityError::SourceIo)?;
    let metadata = source
        .metadata()
        .map_err(GitSnapshotAuthorityError::SourceIo)?;
    if !metadata.is_file() {
        return Err(GitSnapshotAuthorityError::UnsupportedEntry);
    }
    if matches!(observation.kind, FileKind::File) && metadata.len() != observation.bytes {
        return Err(GitSnapshotAuthorityError::SnapshotChanged);
    }
    if metadata.len() > remaining {
        return Err(GitSnapshotAuthorityError::SourceTooLarge);
    }

    let mut destination_options = cap_std::fs::OpenOptions::new();
    destination_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        destination_options.mode(if observation.executable { 0o700 } else { 0o600 });
    }
    let mut destination = destination_directory
        .open_with(path, &destination_options)
        .map_err(GitSnapshotAuthorityError::SourceIo)?;
    let copy_limit = metadata.len();
    let mut bounded = source.take(copy_limit.saturating_add(1));
    let mut digest = Sha256::new();
    let mut copied = 0_u64;
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        deadline.check()?;
        let count = bounded
            .read(&mut buffer)
            .map_err(GitSnapshotAuthorityError::SourceIo)?;
        if count == 0 {
            break;
        }
        copied = copied
            .checked_add(count as u64)
            .ok_or(GitSnapshotAuthorityError::SourceTooLarge)?;
        if copied > remaining || copied > copy_limit {
            return Err(GitSnapshotAuthorityError::SourceTooLarge);
        }
        destination
            .write_all(&buffer[..count])
            .map_err(GitSnapshotAuthorityError::SourceIo)?;
        digest.update(&buffer[..count]);
    }
    if copied != copy_limit {
        return Err(GitSnapshotAuthorityError::SnapshotChanged);
    }
    if matches!(observation.kind, FileKind::File)
        && format_digest(digest.finalize()) != observation.digest.as_deref().unwrap_or_default()
    {
        return Err(GitSnapshotAuthorityError::SnapshotChanged);
    }
    destination
        .sync_all()
        .map_err(GitSnapshotAuthorityError::SourceIo)?;
    deadline.check()?;
    Ok(copied)
}

fn map_source_open_error(error: super::SourceReadError) -> GitSnapshotAuthorityError {
    match error {
        super::SourceReadError::Io(error) => GitSnapshotAuthorityError::SourceIo(error),
        _ => GitSnapshotAuthorityError::InvalidMetadata("materialized source could not be opened"),
    }
}

#[cfg(unix)]
fn freeze_tree(
    root: &Path,
    deadline: &ObservationDeadline,
) -> Result<(), GitSnapshotAuthorityError> {
    freeze_directory(root, deadline)
}

#[cfg(not(unix))]
fn freeze_tree(_: &Path, deadline: &ObservationDeadline) -> Result<(), GitSnapshotAuthorityError> {
    deadline.check()
}

#[cfg(unix)]
fn freeze_directory(
    path: &Path,
    deadline: &ObservationDeadline,
) -> Result<(), GitSnapshotAuthorityError> {
    deadline.check()?;
    for entry in fs::read_dir(path).map_err(GitSnapshotAuthorityError::SourceIo)? {
        deadline.check()?;
        let entry = entry.map_err(GitSnapshotAuthorityError::SourceIo)?;
        let child = entry.path();
        let metadata = fs::symlink_metadata(&child).map_err(GitSnapshotAuthorityError::SourceIo)?;
        if metadata.is_dir() {
            freeze_directory(&child, deadline)?;
        } else if metadata.is_file() {
            let executable = executable(&metadata);
            fs::set_permissions(
                &child,
                Permissions::from_mode(if executable { 0o500 } else { 0o400 }),
            )
            .map_err(GitSnapshotAuthorityError::SourceIo)?;
        } else {
            return Err(GitSnapshotAuthorityError::SourceIo(io::Error::new(
                io::ErrorKind::InvalidData,
                "materialized tree contains a non-regular entry",
            )));
        }
    }
    fs::set_permissions(path, Permissions::from_mode(0o500))
        .map_err(GitSnapshotAuthorityError::SourceIo)?;
    deadline.check()
}

#[cfg(unix)]
fn thaw_tree(root: &Path) {
    let _ = thaw_directory(root);
}

#[cfg(not(unix))]
fn thaw_tree(_: &Path) {}

#[cfg(unix)]
fn thaw_directory(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let metadata = fs::symlink_metadata(&child)?;
        if metadata.is_dir() {
            thaw_directory(&child)?;
        } else if metadata.is_file() {
            fs::set_permissions(&child, Permissions::from_mode(0o600))?;
        }
    }
    fs::set_permissions(path, Permissions::from_mode(0o700))
}

enum ReadLimit {
    Exceeded,
    Io(io::Error),
}

fn read_bounded(mut reader: impl Read, max_output: usize) -> Result<Vec<u8>, ReadLimit> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(ReadLimit::Io)?;
        if count == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(count) > max_output {
            return Err(ReadLimit::Exceeded);
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

#[cfg(test)]
mod tests {
    use sha2::Digest;
    #[cfg(unix)]
    #[test]
    fn temporary_owner_cleans_frozen_tree_before_publication() {
        for fully_frozen in [false, true] {
            let owner = super::OwnedTempDir::new().unwrap();
            let root = owner.path().to_path_buf();
            let nested = root.join("nested");
            std::fs::create_dir(&nested).unwrap();
            std::fs::write(nested.join("source.rs"), b"source").unwrap();
            super::freeze_tree(
                if fully_frozen { &root } else { &nested },
                &ObservationDeadline::new(Duration::from_secs(5)),
            )
            .unwrap();
            drop(owner);
            assert!(!root.exists(), "unpublished frozen tree leaked");
        }
    }

    use super::{GitSnapshotAuthorityError, Observation, ObservationDeadline, stable_observation};
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    fn observation(fingerprint: &str) -> Observation {
        Observation {
            head: "0123456789012345678901234567890123456789".into(),
            fingerprint: fingerprint.into(),
            common_dir: PathBuf::from("/repo/.git"),
            root: PathBuf::from("/repo"),
            files: Vec::new(),
        }
    }

    #[test]
    fn identical_observations_are_accepted() {
        let first = observation("same");
        let result = stable_observation(&first, observation("same"));
        assert!(result.is_ok());
    }

    #[test]
    fn changed_observations_are_rejected() {
        let first = observation("before");
        let result = stable_observation(&first, observation("after"));
        assert!(matches!(
            result,
            Err(super::GitSnapshotAuthorityError::SnapshotChanged)
        ));
    }

    #[test]
    fn expired_observation_deadline_is_rejected_without_waiting() {
        let deadline = ObservationDeadline {
            deadline: Instant::now()
                .checked_sub(Duration::from_secs(1))
                .expect("one second can be subtracted from now"),
        };
        assert!(matches!(
            deadline.remaining(),
            Err(GitSnapshotAuthorityError::SnapshotTimeout)
        ));
    }

    #[test]
    fn expired_materialization_budget_rejects_copy_and_freeze_before_side_effects() {
        let source = super::OwnedTempDir::new().unwrap();
        let destination = super::OwnedTempDir::new().unwrap();
        std::fs::write(source.path().join("input"), b"original").unwrap();
        let source_dir =
            cap_std::fs::Dir::open_ambient_dir(source.path(), cap_std::ambient_authority())
                .unwrap();
        let destination_dir =
            cap_std::fs::Dir::open_ambient_dir(destination.path(), cap_std::ambient_authority())
                .unwrap();
        let deadline = ObservationDeadline::new(Duration::ZERO);
        let observation = super::FileObservation {
            path: "input".into(),
            kind: super::FileKind::File,
            executable: false,
            digest: Some(super::format_digest(super::Sha256::digest(b"original"))),
            bytes: 8,
        };
        assert!(matches!(
            super::copy_observed_files(
                &[observation.clone()],
                &source_dir,
                &destination_dir,
                4096,
                &deadline
            ),
            Err(GitSnapshotAuthorityError::SnapshotTimeout)
        ));
        assert!(matches!(
            super::copy_observed_file(
                &observation,
                std::path::Path::new("input"),
                std::path::Path::new("input"),
                &source_dir,
                &destination_dir,
                4096,
                &deadline
            ),
            Err(GitSnapshotAuthorityError::SnapshotTimeout)
        ));
        assert_eq!(std::fs::read_dir(destination.path()).unwrap().count(), 0);
        assert!(matches!(
            super::freeze_tree(destination.path(), &deadline),
            Err(GitSnapshotAuthorityError::SnapshotTimeout)
        ));
        std::fs::write(destination.path().join("still-writable"), b"ok").unwrap();
    }

    #[test]
    fn future_observation_deadline_reports_positive_remaining_time() {
        let deadline = ObservationDeadline::new(Duration::from_secs(1));
        assert!(deadline.remaining().unwrap() > Duration::ZERO);
    }
}
