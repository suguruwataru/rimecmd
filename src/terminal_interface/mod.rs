/// This module includes that code that interacts with a text terminal
///
/// This module interacts with a terminal using console_codes(4). In
/// addition, it supports function keys via XTerm codes, as documented on
/// https://invisible-island.net/xterm/ctlseqs/ctlseqs.html.
///
/// Though console_codes(4) says that one should not directly parse/write
/// console codes, this is exactly what this module does. The alternative
/// method suggested by the man page, using `terminfo`, is not really
/// practically today. Rust lacks support for `terminfo`. `terminfo`
/// itself is huge and highly complicated, hard to learn and easy to get
/// wrong with. Also, with today's terminals, which generally support a
/// similar set of codes, and the limited terminal functions this program
/// uses, `terminfo` hardly makes a difference.
use crate::key_processor::{Action, KeyProcessor};
use crate::rime_api::RimeSession;
use std::io::{Read, Write};
use std::iter::Iterator;
use std::num::NonZeroUsize;
use std::ops::ControlFlow::{Break, Continue};
use std::os::fd::AsRawFd;

mod input_parser;
mod input_translator;

enum CharacterAttribute {
    Normal,
    Faint,
}

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
    Eot,
    Bs,
    Ht,
    Lf,
    CursorPositionReport {
        row: NonZeroUsize,
        col: NonZeroUsize,
    },
}

pub struct TerminalInterface {
    tty_file: std::fs::File,
    original_mode: Option<libc::termios>,
    key_processor: KeyProcessor,
    input_translator: input_translator::InputTranslator,
}

type Result<T> = std::result::Result<T, crate::Error<std::io::Error>>;

impl TerminalInterface {
    pub fn new(key_processor: KeyProcessor) -> Result<Self> {
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

    fn set_character_attribute(&mut self, character_attribute: CharacterAttribute) -> Result<()> {
        match character_attribute {
            CharacterAttribute::Faint => self.tty_file.write(b"\x1b[2m")?,
            CharacterAttribute::Normal => self.tty_file.write(b"\x1b[0m")?,
        };
        Ok(())
    }

    /// Draw the Rime menu.
    ///
    /// When called, the cursor must be placed where the topleft cell of the menu
    /// is to be. The function doesn't do anything special if the number of lines
    /// below (including) the cell is not enough to contain the menu. It expects
    /// the terminal to automatically scroll so that enough lines will emerge from
    /// the bottom of the terminal to contain everything.
    ///
    /// This method does not flush the output.
    fn draw_menu(&mut self, menu: crate::rime_api::RimeMenu) -> Result<()> {
        for (index, candidate) in menu.candidates.iter().enumerate() {
            self.tty_file.write(b"\r\n")?;
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
                self.set_character_attribute(CharacterAttribute::Faint)?;
                write!(self.tty_file, " {}", comment)?;
                self.set_character_attribute(CharacterAttribute::Normal)?;
            }
            self.erase_line_to_right()?;
        }
        self.erase_after()?;
        Ok(())
    }

    /// Draw the composition, which is what Rime calls the part of UI that includes the edittable
    /// text.
    ///
    /// On success, return wherever Rime considers the cursor position is.
    fn draw_composition(
        &mut self,
        composition: crate::rime_api::RimeComposition,
    ) -> Result<(NonZeroUsize, NonZeroUsize)> {
        self.tty_file.write(b"> ")?;
        self.set_character_attribute(CharacterAttribute::Faint)?;
        let mut final_cursor_position = None;
        for (index, byte) in composition.preedit.as_bytes().iter().enumerate() {
            if index == composition.sel_start {
                self.set_character_attribute(CharacterAttribute::Normal)?;
            }
            if index == composition.sel_end {
                self.set_character_attribute(CharacterAttribute::Faint)?;
            }
            if index == composition.cursor_pos {
                final_cursor_position = Some(self.get_cursor_position()?)
            }
            self.tty_file.write(&[*byte])?;
        }
        self.set_character_attribute(CharacterAttribute::Normal)?;
        Ok(final_cursor_position.unwrap_or(self.get_cursor_position()?))
    }

    fn get_cursor_position(&mut self) -> Result<(NonZeroUsize, NonZeroUsize)> {
        self.tty_file.write(b"\x1b[6n")?;
        let Input::CursorPositionReport { row, col } = self.read_input()? else {
            return Err(crate::Error::UnsupportedInput);
        };
        Ok((row, col))
    }

    fn set_cursor_position(&mut self, (row, col): (NonZeroUsize, NonZeroUsize)) -> Result<()> {
        write!(self.tty_file, "\x1b[{};{}H", row, col)?;
        Ok(())
    }

    pub fn process_input(&mut self, rime_session: &RimeSession) -> Result<Option<String>> {
        self.enter_raw_mode()?;
        self.erase_line_all()?;
        self.carriage_return()?;
        self.erase_after()?;
        write!(self.tty_file, "> ")?;
        self.tty_file.flush()?;
        loop {
            match self.read_input()? {
                Input::Etx | Input::Eot => {
                    self.carriage_return()?;
                    self.erase_after()?;
                    self.exit_raw_mode()?;
                    break Ok(None);
                }
                input => {
                    let Some(input_translator::RimeKey { keycode, mask }) =
                        self.input_translator.translate_input(input)
                    else {
                        break Err(crate::Error::UnsupportedInput);
                    };
                    match self.key_processor.process_key(rime_session, keycode, mask) {
                        Action::UpdateUi { composition, menu } => {
                            self.carriage_return()?;
                            let final_cursor_pos = self.draw_composition(composition)?;
                            self.draw_menu(menu)?;
                            self.set_cursor_position(final_cursor_pos)?;
                            self.tty_file.flush()?;
                        }
                        Action::CommitString(commit_string) => {
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
