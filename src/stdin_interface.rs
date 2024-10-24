use crate::json_request_processor;
use std::io::stdin;

pub struct StdinInterface<'a> {
    json_request_processor: json_request_processor::JsonRequestProcessor<'a>,
}

impl<'a> StdinInterface<'a> {
    pub fn new(json_request_processor: json_request_processor::JsonRequestProcessor<'a>) -> Self {
        Self {
            json_request_processor,
        }
    }

    pub fn process_input(
        &self,
    ) -> Result<json_request_processor::Reply, crate::Error<serde_json::Error>> {
        let request = serde_json::from_reader(stdin())?;
        Ok(self.json_request_processor.process_request(request))
    }
}
