use crate::rime_api::{RimeCommit, RimeContext, RimeSession};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct KeyEventMessage {
    keycode: usize,
    mask: usize,
}

#[derive(Serialize)]
pub struct UpdateMessage {
    commit_text: Option<String>,
    preview_text: String,
}

impl UpdateMessage {
    fn new(rime_commit: RimeCommit, rime_context: RimeContext) -> Self {
        Self {
            commit_text: rime_commit.text,
            preview_text: rime_context.commit_text_preview,
        }
    }
}

pub struct EventProcessor<'a> {
    rime_session: RimeSession<'a>,
}

impl<'a> EventProcessor<'a> {
    #[allow(dead_code)]
    pub fn new(rime_session: RimeSession<'a>) -> Self {
        Self { rime_session }
    }

    #[allow(dead_code)]
    fn process_key_event(&self, key_event_message: KeyEventMessage) -> UpdateMessage {
        self.rime_session
            .process_key(key_event_message.keycode, key_event_message.mask);
        UpdateMessage::new(
            self.rime_session.get_commit(),
            self.rime_session.get_context(),
        )
    }
}

mod test {
    #[test]
    fn get_commit() {
        let rime_api = crate::rime_api::RimeApi::new(
            "./test_user_data_home",
            "./test_shared_data",
            crate::rime_api::LogLevel::OFF,
        );
        let rime_session = super::RimeSession::new(&rime_api);
        let event_processor = super::EventProcessor::new(rime_session);
        assert_eq!(
            serde_json::to_string(&event_processor.process_key_event(super::KeyEventMessage {
                keycode: 109, /* m */
                mask: 0
            }))
            .unwrap(),
            serde_json::to_string(&super::UpdateMessage {
                commit_text: None,
                preview_text: "骂".into()
            })
            .unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&event_processor.process_key_event(super::KeyEventMessage {
                keycode: 73, /* I */
                mask: 0
            }))
            .unwrap(),
            serde_json::to_string(&super::UpdateMessage {
                commit_text: None,
                preview_text: "骂I".into()
            })
            .unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&event_processor.process_key_event(super::KeyEventMessage {
                keycode: 78, /* N */
                mask: 0
            }))
            .unwrap(),
            serde_json::to_string(&super::UpdateMessage {
                commit_text: None,
                preview_text: "骂IN".into()
            })
            .unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&event_processor.process_key_event(super::KeyEventMessage {
                keycode: 89, /* Y */
                mask: 0
            }))
            .unwrap(),
            serde_json::to_string(&super::UpdateMessage {
                commit_text: None,
                preview_text: "骂INY".into()
            })
            .unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&event_processor.process_key_event(super::KeyEventMessage {
                keycode: 32, /* space */
                mask: 0
            }))
            .unwrap(),
            serde_json::to_string(&super::UpdateMessage {
                commit_text: Some("骂INY".into()),
                preview_text: "".into()
            })
            .unwrap(),
        );
    }
}
