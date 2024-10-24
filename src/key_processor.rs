use crate::rime_api::{RimeMenu, RimeSession};

pub enum Action {
    CommitString(String),
    UpdateUi { preedit: String, menu: RimeMenu },
}

pub struct KeyProcessor<'a> {
    rime_session: RimeSession<'a>,
}

impl<'a> KeyProcessor<'a> {
    pub fn new(rime_session: RimeSession<'a>) -> Self {
        Self { rime_session }
    }

    pub fn process_key(&self, keycode: usize, mask: usize) -> Action {
        self.rime_session.process_key(keycode, mask);
        if let Some(commit_string) = self.rime_session.get_commit().text {
            Action::CommitString(commit_string)
        } else {
            let context = self.rime_session.get_context();
            Action::UpdateUi {
                preedit: context.composition.preedit,
                menu: context.menu,
            }
        }
    }
}

#[cfg(test)]
mod test {}
