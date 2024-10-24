use crate::json_request_processor;
use crate::rime_api::RimeSession;
use std::io::stdin;

pub struct StdinInterface {
    json_request_processor: json_request_processor::JsonRequestProcessor,
}

impl StdinInterface {
    pub fn new(json_request_processor: json_request_processor::JsonRequestProcessor) -> Self {
        Self {
            json_request_processor,
        }
    }

    pub fn process_input(
        &self,
        rime_session: &RimeSession,
    ) -> Result<json_request_processor::Reply, crate::Error<serde_json::Error>> {
        let request = serde_json::from_reader(stdin())?;
        Ok(self
            .json_request_processor
            .process_request(rime_session, request))
    }
}
