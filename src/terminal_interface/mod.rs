use crate::key_processor::{KeyProcessor, Report};
use std::io::{Read, Write};
use std::iter::Iterator;
use std::os::fd::AsRawFd;

mod input_parser;

mod input_translator;
#[cfg(test)]
mod tests;

#[allow(dead_code)]
pub enum Input {
    Char(char),
    Del,
    Nul,
    Etx,
}

pub struct TerminalInterface<'a> {
    tty_file: std::fs::File,
    original_mode: Option<libc::termios>,
    key_processor: KeyProcessor<'a>,
    input_translator: input_translator::InputTranslator,
}

pub enum Action {
    Update(Report),
    Exit,
}

type Result<T> = std::result::Result<T, crate::Error<std::io::Error>>;

impl From<std::io::Error> for crate::Error<std::io::Error> {
    fn from(source: std::io::Error) -> Self {
        Self::External(source)
    }
}

impl<'a> Write for TerminalInterface<'a> {
    fn flush(&mut self) -> std::io::Result<()> {
        self.tty_file.flush()
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tty_file.write(buf)
    }
}

impl<'a> TerminalInterface<'a> {
    pub fn new(key_processor: KeyProcessor<'a>) -> Result<Self> {
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
            key_processor,
            input_translator: input_translator::InputTranslator::new(),
        })
    }

    pub fn next_response(&mut self) -> Option<Action> {
        let std::ops::ControlFlow::Break(input) =
            std::io::Read::by_ref(&mut self.tty_file).bytes().try_fold(
                input_parser::ParserState::new(),
                |parser_state, maybe_byte| {
                    let byte = maybe_byte.unwrap();
                    match parser_state.consume_byte(byte) {
                        input_parser::ConsumeByteResult::Pending(state) => {
                            std::ops::ControlFlow::Continue(state)
                        }
                        input_parser::ConsumeByteResult::Completed(input) => {
                            std::ops::ControlFlow::Break(input)
                        }
                    }
                },
            )
        else {
            unreachable!()
        };
        match input {
            Input::Etx => Some(Action::Exit),
            input => {
                let Some(input_translator::RimeKey { keycode, mask }) =
                    self.input_translator.translate_input(input)
                else {
                    unimplemented!()
                };
                Some(Action::Update(
                    self.key_processor.process_key(keycode, mask),
                ))
            }
        }
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

    pub fn carriage_return(&mut self) -> Result<()> {
        self.tty_file.write(b"\r")?;
        Ok(())
    }

    pub fn erase_all_line(&mut self) -> Result<()> {
        self.tty_file.write(b"\x1b[2K")?;
        Ok(())
    }

    pub fn erase_line_to_right(&mut self) -> Result<()> {
        self.tty_file.write(b"\x1b[0K")?;
        Ok(())
    }

    pub fn erase_after(&mut self) -> Result<()> {
        self.tty_file.write(b"\x1b[0J")?;
        Ok(())
    }

    pub fn cursor_up(&mut self, times: usize) -> Result<()> {
        // Cursor up view.height times
        // Only positive integers are accepted, at least in alacritty,
        // so a if is required here.
        if times != 0 {
            write!(self.tty_file, "\x1b[{}A", times)?;
        }
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
}
