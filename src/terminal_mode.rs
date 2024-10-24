use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::{Args, Call, Effect, Error};
use std::io::{stdout, Write};

pub struct TerminalMode<'a> {
    pub args: Args,
    pub terminal_interface: TerminalInterface,
    pub rime_session: RimeSession<'a>,
}

impl<'a> TerminalMode<'a> {
    pub fn main(mut self) -> Result<(), Error> {
        let key_processor = KeyProcessor::new();
        self.terminal_interface.open()?;
        loop {
            let call = self.terminal_interface.next_call()?;
            let action = match call {
                Call::ProcessKey { keycode, mask } => {
                    key_processor.process_key(&self.rime_session, keycode, mask)
                }
                Call::Stop => {
                    self.terminal_interface.close()?;
                    break;
                }
                _ => unreachable!(),
            };
            match action {
                Effect::CommitString(commit_string) => {
                    if !self.args.continue_mode {
                        self.terminal_interface.close()?;
                        writeln!(stdout(), "{}", commit_string)?;
                        break;
                    } else {
                        self.terminal_interface.remove_ui()?;
                        writeln!(stdout(), "{}", commit_string)?;
                        self.terminal_interface.setup_ui()?;
                    }
                }
                Effect::UpdateUi {
                    ref menu,
                    ref composition,
                } => {
                    self.terminal_interface.update_ui(composition, menu)?;
                }
            }
        }
        Ok(())
    }
}
