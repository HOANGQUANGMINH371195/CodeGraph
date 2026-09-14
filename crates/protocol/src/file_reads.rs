//! Untrusted process-boundary report from CodeGraph `file-reads`. This wire
//! shape carries capture claims only; consumers must re-read bytes under a
//! host-granted root before creating evidence or graph candidates.

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub kind: String,
    pub status: String,
    #[serde(rename = "sourceCoordinateEncoding")]
    pub source_coordinate_encoding: String,
    pub source: Option<CapturedFile>,
    /// Discovery has a versioned but extractor-owned nested schema. The
    /// application adapter validates the narrow candidate subset it consumes.
    pub discovery: Option<serde_json::Value>,
    pub targets: Vec<Target>,
    pub reason: Option<String>,
    #[serde(rename = "runtimeVerified")]
    pub runtime_verified: bool,
    #[serde(rename = "atomicSnapshotVerified")]
    pub atomic_snapshot_verified: bool,
    #[serde(rename = "raceFreeContainmentVerified")]
    pub race_free_containment_verified: bool,
    #[serde(rename = "requiresStableFilesystem")]
    pub requires_stable_filesystem: bool,
    pub persisted: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedFile {
    pub path: String,
    pub sha256: String,
    #[serde(rename = "byteLength")]
    pub byte_length: u64,
    #[serde(rename = "lineCount")]
    pub line_count: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub status: String,
    #[serde(rename = "runtimeVerified")]
    pub runtime_verified: bool,
    #[serde(rename = "atomicSnapshotVerified")]
    pub atomic_snapshot_verified: bool,
    #[serde(rename = "raceFreeContainmentVerified")]
    pub race_free_containment_verified: bool,
    #[serde(rename = "containmentChecksPassed")]
    pub containment_checks_passed: bool,
    #[serde(rename = "requiresStableFilesystem")]
    pub requires_stable_filesystem: bool,
    pub reason: Option<String>,
    pub source: Option<CapturedFile>,
    pub target: Option<CapturedFile>,
    #[serde(rename = "candidateIndex")]
    pub candidate_index: u32,
}

impl Report {
    pub fn valid_candidate_transport(&self) -> bool {
        self.schema_version == 1
            && self.kind == "file_reads"
            && self.status == "observed"
            && self.source_coordinate_encoding == "utf16-code-unit"
            && !self.runtime_verified
            && !self.atomic_snapshot_verified
            && !self.race_free_containment_verified
            && self.requires_stable_filesystem
            && !self.persisted
    }

    /// Extract only the candidate extent consumed by the harness. Nested
    /// discovery fields remain extractor-owned, but every value used for a
    /// durable link is type-checked and bound to the report source hash.
    pub fn candidate_read_extents(&self) -> Result<Vec<ReadExtent>, &'static str> {
        let source = self.source.as_ref().ok_or("missing file-read source")?;
        let discovery = self
            .discovery
            .as_ref()
            .ok_or("missing file-read discovery")?;
        let object = discovery.as_object().ok_or("invalid file-read discovery")?;
        if object.get("coordinateEncoding").and_then(Value::as_str) != Some("utf16-code-unit") {
            return Err("invalid file-read discovery coordinate encoding");
        }
        if object.get("sourceSha256").and_then(Value::as_str) != Some(source.sha256.as_str()) {
            return Err("file-read discovery source hash mismatch");
        }
        let candidates = object
            .get("candidates")
            .and_then(Value::as_array)
            .ok_or("missing file-read candidates")?;
        candidates
            .iter()
            .map(|candidate| {
                let candidate = candidate.as_object().ok_or("invalid file-read candidate")?;
                if candidate.get("status").and_then(Value::as_str) != Some("candidate")
                    || candidate.get("sourceSha256").and_then(Value::as_str)
                        != Some(source.sha256.as_str())
                {
                    return Err("invalid file-read candidate status or source hash");
                }
                let invocation = candidate
                    .get("read")
                    .and_then(Value::as_object)
                    .and_then(|read| read.get("invocation"))
                    .and_then(Value::as_object)
                    .ok_or("missing file-read invocation")?;
                let start = invocation
                    .get("start")
                    .and_then(Value::as_u64)
                    .ok_or("invalid file-read invocation start")?;
                let end = invocation
                    .get("end")
                    .and_then(Value::as_u64)
                    .ok_or("invalid file-read invocation end")?;
                let start =
                    u32::try_from(start).map_err(|_| "file-read invocation start out of range")?;
                let end =
                    u32::try_from(end).map_err(|_| "file-read invocation end out of range")?;
                if start >= end {
                    return Err("invalid file-read invocation extent");
                }
                Ok(ReadExtent { start, end })
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadExtent {
    pub start: u32,
    pub end: u32,
}
