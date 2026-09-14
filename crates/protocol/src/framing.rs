//! Incremental LF-delimited bytes. No I/O, JSON parsing or lifecycle authority.
//! Callers must bound input reads and output queues separately.

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("frame limit must be between 1 byte and 16 MiB, including delimiter")]
    InvalidLimit,
    #[error("frame exceeds byte limit")]
    TooLarge,
    #[error("frame buffer allocation failed")]
    Allocation,
    #[error("EOF before frame delimiter")]
    Truncated,
    #[error("decoder is already finished or failed")]
    Closed,
}

#[derive(Debug)]
enum State {
    Open,
    Closed,
}

#[derive(Debug)]
pub struct LineDecoder {
    limit: usize,
    pending: Vec<u8>,
    state: State,
}

impl LineDecoder {
    pub fn new(limit: usize) -> Result<Self, FrameError> {
        if !(1..=16 * 1024 * 1024).contains(&limit) {
            return Err(FrameError::InvalidLimit);
        }
        Ok(Self {
            limit,
            pending: Vec::new(),
            state: State::Open,
        })
    }

    /// Consume at most one frame. On success, resubmit input[consumed..] after
    /// handling the returned frame. Empty frames are returned for the JSON
    /// router to reject. The cap counts LF and CR, even though they are removed.
    /// On any error, discard this decoder/connection; no resynchronization.
    pub fn push(&mut self, input: &[u8]) -> Result<(usize, Option<Vec<u8>>), FrameError> {
        if matches!(self.state, State::Closed) {
            return Err(FrameError::Closed);
        }
        let remaining = self.limit - self.pending.len();
        let inspected = &input[..input.len().min(remaining)];
        let newline = inspected.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(inspected.len(), |index| index + 1);
        // At the limit without LF there is no room for a required delimiter.
        if newline.is_none() && consumed == remaining {
            self.close();
            return Err(FrameError::TooLarge);
        }
        if self.pending.try_reserve_exact(consumed).is_err() {
            self.close();
            return Err(FrameError::Allocation);
        }
        self.pending.extend_from_slice(&inspected[..consumed]);
        if newline.is_some() {
            self.pending.pop();
            if self.pending.last() == Some(&b'\r') {
                self.pending.pop();
            }
            return Ok((consumed, Some(std::mem::take(&mut self.pending))));
        }
        Ok((consumed, None))
    }

    /// Call only on actual transport EOF, never on read timeout/empty chunk.
    pub fn finish(&mut self) -> Result<(), FrameError> {
        if matches!(self.state, State::Closed) {
            return Err(FrameError::Closed);
        }
        let incomplete = !self.pending.is_empty();
        self.close();
        if incomplete {
            Err(FrameError::Truncated)
        } else {
            Ok(())
        }
    }

    fn close(&mut self) {
        self.state = State::Closed;
        self.pending = Vec::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_crlf_and_multiple_frames_are_independent_of_chunking() {
        let input = "xin chào 🌱\r\n{}\n\n".as_bytes();
        for chunk_size in 1..=input.len() {
            let mut decoder = LineDecoder::new(64).unwrap();
            let mut frames = Vec::new();
            for chunk in input.chunks(chunk_size) {
                let mut rest = chunk;
                while !rest.is_empty() {
                    let (consumed, frame) = decoder.push(rest).unwrap();
                    assert!(consumed > 0);
                    rest = &rest[consumed..];
                    if let Some(frame) = frame {
                        frames.push(frame);
                    }
                }
            }
            assert_eq!(
                frames,
                vec!["xin chào 🌱".as_bytes().to_vec(), b"{}".to_vec(), vec![]]
            );
            assert_eq!(decoder.finish(), Ok(()));
        }
    }

    #[test]
    fn cap_includes_delimiter_and_failure_cannot_resynchronize() {
        let mut exact = LineDecoder::new(4).unwrap();
        assert_eq!(exact.push(b"{}\r\nnext"), Ok((4, Some(b"{}".to_vec()))));
        assert_eq!(exact.push(b"1234\n{}\n"), Err(FrameError::TooLarge));
        assert!(exact.pending.is_empty());
        assert_eq!(exact.push(b"{}\n"), Err(FrameError::Closed));
        let mut split = LineDecoder::new(4).unwrap();
        assert_eq!(split.push(b"123"), Ok((3, None)));
        assert_eq!(split.push(b"4"), Err(FrameError::TooLarge));
        let mut one = LineDecoder::new(1).unwrap();
        assert_eq!(one.push(b"\n"), Ok((1, Some(vec![]))));
        assert_eq!(one.push(b"x"), Err(FrameError::TooLarge));
    }

    #[test]
    fn eof_is_explicit_and_truncation_never_emits_partial_json() {
        let mut decoder = LineDecoder::new(16).unwrap();
        assert_eq!(decoder.push(b""), Ok((0, None)));
        assert_eq!(decoder.push(b"{}"), Ok((2, None)));
        assert_eq!(decoder.push(b""), Ok((0, None)));
        assert_eq!(decoder.finish(), Err(FrameError::Truncated));
        assert_eq!(decoder.finish(), Err(FrameError::Closed));
        assert_eq!(decoder.push(b"\n"), Err(FrameError::Closed));
        let mut clean = LineDecoder::new(16).unwrap();
        assert_eq!(clean.finish(), Ok(()));
        assert_eq!(clean.push(b"{}\n"), Err(FrameError::Closed));
        for limit in [0, 16 * 1024 * 1024 + 1, usize::MAX] {
            assert!(matches!(
                LineDecoder::new(limit),
                Err(FrameError::InvalidLimit)
            ));
        }
    }
}
