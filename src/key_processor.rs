use crate::rime_api::RimeSession;
use crate::Effect;

pub struct KeyProcessor;

impl KeyProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn process_key(&self, rime_session: &RimeSession, keycode: usize, mask: usize) -> Effect {
        let key_processed = rime_session.process_key(keycode, mask);
        match (key_processed, rime_session.get_commit().text) {
            (true, Some(commit_string)) => Effect::CommitString(commit_string),
            (false, commit_string) => Effect::RawKeyEvent {
                keycode,
                mask,
                accompanying_commit_string: commit_string,
            },
            _ => {
                let context = rime_session.get_context();
                Effect::UpdateUi {
                    composition: context.composition,
                    menu: context.menu,
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::key_processor::{Effect, KeyProcessor};
    use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};
    use std::sync::{Arc, Mutex};

    #[test]
    #[ignore = "not thread safe"]
    fn get_commit() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(Arc::new(Mutex::new(rime_api)));
        let key_processor = KeyProcessor::new();
        let report = key_processor.process_key(&rime_session, 109 /* m */, 0);
        assert_eq!(
            match report {
                Effect::UpdateUi { composition, menu } =>
                    (composition.preedit, menu.candidates.len()),
                _ => panic!(),
            },
            ("m".into(), 5),
        );
        let report = key_processor.process_key(&rime_session, 73 /* I */, 0);
        assert_eq!(
            match report {
                Effect::UpdateUi { composition, menu } =>
                    (composition.preedit, menu.candidates.len()),
                _ => panic!(),
            },
            ("骂I".into(), 0),
        );
        let response = key_processor.process_key(&rime_session, 78 /* N */, 0);
        assert_eq!(
            match response {
                Effect::UpdateUi { composition, menu } =>
                    (composition.preedit, menu.candidates.len()),
                _ => panic!(),
            },
            ("骂IN".into(), 0),
        );
        let report = key_processor.process_key(&rime_session, 89 /* Y */, 0);
        assert_eq!(
            match report {
                Effect::UpdateUi { composition, menu } =>
                    (composition.preedit, menu.candidates.len()),
                _ => panic!(),
            },
            ("骂INY".into(), 0),
        );
        let report = key_processor.process_key(&rime_session, 32 /* space */, 0);
        assert_eq!(
            match report {
                Effect::CommitString(commit_string) => commit_string,
                _ => panic!(),
            },
            "骂INY",
        );
    }
}
