use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::{Action, Args, Call, Error};
use std::io::{stdout, Write};

pub struct TerminalMode<'a> {
    pub args: Args,
    pub terminal_interface: TerminalInterface,
    pub rime_session: RimeSession<'a>,
}

impl<'a> TerminalMode<'a> {
    pub async fn main(mut self) -> Result<(), Error> {
        let key_processor = KeyProcessor::new();
        self.terminal_interface.open().await?;
        loop {
            let call = self.terminal_interface.next_call().await?;
            let action = match call {
                Call::ProcessKey { keycode, mask } => {
                    key_processor.process_key(&self.rime_session, keycode, mask)
                }
                Call::Stop => {
                    self.terminal_interface.close().await?;
                    break;
                }
                _ => unreachable!(),
            };
            match action {
                Action::CommitString(commit_string) => {
                    if !self.args.continue_mode {
                        self.terminal_interface.close().await?;
                        writeln!(stdout(), "{}", commit_string)?;
                        break;
                    } else {
                        self.terminal_interface.remove_ui().await?;
                        writeln!(stdout(), "{}", commit_string)?;
                        self.terminal_interface.setup_ui().await?;
                    }
                }
                Action::UpdateUi {
                    ref menu,
                    ref composition,
                } => {
                    self.terminal_interface.update_ui(composition, menu).await?;
                }
            }
        }
        Ok(())
    }
}
