//! Bounded journal metadata, never recovered execution authority.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    journal_version: u32,
    receipt: crate::RpcTerminalReceipt,
}

struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}
impl std::io::Write for BoundedBuffer {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        if input.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(std::io::Error::other(
                "journal manifest exceeds byte budget",
            ));
        }
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("journal input could not be read")]
    Io(#[from] std::io::Error),
    #[error("journal manifest exceeds byte budget")]
    TooLarge,
    #[error("invalid journal JSON")]
    Json(#[from] serde_json::Error),
    #[error("unsupported journal version")]
    Version,
    #[error("invalid journal receipt")]
    Receipt(#[from] crate::ProtocolError),
    #[error("journal launch differs from expected launch")]
    LaunchMismatch,
    #[error("journal requires both output descriptors within cumulative budget")]
    Outputs,
}

/// Reads at most the manifest cap plus one byte to detect oversize input.
/// The caller owns path selection, regular-file checks and I/O deadlines.
/// Missing files and other I/O failures remain errors, never absent receipts.
pub fn decode_reader(
    reader: impl std::io::Read,
    expected: &graph_domain::RpcLaunchSpec,
    max_manifest_bytes: usize,
    max_output_bytes: u64,
) -> Result<graph_domain::RpcTerminalReceipt, JournalError> {
    use std::io::Read;
    let limit = u64::try_from(max_manifest_bytes)
        .ok()
        .and_then(|cap| cap.checked_add(1))
        .ok_or(JournalError::TooLarge)?;
    let mut bytes = Vec::new();
    reader.take(limit).read_to_end(&mut bytes)?;
    decode(&bytes, expected, max_manifest_bytes, max_output_bytes)
}

fn validate(
    receipt: &graph_domain::RpcTerminalReceipt,
    expected: &graph_domain::RpcLaunchSpec,
    max_output_bytes: u64,
) -> Result<(), JournalError> {
    if receipt.spawn().launch() != expected {
        return Err(JournalError::LaunchMismatch);
    }
    let (Some(stdout), Some(stderr)) = (receipt.stdout(), receipt.stderr()) else {
        return Err(JournalError::Outputs);
    };
    if stdout
        .byte_length()
        .checked_add(stderr.byte_length())
        .is_none_or(|total| total > max_output_bytes)
    {
        return Err(JournalError::Outputs);
    }
    Ok(())
}

/// Encodes historical metadata. Caller must separately persist and verify bytes.
pub fn encode(
    receipt: &graph_domain::RpcTerminalReceipt,
    expected: &graph_domain::RpcLaunchSpec,
    max_manifest_bytes: usize,
    max_output_bytes: u64,
) -> Result<Vec<u8>, JournalError> {
    validate(receipt, expected, max_output_bytes)?;
    let mut buffer = BoundedBuffer {
        bytes: Vec::new(),
        limit: max_manifest_bytes,
        exceeded: false,
    };
    let result = serde_json::to_writer(
        &mut buffer,
        &Manifest {
            journal_version: 1,
            receipt: receipt.into(),
        },
    );
    if buffer.exceeded {
        return Err(JournalError::TooLarge);
    }
    result?;
    Ok(buffer.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejected_write_never_extends_manifest_buffer() {
        let mut buffer = BoundedBuffer {
            bytes: Vec::new(),
            limit: 3,
            exceeded: false,
        };
        buffer.write_all(b"ab").unwrap();
        assert!(buffer.write_all(b"cd").is_err());
        assert_eq!(buffer.bytes, b"ab");
        assert!(buffer.exceeded);
        let mut empty = BoundedBuffer {
            bytes: Vec::new(),
            limit: 0,
            exceeded: false,
        };
        assert!(empty.write_all(b"x").is_err());
        assert!(empty.bytes.is_empty());
    }
}

/// Checks the cap before JSON parsing; no process or blob capability is created.
pub fn decode(
    bytes: &[u8],
    expected: &graph_domain::RpcLaunchSpec,
    max_manifest_bytes: usize,
    max_output_bytes: u64,
) -> Result<graph_domain::RpcTerminalReceipt, JournalError> {
    if bytes.len() > max_manifest_bytes {
        return Err(JournalError::TooLarge);
    }
    let manifest: Manifest = serde_json::from_slice(bytes)?;
    if manifest.journal_version != 1 {
        return Err(JournalError::Version);
    }
    let receipt = manifest.receipt.try_into_domain()?;
    validate(&receipt, expected, max_output_bytes)?;
    Ok(receipt)
}
