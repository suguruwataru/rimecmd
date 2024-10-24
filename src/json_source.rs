use crate::poll_data::{PollData, ReadData};
use crate::{Error, Result};
use serde::de::DeserializeOwned;
use std::io::Read;
use std::os::fd::AsRawFd;

pub struct JsonSource<I: Read + AsRawFd> {
    src: I,
}

impl<I: Read + AsRawFd> JsonSource<I> {
    pub fn new(source: I) -> Self {
        Self { src: source }
    }
}

impl<I: Read + AsRawFd, D: DeserializeOwned> ReadData<D> for JsonSource<I> {
    fn register(&self, poll_request: &mut PollData<D>) -> Result<()> {
        poll_request.register(&self.src.as_raw_fd())
    }

    fn read_data(&mut self) -> Result<D> {
        let mut buf = [0u8; 1024];
        let mut json_bytes = vec![];
        loop {
            let count = self.src.read(&mut buf)?;
            if count == 0 {
                break Err(Error::InputClosed);
            }
            json_bytes.extend_from_slice(&buf[0..count]);
            match serde_json::from_slice::<D>(&json_bytes) {
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
