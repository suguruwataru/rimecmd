use crate::json_request_processor::Request;
use crate::poll_data::{PollData, ReadData};
use crate::rime_api::{RimeComposition, RimeMenu};
use crate::terminal_json_mode::Data;
use crate::Call;
use std::io::{Read, Write};
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
use std::iter::Iterator;
use std::num::NonZeroUsize;
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
    input_translator: input_translator::InputTranslator,
    input_buffer: Vec<Input>,
}

type Result<T> = std::result::Result<T, crate::Error>;

impl ReadData<Request> for TerminalInterface {
    fn register(&self, poll_request: &mut PollData<Request>) -> Result<()> {
        poll_request.register(&self.tty_file.as_raw_fd())
    }

    fn read_data(&mut self) -> Result<Request> {
        let call = self.next_call()?;
        Ok(Request {
            id: uuid::Uuid::new_v4().into(),
            call,
        })
    }
}

impl ReadData<Data> for TerminalInterface {
    fn read_data(&mut self) -> Result<Data> {
        Ok(Data::TerminalRequest(ReadData::<Request>::read_data(self)?))
    }
    fn register(&self, poll_request: &mut PollData<Data>) -> Result<()> {
        poll_request.register(&self.tty_file.as_raw_fd())
    }
}

impl TerminalInterface {
    pub fn new() -> Result<Self> {
        Ok(Self {
            input_buffer: vec![],
            tty_file: std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .map_err(|io_err| match io_err.kind() {
                    std::io::ErrorKind::NotFound => crate::Error::NotATerminal,
                    _ => crate::Error::Io(io_err),
                })?,
            original_mode: None,
            input_translator: input_translator::InputTranslator::new(),
        })
    }

    fn read_input(&mut self) -> Result<Input> {
        let mut buf = [0u8; 1];
        let mut input_parser_state = input_parser::ParserState::new();
        loop {
            self.tty_file.read(&mut buf)?;
            let byte = buf[0];
            match input_parser_state.consume_byte(byte) {
                input_parser::ConsumeByteResult::Pending(new_state) => {
                    input_parser_state = new_state
                }
                input_parser::ConsumeByteResult::Completed(input) => break Ok(input),
            }
        }
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
    /// is to be. The function doesn't do anything special if the space is not enough
    /// to contain the menu. It expects the terminal to automatically scroll so that
    /// enough lines will emerge from the bottom of the terminal to contain everything.
    ///
    /// This method does not flush the output.
    ///
    /// On success, return the row to place the cursor, using 1-index.
    fn draw_menu(&mut self, menu: &crate::rime_api::RimeMenu) -> Result<NonZeroUsize> {
        let mut height = 0;
        for (index, candidate) in menu.candidates.iter().enumerate() {
            self.tty_file.write(b"\r\n")?;
            if index == menu.highlighted_candidate_index {
                // The escape code here gives the index inverted color,
                self.tty_file
                    .write(format!("\x1b[7m{}.\x1b[0m {}", index + 1, candidate.text).as_bytes())?;
            } else {
                self.tty_file
                    .write(format!("{}. {}", index + 1, candidate.text).as_bytes())?;
            }
            if let Some(comment) = candidate.comment.as_ref() {
                self.set_character_attribute(CharacterAttribute::Faint)?;
                self.tty_file.write(format!(" {}", comment).as_bytes())?;
                self.set_character_attribute(CharacterAttribute::Normal)?;
            }
            self.erase_line_to_right()?;
            height = index + 1;
        }
        self.erase_after()?;
        let last_line_row = self.get_cursor_position()?.0;
        Ok((last_line_row.get() - height)
            .try_into()
            .unwrap_or(NonZeroUsize::new(1).unwrap()))
    }

    /// Draw the composition, which is what Rime calls the part of UI that includes the edittable
    /// text.
    ///
    /// On success, return the column to place the cursor, using 1-index, based on wherever Rime
    /// considers the cursor position is.
    fn draw_composition(
        &mut self,
        composition: &crate::rime_api::RimeComposition,
    ) -> Result<NonZeroUsize> {
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
        self.erase_line_to_right()?;
        Ok(if let Some(final_cursor_position) = final_cursor_position {
            final_cursor_position.1
        } else {
            self.get_cursor_position()?.1
        })
    }

    fn get_cursor_position(&mut self) -> Result<(NonZeroUsize, NonZeroUsize)> {
        self.tty_file.write(b"\x1b[6n")?;
        loop {
            let input = self.read_input()?;
            if let Input::CursorPositionReport { row, col } = input {
                break Ok((row, col));
            } else {
                self.input_buffer.push(input);
            }
        }
    }

    fn set_cursor_position(&mut self, (row, col): (NonZeroUsize, NonZeroUsize)) -> Result<()> {
        self.tty_file
            .write(format!("\x1b[{};{}H", row, col).as_bytes())?;
        Ok(())
    }

    pub fn open(&mut self) -> Result<()> {
        self.enter_raw_mode()?;
        self.setup_ui()?;
        Ok(())
    }

    pub fn next_call(&mut self) -> Result<Call> {
        let input = match self.input_buffer.pop() {
            Some(input) => input,
            None => self.read_input()?,
        };
        match input {
            Input::Etx | Input::Eot => Ok(Call::StopClient),
            input => {
                let Some(input_translator::RimeKey { keycode, mask }) =
                    self.input_translator.translate_input(input)
                else {
                    return Err(crate::Error::UnsupportedInput);
                };
                Ok(Call::ProcessKey { keycode, mask })
            }
        }
    }

    pub fn update_ui(&mut self, composition: &RimeComposition, menu: &RimeMenu) -> Result<()> {
        self.carriage_return()?;
        let final_cursor_col = self.draw_composition(composition)?;
        let final_cursor_row = self.draw_menu(menu)?;
        self.set_cursor_position((final_cursor_row, final_cursor_col))?;
        self.tty_file.flush()?;
        Ok(())
    }

    pub fn setup_ui(&mut self) -> Result<()> {
        self.carriage_return()?;
        self.tty_file.write(b"> ")?;
        self.erase_after()?;
        self.tty_file.flush()?;
        Ok(())
    }

    pub fn remove_ui(&mut self) -> Result<()> {
        self.carriage_return()?;
        self.erase_after()?;
        self.tty_file.flush()?;
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        self.remove_ui()?;
        self.exit_raw_mode()?;
        Ok(())
    }

    fn enter_raw_mode(&mut self) -> Result<()> {
        let mut raw = unsafe { std::mem::zeroed() };
        unsafe {
            libc::cfmakeraw(&mut raw);
        }
        let mut original = unsafe { std::mem::zeroed() };
        if -1 == unsafe { libc::tcgetattr(self.tty_file.as_raw_fd(), &mut original) } {
            return Err(crate::Error::Io(std::io::Error::last_os_error()));
        }
        self.original_mode = Some(original);
        if -1 == unsafe { libc::tcsetattr(self.tty_file.as_raw_fd(), libc::TCSADRAIN, &raw) } {
            return Err(crate::Error::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn carriage_return(&mut self) -> Result<()> {
        self.tty_file.write(b"\r")?;
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
            return Err(crate::Error::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}
