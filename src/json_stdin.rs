use crate::json_request_processor::Request;
use crate::poll_request::{PollRequest, RequestSource};
use crate::{Error, Result};
use std::io::Read;
use std::os::fd::AsRawFd;
pub struct JsonStdin {
    stdin: std::io::Stdin,
}

impl JsonStdin {
    pub fn new() -> Self {
        Self {
            stdin: std::io::stdin(),
        }
    }
}

impl RequestSource for JsonStdin {
    fn register(&self, poll_request: &mut PollRequest) -> Result<()> {
        poll_request.register(&self.stdin.as_raw_fd())
    }

    fn next_request(&mut self) -> Result<Request> {
        let mut buf = [0u8; 1024];
        let mut json_bytes = vec![];
        loop {
            let count = self.stdin.read(&mut buf)?;
            if count == 0 {
                break Err(Error::InputClosed);
            }
            json_bytes.extend_from_slice(&buf[0..count]);
            match serde_json::from_slice::<Request>(&json_bytes) {
                Ok(call) => break Ok(call),
                Err(err) => {
                    if err.is_eof() {
                        continue;
                    }
                    break Err(crate::Error::Json(err));
                }
            };
        }
    }
}
