use crate::rime_api::{RimeMenu, RimeSession};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub enum Request {
    ProcessKey { keycode: usize, mask: usize },
    Status,
}

#[derive(Debug, Serialize)]
pub enum Response {
    ProcessKey {
        commit_text: Option<String>,
        preview_text: String,
        menu: RimeMenu,
    },
    Status {
        schema_name: String,
    },
    CharactorNotSupported(char),
    Exit,
}

pub struct RequestHandler<'a> {
    rime_session: RimeSession<'a>,
}

impl<'a> RequestHandler<'a> {
    pub fn new(rime_session: RimeSession<'a>) -> Self {
        Self { rime_session }
    }

    pub fn handle_request(&self, request: Request) -> Response {
        match request {
            Request::ProcessKey { keycode, mask } => self.handle_process_key_request(keycode, mask),
            Request::Status => self.handle_status_request(),
        }
    }

    fn handle_status_request(&self) -> Response {
        let status = self.rime_session.get_status();
        Response::Status {
            schema_name: status.schema_name,
        }
    }

    fn handle_process_key_request(&self, keycode: usize, mask: usize) -> Response {
        self.rime_session.process_key(keycode, mask);
        let context = self.rime_session.get_context();
        Response::ProcessKey {
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
    #[ignore]
    fn request_handler_get_commit() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = super::RimeSession::new(&rime_api);
        let request_handler = super::RequestHandler::new(rime_session);
        let response = request_handler.handle_process_key_request(109 /* m */, 0);
        assert_eq!(
            match response {
                crate::request_handler::Response::ProcessKey {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
                _ => unreachable!(),
            },
            ("骂".into(), None, 5),
        );
        let response = request_handler.handle_process_key_request(73 /* I */, 0);
        assert_eq!(
            match response {
                crate::request_handler::Response::ProcessKey {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
                _ => unreachable!(),
            },
            ("骂I".into(), None, 0),
        );
        let response = request_handler.handle_process_key_request(78 /* N */, 0);
        assert_eq!(
            match response {
                crate::request_handler::Response::ProcessKey {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
                _ => unreachable!(),
            },
            ("骂IN".into(), None, 0),
        );
        let response = request_handler.handle_process_key_request(89 /* Y */, 0);
        assert_eq!(
            match response {
                crate::request_handler::Response::ProcessKey {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
                _ => unreachable!(),
            },
            ("骂INY".into(), None, 0),
        );
        let response = request_handler.handle_process_key_request(32 /* space */, 0);
        assert_eq!(
            match response {
                crate::request_handler::Response::ProcessKey {
                    preview_text,
                    commit_text,
                    menu,
                } => (preview_text, commit_text, menu.page_size),
                _ => unreachable!(),
            },
            ("".into(), Some("骂INY".into()), 0),
        );
    }

    #[test]
    #[ignore]
    fn request_handler_handle_status_request() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = super::RimeSession::new(&rime_api);
        let request_handler = super::RequestHandler::new(rime_session);
        assert_eq!(
            serde_json::to_string(&request_handler.handle_request(super::Request::Status)).unwrap(),
            serde_json::to_string(&super::Response::Status {
                schema_name: "luna_pinyin".into()
            })
            .unwrap(),
        );
    }
}
