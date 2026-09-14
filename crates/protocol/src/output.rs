use std::io::{self, Write};

/// Encode a whole JSON line before publishing anything. Bounds serialized bytes
/// including LF, not input objects, allocator overhead or OS write atomicity.
/// This generic utility does not enforce method-specific RPC validation.
pub fn json_line<T: serde::Serialize + ?Sized>(value: &T, max_bytes: usize) -> io::Result<Vec<u8>> {
    let mut buffer = LimitedBuffer {
        bytes: Vec::new(),
        max_bytes,
    };
    serde_json::to_writer(&mut buffer, value).map_err(io::Error::other)?;
    buffer.write_all(b"\n")?;
    Ok(buffer.bytes)
}

struct LimitedBuffer {
    bytes: Vec<u8>,
    max_bytes: usize,
}

impl Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("JSON output exceeds byte budget"));
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
