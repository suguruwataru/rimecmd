use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};

#[test]
#[ignore = "Test uses global object. It can only be run in one-thread mode."]
fn get_context() {
    let rime_api =
        crate::rime_api::RimeApi::new(temporary_directory_path(), "./test_shared_data", LOG_LEVEL);
    let rime_session = crate::rime_api::RimeSession::new(&rime_api);
    rime_session.process_key(109 /* m */, 0);
    assert_eq!("m", rime_session.get_context().composition.preedit);
}

#[test]
#[ignore = "Test uses global object. It can only be run in one-thread mode."]
fn process_return() {
    let rime_api =
        crate::rime_api::RimeApi::new(temporary_directory_path(), "./test_shared_data", LOG_LEVEL);
    let rime_session = crate::rime_api::RimeSession::new(&rime_api);
    rime_session.process_key(109 /* m */, 0);
    rime_session.process_key(110 /* n */, 0);
    rime_session.process_key(111 /* o */, 0);
    rime_session.process_key(0xff0d /* Return */, 0);
    assert_eq!("mno", rime_session.get_commit().text.unwrap());
}

#[test]
#[ignore = "Test uses global object. It can only be run in one-thread mode."]
fn process_backspace() {
    let rime_api =
        crate::rime_api::RimeApi::new(temporary_directory_path(), "./test_shared_data", LOG_LEVEL);
    let rime_session = crate::rime_api::RimeSession::new(&rime_api);
    rime_session.process_key(109 /* m */, 0);
    rime_session.process_key(105 /* i */, 0);
    assert_eq!("mi", rime_session.get_context().composition.preedit);
    rime_session.process_key(0xff08 /* Backspace */, 0);
    assert_eq!("m", rime_session.get_context().composition.preedit);
}
