use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub const MAX_JSON_BYTES: u64 = 8 * 1024 * 1024;

/// Byte cap only: opening/reading a special file may still block. This is not
/// a sandbox, a filesystem snapshot, or a parser-allocation/deadline bound.
pub fn read_json_bytes(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    read_limited(File::open(path)?, MAX_JSON_BYTES)
}

fn read_limited(reader: impl Read, limit: u64) -> io::Result<Vec<u8>> {
    let sentinel_limit = limit
        .checked_add(1)
        .ok_or_else(|| io::Error::other("invalid JSON input byte limit"))?;
    let mut bytes = Vec::new();
    reader.take(sentinel_limit).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::other("JSON input exceeds 8 MiB byte limit"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_boundary_empty_and_infinite_reader_are_bounded() {
        assert_eq!(read_limited(b"abc".as_slice(), 3).unwrap(), b"abc");
        assert!(read_limited(b"abcd".as_slice(), 3).is_err());
        assert!(read_limited(io::empty(), 0).unwrap().is_empty());
        struct Counting {
            count: usize,
        }
        impl Read for Counting {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                out.fill(b'x');
                self.count += out.len();
                Ok(out.len())
            }
        }
        let mut reader = Counting { count: 0 };
        assert!(read_limited(&mut reader, 1024).is_err());
        assert_eq!(reader.count, 1025);
    }
}
