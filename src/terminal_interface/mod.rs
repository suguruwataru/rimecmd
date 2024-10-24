use crate::key_processor::{Action, KeyProcessor};
use std::io::{Read, Write};
use std::iter::Iterator;
use std::ops::ControlFlow::{Break, Continue};
use std::os::fd::AsRawFd;

mod input_parser;
mod input_translator;

pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    KeypadHome,
    Insert,
    Delete,
    KeypadEnd,
    PageUp,
    PageDown,
    Char(char),
    Cr,
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

    fn read_input(&mut self) -> Result<Input> {
        let Break(result_input) = std::io::Read::by_ref(&mut self.tty_file).bytes().try_fold(
            Ok(input_parser::ParserState::new()),
            |parser_state: Result<input_parser::ParserState>, byte| {
                let Ok(byte) = byte else {
                    return Break(Err(byte.unwrap_err()));
                };
                match parser_state.unwrap().consume_byte(byte) {
                    input_parser::ConsumeByteResult::Pending(state) => Continue(Ok(state)),
                    input_parser::ConsumeByteResult::Completed(input) => Break(Ok(input)),
                }
            },
        ) else {
            unreachable!()
        };
        Ok(result_input?)
    }

    /// Draw the Rime menu.
    ///
    /// When called, the cursor must be placed where the topleft cell of the menu
    /// is to be. The function doesn't do anything special if the number of lines
    /// below (including) the cell is not enough to contain the menu. It expects
    /// the terminal to automatically scroll so that enough lines will emerge from
    /// the bottom of the terminal to contain everything.
    ///
    /// On success, the height of the drawn menu will be returned. The cursor will
    /// be placed at the end of the last line.
    fn draw_menu(&mut self, menu: crate::rime_api::RimeMenu) -> Result<usize> {
        let mut height = 0;
        for (index, candidate) in menu
            .candidates
            .iter()
            .skip(menu.page_size * menu.page_no)
            .take(menu.page_size)
            .enumerate()
        {
            if index == menu.highlighted_candidate_index {
                // The escape code here gives the index inverted color,
                write!(
                    self.tty_file,
                    "\x1b[7m{}.\x1b[0m {}",
                    index + 1,
                    candidate.text
                )?;
            } else {
                write!(self.tty_file, "{}. {}", index + 1, candidate.text)?;
            }
            if let Some(comment) = candidate.comment.as_ref() {
                // The escape code here gives the comment faint color,
                write!(self.tty_file, " \x1b[2m{}\x1b[0m", comment)?;
            }
            self.erase_line_to_right()?;
            self.tty_file.write(b"\r\n")?;
            height = index + 1;
        }
        Ok(height)
    }

    pub fn process_input(&mut self) -> Result<Option<String>> {
        let mut height = 0;
        self.enter_raw_mode()?;
        self.erase_line_all()?;
        self.carriage_return()?;
        self.erase_after()?;
        write!(self.tty_file, "> ")?;
        self.tty_file.flush()?;
        loop {
            match self.read_input()? {
                Input::Etx => {
                    self.cursor_up(height)?;
                    self.carriage_return()?;
                    self.erase_after()?;
                    self.exit_raw_mode()?;
                    break Ok(None);
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
                            self.carriage_return()?;
                            height = self.draw_menu(menu)?;
                            write!(self.tty_file, "> {}", preedit)?;
                            self.erase_after()?;
                            self.tty_file.flush()?;
                        }
                        Action::CommitString(commit_string) => {
                            self.cursor_up(height)?;
                            self.carriage_return()?;
                            self.erase_after()?;
                            self.exit_raw_mode()?;
                            break Ok(Some(commit_string));
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
