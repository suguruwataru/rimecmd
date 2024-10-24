use crate::rime_api::RimeSession;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JsonRequestProcessor<'a> {
    rime_session: RimeSession<'a>,
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
}

impl<'a> JsonRequestProcessor<'a> {
    pub fn new(rime_session: RimeSession<'a>) -> Self {
        Self { rime_session }
    }

    pub fn process_request(&self, Request { id, call: method }: Request) -> Reply {
        match method {
            Call::SchemaName => {
                let status = self.rime_session.get_status();
                Reply {
                    id,
                    result: Result::SchemaName(status.schema_name),
                }
            }
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
        let json_request_processor = JsonRequestProcessor::new(rime_session);
        let schema_reply = json_request_processor.process_request(
            serde_json::from_str(r#"{"id":"22","call":{"method":"schema_name"}}"#).unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&schema_reply).unwrap(),
            r#"{"id":"22","result":"luna_pinyin"}"#
        );
    }
}
