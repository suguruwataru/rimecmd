use crate::request_handler::{Request, RequestHandler, Response};
use crate::rime_api::key_mappings::{
    rime_character_to_key_name_map, rime_key_name_to_key_code_map,
};
use std::collections::HashMap;
mod input_parser;

pub struct TerminalInterface<'a> {
    request_handler: RequestHandler<'a>,
    rime_character_to_key_name_map: HashMap<char, &'static str>,
    rime_key_name_to_key_code_map: HashMap<&'static str, usize>,
}

impl<'a> TerminalInterface<'a> {
    #[allow(dead_code)]
    pub fn new(request_handler: RequestHandler<'a>) -> Self {
        Self {
            request_handler,
            rime_key_name_to_key_code_map: rime_key_name_to_key_code_map(),
            rime_character_to_key_name_map: rime_character_to_key_name_map(),
        }
    }

    #[allow(dead_code)]
    fn handle_character(&self, character: char) -> Response {
        match self.rime_character_to_key_name_map.get(&character) {
            Some(key_name) => self.request_handler.handle_request(Request::ProcessKey {
                keycode: self
                    .rime_key_name_to_key_code_map
                    .get(key_name)
                    .copied()
                    .unwrap(),
                mask: 0,
            }),
            None => Response::CharactorNotSupported(character),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};

    #[test]
    fn handle_charactor() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        let terminal_interface =
            super::TerminalInterface::new(super::RequestHandler::new(rime_session));
        assert_eq!(
            serde_json::to_string(&terminal_interface.handle_character('m')).unwrap(),
            serde_json::to_string(&super::Response::ProcessKey {
                commit_text: None,
                preview_text: "骂".into()
            })
            .unwrap(),
        );
    }
}
