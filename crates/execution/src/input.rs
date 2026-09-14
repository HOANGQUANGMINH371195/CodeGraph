use std::{
    io::{self, Write},
    process::ChildStdin,
};

/// Byte delivery observations, never peer acknowledgement or execution proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct InputProgress {
    pub total: usize,
    pub written: usize,
    pub failed: bool,
}

/// Linux pipe owner. One pending frame; no unbounded queue or background thread.
/// The caller retains RPC metadata independently until a correlated response.
pub struct NonblockingInput {
    pipe: ChildStdin,
    frame: Vec<u8>,
    progress: InputProgress,
    max_frame: usize,
}
impl NonblockingInput {
    pub fn new(pipe: ChildStdin, max_frame: usize) -> io::Result<Self> {
        if max_frame == 0 || max_frame > 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid input frame cap",
            ));
        }
        let flags = rustix::fs::fcntl_getfl(&pipe)?;
        rustix::fs::fcntl_setfl(&pipe, flags | rustix::fs::OFlags::NONBLOCK)?;
        Ok(Self {
            pipe,
            frame: Vec::new(),
            progress: InputProgress {
                total: 0,
                written: 0,
                failed: false,
            },
            max_frame,
        })
    }

    /// Admission only. Caller must not replace an unfinished/failed frame.
    pub fn begin(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.progress.failed || self.progress.written < self.progress.total {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "input frame unresolved",
            ));
        }
        if bytes.is_empty() || bytes.len() > self.max_frame {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid input frame size",
            ));
        }
        self.frame.clear();
        self.frame.extend_from_slice(bytes);
        self.progress = InputProgress {
            total: bytes.len(),
            written: 0,
            failed: false,
        };
        Ok(())
    }

    pub fn progress(&self) -> InputProgress {
        self.progress
    }

    /// At most 8 KiB and one OS write; WouldBlock/Interrupted preserve progress.
    /// After any other error, no further writes are attempted on this pipe.
    pub fn pump(&mut self) -> io::Result<InputProgress> {
        if self.progress.failed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "input writer previously failed",
            ));
        }
        if self.progress.written == self.progress.total {
            return Ok(self.progress);
        }
        let end = self.progress.total.min(self.progress.written + 8192);
        match self.pipe.write(&self.frame[self.progress.written..end]) {
            Ok(0) => {
                self.progress.failed = true;
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "input write made no progress",
                ));
            }
            Ok(count) => self.progress.written += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => {
                self.progress.failed = true;
                return Err(error);
            }
        }
        Ok(self.progress)
    }

    /// Close stdin without draining a blocked write. Progress remains uncertain
    /// even when written == total; close is not an RPC acknowledgement.
    pub fn close(self) -> InputProgress {
        self.progress
    }
}
