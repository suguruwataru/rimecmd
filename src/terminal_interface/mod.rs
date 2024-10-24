use crate::key_processor::{Action, KeyProcessor};
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

type Result<T> = std::result::Result<T, crate::Error<std::io::Error>>;

impl From<std::io::Error> for crate::Error<std::io::Error> {
    fn from(source: std::io::Error) -> Self {
        Self::External(source)
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

    pub fn process_input(&mut self) -> Result<String> {
        let mut height = 0;
        self.enter_raw_mode()?;
        self.erase_line_all()?;
        self.carriage_return()?;
        self.erase_after()?;
        write!(self.tty_file, "> ")?;
        self.tty_file.flush()?;
        loop {
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
                Input::Etx => {
                    self.cursor_up(height)?;
                    self.carriage_return()?;
                    self.erase_after()?;
                    self.exit_raw_mode()?;
                    break Ok("".into());
                }
                input => {
                    let Some(input_translator::RimeKey { keycode, mask }) =
                        self.input_translator.translate_input(input)
                    else {
                        unimplemented!()
                    };
                    match self.key_processor.process_key(keycode, mask) {
                        Action::UpdateUi { preedit, menu } => {
                            self.cursor_up(height)?;
                            height = menu.page_size;
                            self.carriage_return()?;
                            for (index, candidate) in
                                menu.candidates.iter().take(menu.page_size).enumerate()
                            {
                                write!(self.tty_file, "{}. {}", index + 1, candidate.text)?;
                                self.erase_line_to_right()?;
                                self.tty_file.write(b"\r\n")?;
                            }
                            write!(self.tty_file, "> {}", preedit)?;
                            self.erase_after()?;
                            self.tty_file.flush()?;
                        }
                        Action::CommitString(commit_string) => {
                            self.cursor_up(height)?;
                            self.carriage_return()?;
                            self.erase_after()?;
                            self.exit_raw_mode()?;
                            break Ok(commit_string);
                        }
                    }
                }
            }
        }
    }

    fn enter_raw_mode(&mut self) -> Result<()> {
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

    fn carriage_return(&mut self) -> Result<()> {
        self.tty_file.write(b"\r")?;
        Ok(())
    }

    fn erase_line_all(&mut self) -> Result<()> {
        self.tty_file.write(b"\x1b[2K")?;
        Ok(())
    }

    fn erase_line_to_right(&mut self) -> Result<()> {
        self.tty_file.write(b"\x1b[0K")?;
        Ok(())
    }

    fn erase_after(&mut self) -> Result<()> {
        self.tty_file.write(b"\x1b[0J")?;
        Ok(())
    }

    fn cursor_up(&mut self, times: usize) -> Result<()> {
        // Cursor up view.height times
        // Only positive integers are accepted, at least in alacritty,
        // so a if is required here.
        if times != 0 {
            write!(self.tty_file, "\x1b[{}A", times)?;
        }
        Ok(())
    }

    fn exit_raw_mode(&mut self) -> Result<()> {
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
