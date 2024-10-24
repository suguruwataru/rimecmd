use crate::rime_api::RimeSession;

pub struct JsonRequestProcessor<'a> {
    rime_session: RimeSession<'a>,
}

pub enum Request {
    SchemaName,
}

pub enum Reply {
    SchemaName(String),
}

impl<'a> JsonRequestProcessor<'a> {
    pub fn new(rime_session: RimeSession<'a>) -> Self {
        Self { rime_session }
    }

    pub fn process_request(&self, request: Request) -> Reply {
        match request {
            Request::SchemaName => {
                let status = self.rime_session.get_status();
                Reply::SchemaName(status.schema_name)
            }
        }
    }
}
