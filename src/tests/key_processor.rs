use crate::key_processor::{Action, KeyProcessor};
use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};

#[test]
#[ignore = "Test uses global object. It can only be run in one-thread mode."]
fn get_commit() {
    let rime_api =
        crate::rime_api::RimeApi::new(temporary_directory_path(), "./test_shared_data", LOG_LEVEL);
    let rime_session = crate::rime_api::RimeSession::new(&rime_api);
    let key_processor = KeyProcessor::new(rime_session);
    let report = key_processor.process_key(109 /* m */, 0);
    assert_eq!(
        match report {
            Action::UpdateUi { preedit, menu } => (preedit, menu.page_size),
            _ => panic!(),
        },
        ("m".into(), 5),
    );
    let report = key_processor.process_key(73 /* I */, 0);
    assert_eq!(
        match report {
            Action::UpdateUi { preedit, menu } => (preedit, menu.page_size),
            _ => panic!(),
        },
        ("骂I".into(), 0),
    );
    let response = key_processor.process_key(78 /* N */, 0);
    assert_eq!(
        match response {
            Action::UpdateUi { preedit, menu } => (preedit, menu.page_size),
            _ => panic!(),
        },
        ("骂IN".into(), 0),
    );
    let report = key_processor.process_key(89 /* Y */, 0);
    assert_eq!(
        match report {
            Action::UpdateUi { preedit, menu } => (preedit, menu.page_size),
            _ => panic!(),
        },
        ("骂INY".into(), 0),
    );
    let report = key_processor.process_key(32 /* space */, 0);
    assert_eq!(
        match report {
            Action::CommitString(commit_string) => commit_string,
            _ => panic!(),
        },
        "骂INY",
    );
}
