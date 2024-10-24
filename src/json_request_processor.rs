use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use crate::{Call, Effect};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Request {
    pub id: String,
    pub call: Call,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Reply {
    /// `None` when the reply is caused by terminal interaction or error
    /// erroneous request. Such request might include a valid `id` field,
    /// but for simplicity of implementation, even in such cases the reply
    /// has `id` `None`.
    ///
    /// Otherwise this is always the same as the id of the request this
    /// reply is for.
    pub id: Option<String>,
    pub outcome: Outcome,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    SchemaName(String),
    Effect(Effect),
    Error { code: usize, message: String },
}

impl TryFrom<crate::Error> for Outcome {
    type Error = crate::Error;

    fn try_from(error: crate::Error) -> std::result::Result<Self, crate::Error> {
        use crate::Error::*;
        match error {
            UnsupportedInput => Ok(Outcome::Error {
                code: 22,
                message: "received unsupported input".into(),
            }),
            Json(json_error) => Ok(Outcome::Error {
                code: 24,
                message: format!("{}", json_error),
            }),
            Io(io_error) => Ok(Outcome::Error {
                code: 25,
                message: format!("{}", io_error),
            }),
            err => Err(err),
        }
    }
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
                    outcome: Outcome::SchemaName(status.schema_name),
                }
            }
            Call::ProcessKey { keycode, mask } => Reply {
                id: Some(id),
                outcome: Outcome::Effect(self.key_processor.process_key(
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
            r#"{"id":"22","outcome":{"schema_name":"luna_pinyin"}}"#
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
            r#"{"id":"22","outcome":{"effect":{"update_ui":{"composition":{"length":18,"cursor_pos":0,"sel_start":0,"sel_end":0,"preedit":"〔方案選單〕"},"menu":{"candidates":[{"text":"朙月拼音","comment":null},{"text":"中／半／漢／。","comment":null},{"text":"朙月拼音·简化字","comment":null},{"text":"朙月拼音·語句流","comment":null},{"text":"bopomofo","comment":null}],"page_no":0,"highlighted_candidate_index":0,"is_last_page":false}}}}}"#
        );
    }
}
