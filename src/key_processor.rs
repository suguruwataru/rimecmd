use crate::rime_api::{RimeMenu, RimeSession};

pub struct Report {
    pub commit_text: Option<String>,
    pub preview_text: String,
    pub menu: RimeMenu,
}

pub struct KeyProcessor<'a> {
    rime_session: RimeSession<'a>,
}

impl<'a> KeyProcessor<'a> {
    pub fn new(rime_session: RimeSession<'a>) -> Self {
        Self { rime_session }
    }

    pub fn process_key(&self, keycode: usize, mask: usize) -> Report {
        self.rime_session.process_key(keycode, mask);
        let context = self.rime_session.get_context();
        Report {
            commit_text: self.rime_session.get_commit().text,
            preview_text: context.commit_text_preview,
            menu: context.menu,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};

    #[test]
    #[ignore = "Test uses global object. It can only be run in one-thread mode."]
    fn request_handler_get_commit() {
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
                super::Report {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
            },
            ("骂".into(), None, 5),
        );
        let report = key_processor.process_key(73 /* I */, 0);
        assert_eq!(
            match report {
                super::Report {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
            },
            ("骂I".into(), None, 0),
        );
        let response = key_processor.process_key(78 /* N */, 0);
        assert_eq!(
            match response {
                super::Report {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
            },
            ("骂IN".into(), None, 0),
        );
        let report = key_processor.process_key(89 /* Y */, 0);
        assert_eq!(
            match report {
                super::Report {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
            },
            ("骂INY".into(), None, 0),
        );
        let report = key_processor.process_key(32 /* space */, 0);
        assert_eq!(
            match report {
                super::Report {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
            },
            ("".into(), Some("骂INY".into()), 0),
        );
    }
}
