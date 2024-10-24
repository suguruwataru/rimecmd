use crate::json_request_processor::Reply;
use crate::poll_data::{PollData, ReadData};
use crate::Result;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

pub struct Client {
    server_stream: UnixStream,
    json_bytes: Vec<u8>,
}

pub enum ServerReply {
    Complete(Reply),
    Incomplete,
}

impl<D: From<ServerReply>> ReadData<D> for Client {
    fn read_data(&mut self) -> Result<D> {
        let mut buf = [0; 1024];
        let count = self.server_stream.read(&mut buf)?;
        self.json_bytes.extend_from_slice(&buf[0..count]);
        match serde_json::from_slice::<Reply>(&self.json_bytes) {
            Ok(reply) => {
                self.json_bytes.clear();
                Ok(ServerReply::Complete(reply).into())
            }
            Err(err) if err.is_eof() => Ok(ServerReply::Incomplete.into()),
            Err(err) => Err(err.into()),
        }
    }

    fn register(&self, poll_data: &mut PollData<D>) -> Result<()> {
        poll_data.register(&self.server_stream)?;
        Ok(())
    }
}

impl Client {
    pub fn new(server_socket: UnixStream) -> Self {
        Self {
            server_stream: server_socket,
            json_bytes: vec![],
        }
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.server_stream.write(&bytes)?;
        self.server_stream.flush()?;
        Ok(())
    }

    pub fn shutdown(self) -> Result<()> {
        self.server_stream.shutdown(Shutdown::Both)?;
        Ok(())
    }
}
