use crate::request_handler::{Request, RequestHandler, Response};
use crate::rime_api::key_mappings::{
    rime_character_to_key_name_map, rime_key_name_to_key_code_map,
};
use std::collections::HashMap;
use std::io::Write;
use std::os::fd::AsRawFd;

mod input_parser;

pub struct TerminalInterface<'a> {
    tty_file: std::fs::File,
    original_mode: Option<libc::termios>,
    request_handler: RequestHandler<'a>,
    rime_character_to_key_name_map: HashMap<char, &'static str>,
    rime_key_name_to_key_code_map: HashMap<&'static str, usize>,
}

type Result<T> = std::result::Result<T, crate::Error<std::io::Error>>;

impl From<std::io::Error> for crate::Error<std::io::Error> {
    fn from(source: std::io::Error) -> Self {
        Self::External(source)
    }
}

impl<'a> TerminalInterface<'a> {
    pub fn new(request_handler: RequestHandler<'a>) -> Result<Self> {
        Ok(Self {
            tty_file: std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .map_err(|io_err| match io_err.kind() {
                    std::io::ErrorKind::NotFound => crate::Error::NotATerminal,
                    _ => crate::Error::External(io_err),
                })?,
            original_mode: None,
            request_handler,
            rime_key_name_to_key_code_map: rime_key_name_to_key_code_map(),
            rime_character_to_key_name_map: rime_character_to_key_name_map(),
        })
    }

    pub fn enter_raw_mode(&mut self) -> Result<()> {
        let mut raw = unsafe { std::mem::zeroed() };
        unsafe {
            libc::cfmakeraw(&mut raw);
        }
        let mut original = unsafe { std::mem::zeroed() };
        if -1 == unsafe { libc::tcgetattr(self.tty_file.as_raw_fd(), &mut original) } {
            return Err(crate::Error::External(std::io::Error::last_os_error()));
        }
        self.original_mode = Some(original);
        if -1 == unsafe { libc::tcsetattr(self.tty_file.as_raw_fd(), libc::TCSADRAIN, &raw) } {
            return Err(crate::Error::External(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    pub fn open(&mut self) -> Result<()> {
        self.enter_raw_mode()?;
        self.tty_file.write(b"> ")?;
        self.tty_file.flush()?;
        Ok(())
    }

    pub fn exit_raw_mode(&mut self) -> Result<()> {
        if -1
            == unsafe {
                libc::tcsetattr(
                    self.tty_file.as_raw_fd(),
                    libc::TCSADRAIN,
                    &self.original_mode.take().unwrap(),
                )
            }
        {
            return Err(crate::Error::External(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    pub fn handle_character(&self, character: char) -> Response {
        match self.rime_character_to_key_name_map.get(&character) {
            Some(key_name) => self.request_handler.handle_request(Request::ProcessKey {
                keycode: self
                    .rime_key_name_to_key_code_map
                    .get(key_name)
                    .copied()
                    .unwrap(),
                mask: 0,
            }),
            None => Response::CharactorNotSupported(character),
        }
    }
}
