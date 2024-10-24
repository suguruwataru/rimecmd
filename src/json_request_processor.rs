use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JsonRequestProcessor {
    key_processor: KeyProcessor,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Request {
    pub id: String,
    pub call: Call,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "snake_case",
    tag = "method",
    content = "params",
    deny_unknown_fields
)]
pub enum Call {
    SchemaName,
    ProcessKey { keycode: usize, mask: usize },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Reply {
    pub id: String,
    pub result: Result,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", untagged, deny_unknown_fields)]
pub enum Result {
    SchemaName(String),
    Action(crate::key_processor::Action),
}

impl JsonRequestProcessor {
    pub fn new() -> Self {
        Self {
            key_processor: crate::key_processor::KeyProcessor::new(),
        }
    }

    pub fn process_request(
        &self,
        rime_session: &RimeSession,
        Request { id, call: method }: Request,
    ) -> Reply {
        match method {
            Call::SchemaName => {
                let status = rime_session.get_status();
                Reply {
                    id,
                    result: Result::SchemaName(status.schema_name),
                }
            }
            Call::ProcessKey { keycode, mask } => Reply {
                id,
                result: Result::Action(self.key_processor.process_key(rime_session, keycode, mask)),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore = "not thread safe"]
    fn schema_name() {
        let rime_api = crate::rime_api::RimeApi::new(
            crate::testing_utilities::temporary_directory_path(),
            "./test_shared_data",
            crate::testing_utilities::LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        let json_request_processor = JsonRequestProcessor::new();
        let schema_reply = json_request_processor.process_request(
            &rime_session,
            serde_json::from_str(r#"{"id":"22","call":{"method":"schema_name"}}"#).unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&schema_reply).unwrap(),
            r#"{"id":"22","result":"luna_pinyin"}"#
        );
    }

    #[test]
    #[ignore = "not thread safe"]
    fn process_key() {
        let rime_api = crate::rime_api::RimeApi::new(
            crate::testing_utilities::temporary_directory_path(),
            "./test_shared_data",
            crate::testing_utilities::LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        let json_request_processor = JsonRequestProcessor::new();
        let schema_reply = json_request_processor.process_request(
            &rime_session,
            serde_json::from_str(
                // Ctrl-`
                r#"{
                    "id": "22",
                    "call": {
                        "method": "process_key",
                        "params": {
                            "keycode": 96,
                            "mask": 4
                        }
                    }
                }"#,
            )
            .unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&schema_reply).unwrap(),
            r#"{"id":"22","result":{"action":"update_ui","params":{"composition":{"length":18,"cursor_pos":0,"sel_start":0,"sel_end":0,"preedit":"〔方案選單〕"},"menu":{"candidates":[{"text":"朙月拼音","comment":null},{"text":"中／半／漢／。","comment":null},{"text":"朙月拼音·简化字","comment":null},{"text":"朙月拼音·語句流","comment":null},{"text":"bopomofo","comment":null}],"page_no":0,"highlighted_candidate_index":0,"is_last_page":false}}}}"#
        );
    }
}
