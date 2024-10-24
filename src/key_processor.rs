use crate::rime_api::{RimeMenu, RimeSession};

pub enum Instruction {
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

    pub fn process_key(&self, keycode: usize, mask: usize) -> Instruction {
        self.rime_session.process_key(keycode, mask);
        if let Some(commit_string) = self.rime_session.get_commit().text {
            Instruction::CommitString(commit_string)
        } else {
            let context = self.rime_session.get_context();
            Instruction::UpdateUi {
                preedit: context.composition.preedit,
                menu: context.menu,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};

    #[test]
    #[ignore = "Test uses global object. It can only be run in one-thread mode."]
    fn get_commit() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = super::RimeSession::new(&rime_api);
        let key_processor = super::KeyProcessor::new(rime_session);
        let report = key_processor.process_key(109 /* m */, 0);
        assert_eq!(
            match report {
                super::Instruction::UpdateUi { preedit, menu } => (preedit, menu.page_size),
                _ => panic!(),
            },
            ("m".into(), 5),
        );
        let report = key_processor.process_key(73 /* I */, 0);
        assert_eq!(
            match report {
                super::Instruction::UpdateUi { preedit, menu } => (preedit, menu.page_size),
                _ => panic!(),
            },
            ("骂I".into(), 0),
        );
        let response = key_processor.process_key(78 /* N */, 0);
        assert_eq!(
            match response {
                super::Instruction::UpdateUi { preedit, menu } => (preedit, menu.page_size),
                _ => panic!(),
            },
            ("骂IN".into(), 0),
        );
        let report = key_processor.process_key(89 /* Y */, 0);
        assert_eq!(
            match report {
                super::Instruction::UpdateUi { preedit, menu } => (preedit, menu.page_size),
                _ => panic!(),
            },
            ("骂INY".into(), 0),
        );
        let report = key_processor.process_key(32 /* space */, 0);
        assert_eq!(
            match report {
                super::Instruction::CommitString(commit_string) => commit_string,
                _ => panic!(),
            },
            "骂INY",
        );
    }
}
