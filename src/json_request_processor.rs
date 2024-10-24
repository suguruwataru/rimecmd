use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Request {
    pub id: String,
    pub call: Call,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "snake_case",
    tag = "method",
    content = "params",
    deny_unknown_fields
)]
pub enum Call {
    SchemaName,
    Stop,
    ProcessKey { keycode: usize, mask: usize },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Reply {
    pub id: Option<String>,
    pub result: Result,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", untagged, deny_unknown_fields)]
pub enum Result {
    SchemaName(String),
    Action(crate::key_processor::Action),
}

pub struct JsonRequestProcessor<'a> {
    pub key_processor: KeyProcessor,
    pub rime_session: &'a RimeSession<'a>,
}

impl JsonRequestProcessor<'_> {
    pub fn process_request(&self, Request { id, call: method }: Request) -> Reply {
        match method {
            Call::SchemaName => {
                let status = self.rime_session.get_status();
                Reply {
                    id: Some(id),
                    result: Result::SchemaName(status.schema_name),
                }
            }
            Call::ProcessKey { keycode, mask } => Reply {
                id: Some(id),
                result: Result::Action(self.key_processor.process_key(
                    self.rime_session,
                    keycode,
                    mask,
                )),
            },
            _ => todo!(),
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
        let json_request_processor = JsonRequestProcessor {
            key_processor: KeyProcessor::new(),
            rime_session: &rime_session,
        };
        let schema_reply = json_request_processor.process_request(
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
        let json_request_processor = JsonRequestProcessor {
            key_processor: KeyProcessor::new(),
            rime_session: &rime_session,
        };
        let schema_reply = json_request_processor.process_request(
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
