pub mod key_mappings;
use crate::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::Once;

static RIME_API_SETUP: Once = Once::new();

#[link(name = "rimecmd", kind = "static")]
extern "C" {
    fn c_get_rime_api() -> *mut CRimeApi;
    fn c_setup_rime_api_once(
        c_rime_api: *mut CRimeApi,
        user_data_dir: *const c_char,
        shared_data_dir: *const c_char,
        log_level: c_int,
    );
    fn c_initialize_rime_api(
        c_rime_api: *mut CRimeApi,
        user_data_dir: *const c_char,
        shared_data_dir: *const c_char,
        log_level: c_int,
    );
    fn c_do_maintenance(c_rime_api: *mut CRimeApi);
    fn c_destory_rime_api(rime_api: *mut CRimeApi) -> c_void;
    fn c_get_user_data_dir(rime_api: *mut CRimeApi) -> *mut std::ffi::c_char;
    fn c_get_shared_data_dir(rime_api: *mut CRimeApi) -> *mut std::ffi::c_char;
    fn c_get_schema_list(
        rime_api: *mut CRimeApi,
        schema_list: *mut CRimeSchemaList,
    ) -> std::ffi::c_int;
    fn c_free_schema_list(rime_api: *mut CRimeApi, schema_list: *mut CRimeSchemaList) -> c_void;
    fn c_create_session(rime_api: *mut CRimeApi) -> usize;
    fn c_destory_session(rime_api: *mut CRimeApi, session_id: usize) -> c_void;
    fn c_get_status(
        rime_api: *mut CRimeApi,
        session_id: usize,
        status: *mut CRimecmdRimeStatus,
    ) -> c_void;
    fn c_free_status(status: *mut CRimecmdRimeStatus) -> c_void;
    fn c_get_commit(
        rime_api: *mut CRimeApi,
        session_id: usize,
        commit: *mut CRimecmdRimeCommit,
    ) -> c_void;
    fn c_free_commit(commit: *mut CRimecmdRimeCommit) -> c_void;
    fn c_process_key(
        rime_api: *mut CRimeApi,
        session_id: usize,
        keycode: c_int,
        mask: c_int,
    ) -> c_int;
    fn c_get_context(
        rime_api: *mut CRimeApi,
        session_id: usize,
        context: *mut CRimecmdRimeContext,
    ) -> c_void;
    fn c_free_context(context: *mut CRimecmdRimeContext) -> c_void;
    fn c_get_current_schema(
        rime_api: *mut CRimeApi,
        session_id: usize,
        schema_id: *mut c_char,
        // currently, in Rust usize is conventionally used for size_t in C.
        // According to standards, this is not perfectly correct.
        // However, correct enough here.
        buffer_size: usize,
    ) -> c_int;
    fn c_candidate_list_begin(
        rime_api: *mut CRimeApi,
        session_id: usize,
        iterator: *mut CRimeCandidateListIterator,
    ) -> c_int;
    fn c_candidate_list_next(
        rime_api: *mut CRimeApi,
        iterator: *mut CRimeCandidateListIterator,
    ) -> c_int;
    fn c_candidate_list_end(
        rime_api: *mut CRimeApi,
        iterator: *mut CRimeCandidateListIterator,
    ) -> c_void;
}

#[repr(C)]
struct CRimeCandidate {
    text: *mut c_char,
    comment: *mut c_char,
    reserved: *mut c_void,
}

#[repr(C)]
struct CRimeMenu {
    page_size: c_int,
    page_no: c_int,
    is_last_page: c_int,
    highlighted_candidate_index: c_int,
    num_candidates: c_int,
    candidates: *mut CRimeCandidate,
    select_keys: *mut c_char,
}

#[repr(C)]
struct CRimeCandidateListIterator {
    ptr: *mut c_void,
    index: c_int,
    candidate: CRimeCandidate,
}

#[repr(C)]
struct CRimecmdRimeComposition {
    length: c_int,
    cursor_pos: c_int,
    sel_start: c_int,
    sel_end: c_int,
    preedit: *mut c_char,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RimeComposition {
    pub length: usize,
    pub cursor_pos: usize,
    pub sel_start: usize,
    pub sel_end: usize,
    pub preedit: String,
}

fn rime_composition_from_c(
    c_rimecmd_rime_composition: &CRimecmdRimeComposition,
) -> RimeComposition {
    RimeComposition {
        length: c_rimecmd_rime_composition.length as usize,
        cursor_pos: c_rimecmd_rime_composition.cursor_pos as usize,
        sel_start: c_rimecmd_rime_composition.sel_start as usize,
        sel_end: c_rimecmd_rime_composition.sel_end as usize,
        preedit: if c_rimecmd_rime_composition.preedit.is_null() {
            "".into()
        } else {
            unsafe { std::ffi::CStr::from_ptr(c_rimecmd_rime_composition.preedit) }
                .to_str()
                .unwrap()
                .to_owned()
        },
    }
}

#[repr(C)]
struct CRimecmdRimeContext {
    composition: CRimecmdRimeComposition,
    menu: CRimeMenu,
    commit_text_preview: *mut c_char,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RimeCandidate {
    pub text: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RimeMenu {
    pub candidates: Vec<RimeCandidate>,
    pub page_no: usize,
    pub highlighted_candidate_index: usize,
    pub is_last_page: bool,
}

fn rime_candidate_from_c(c_rime_candidate: &CRimeCandidate) -> RimeCandidate {
    RimeCandidate {
        text: unsafe { CStr::from_ptr(c_rime_candidate.text) }
            .to_owned()
            .into_string()
            .unwrap(),
        comment: (!c_rime_candidate.comment.is_null()).then(|| {
            unsafe { CStr::from_ptr(c_rime_candidate.comment) }
                .to_owned()
                .into_string()
                .unwrap()
        }),
    }
}

fn get_rime_menu(c_rime_api: *mut CRimeApi, session_id: usize, menu: &CRimeMenu) -> RimeMenu {
    let mut iterator = CRimeCandidateListIterator {
        ptr: std::ptr::null_mut(),
        index: 0,
        candidate: CRimeCandidate {
            text: std::ptr::null_mut(),
            comment: std::ptr::null_mut(),
            reserved: std::ptr::null_mut(),
        },
    };
    unsafe {
        c_candidate_list_begin(c_rime_api, session_id, &mut iterator);
    }
    RimeMenu {
        page_no: menu.page_no as usize,
        is_last_page: menu.is_last_page == 1,
        highlighted_candidate_index: menu.highlighted_candidate_index as usize,
        candidates: std::iter::from_fn(|| {
            if 1 == unsafe { c_candidate_list_next(c_rime_api, &mut iterator) } {
                Some(rime_candidate_from_c(&iterator.candidate))
            } else {
                unsafe { c_candidate_list_end(c_rime_api, &mut iterator) };
                None
            }
        })
        .skip((menu.page_size * menu.page_no) as usize)
        .take(menu.page_size as usize)
        .collect(),
    }
}

#[derive(Debug)]
pub struct RimeContext {
    pub commit_text_preview: String,
    pub composition: RimeComposition,
    pub menu: RimeMenu,
}

// The first fields make it the proper way of declaring an opaque C type as
// documented in Rustonomicon.
#[repr(C)]
struct CRimeApi {
    data: [u8; 0],
    marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[repr(C)]
struct CRimeSchemaListItem {
    schema_id: *mut std::ffi::c_char,
    name: *mut std::ffi::c_char,
    reserved: *mut std::ffi::c_void,
}

#[repr(C)]
struct CRimeSchemaList {
    // currently, in Rust usize is conventionally used for size_t in C.
    // According to standards, this is not perfectly correct.
    // However, correct enough here.
    size: usize,
    list: *mut CRimeSchemaListItem,
}

#[repr(C)]
struct CRimecmdRimeStatus {
    schema_id: *mut c_char,
    schema_name: *mut c_char,
    is_disabled: c_int,
    is_composing: c_int,
    is_ascii_mode: c_int,
    is_full_shape: c_int,
    is_simplified: c_int,
    is_traditional: c_int,
    is_ascii_punct: c_int,
}

#[repr(C)]
struct CRimecmdRimeCommit {
    text: *mut c_char,
}

pub struct RimeCommit {
    pub text: Option<String>,
}

pub struct RimeStatus {
    pub schema_id: String,
    pub schema_name: String,
    pub is_disabled: bool,
    pub is_composing: bool,
    pub is_ascii_mode: bool,
    pub is_full_shape: bool,
    pub is_simplified: bool,
    pub is_traditional: bool,
    pub is_ascii_punct: bool,
}

pub struct RimeSchema {
    schema_id: String,
    name: String,
}

fn rime_schema_from_c(c_rime_schema_item: &CRimeSchemaListItem) -> RimeSchema {
    RimeSchema {
        schema_id: unsafe { std::ffi::CStr::from_ptr(c_rime_schema_item.schema_id) }
            .to_owned()
            .into_string()
            .unwrap(),
        name: unsafe { std::ffi::CStr::from_ptr(c_rime_schema_item.schema_id) }
            .to_owned()
            .into_string()
            .unwrap(),
    }
}

fn c_string_from_path(path: &std::path::Path) -> Result<std::ffi::CString, Error> {
    path.to_str()
        .ok_or(Error::NonUtf8DataHomePath)
        .and_then(|data_home_str| {
            std::ffi::CString::new(data_home_str).map_err(|err| Error::NulInCString(err))
        })
}

pub struct RimeSession<'a> {
    api: &'a RimeApi,
    session_id: usize,
}

impl<'a> RimeSession<'a> {
    pub fn new(api: &'a RimeApi) -> Self {
        Self {
            session_id: unsafe { c_create_session(api.c_rime_api) },
            api,
        }
    }

    pub fn process_key(&self, keycode: usize, mask: usize) -> bool {
        1 == unsafe {
            c_process_key(
                self.api.c_rime_api,
                self.session_id,
                keycode.try_into().unwrap(),
                mask.try_into().unwrap(),
            )
        }
    }

    pub fn get_current_schema(&self) -> String {
        let mut buffer = [0; 1024];
        if 0 == unsafe {
            c_get_current_schema(
                self.api.c_rime_api,
                self.session_id,
                buffer.as_mut_ptr(),
                1024,
            )
        } {
            panic!();
        }
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_owned()
            .into_string()
            .unwrap()
    }

    pub fn get_context(&self) -> RimeContext {
        let mut c_context = CRimecmdRimeContext {
            commit_text_preview: std::ptr::null_mut(),
            composition: CRimecmdRimeComposition {
                sel_end: 0,
                sel_start: 0,
                length: 0,
                cursor_pos: 0,
                preedit: std::ptr::null_mut(),
            },
            menu: CRimeMenu {
                num_candidates: 0,
                page_size: 0,
                page_no: 0,
                is_last_page: 0,
                highlighted_candidate_index: 0,
                candidates: std::ptr::null_mut(),
                select_keys: std::ptr::null_mut(),
            },
        };
        unsafe {
            c_get_context(self.api.c_rime_api, self.session_id, &mut c_context);
        }
        let context = RimeContext {
            commit_text_preview: if c_context.commit_text_preview.is_null() {
                "".into()
            } else {
                unsafe { std::ffi::CStr::from_ptr(c_context.commit_text_preview) }
                    .to_str()
                    .unwrap()
                    .to_owned()
            },
            composition: rime_composition_from_c(&c_context.composition),
            menu: get_rime_menu(self.api.c_rime_api, self.session_id, &c_context.menu),
        };
        unsafe {
            c_free_context(&mut c_context);
        }
        context
    }

    pub fn get_commit(&self) -> RimeCommit {
        let mut c_commit = CRimecmdRimeCommit {
            text: std::ptr::null_mut(),
        };
        unsafe {
            c_get_commit(self.api.c_rime_api, self.session_id, &mut c_commit);
        }
        let commit = RimeCommit {
            text: (!c_commit.text.is_null()).then(|| {
                unsafe { std::ffi::CStr::from_ptr(c_commit.text) }
                    .to_str()
                    .unwrap()
                    .to_owned()
            }),
        };
        unsafe {
            c_free_commit(&mut c_commit);
        }
        commit
    }

    pub fn get_status(&self) -> RimeStatus {
        let mut c_status = CRimecmdRimeStatus {
            schema_id: std::ptr::null_mut(),
            schema_name: std::ptr::null_mut(),
            is_disabled: 0,
            is_composing: 0,
            is_ascii_mode: 0,
            is_full_shape: 0,
            is_simplified: 0,
            is_traditional: 0,
            is_ascii_punct: 0,
        };
        unsafe {
            c_get_status(self.api.c_rime_api, self.session_id, &mut c_status);
        }
        if c_status.schema_id.is_null() {
            panic!();
        }
        if c_status.schema_name.is_null() {
            panic!();
        }
        let status = RimeStatus {
            is_disabled: 1 == c_status.is_disabled,
            is_composing: 1 == c_status.is_composing,
            is_ascii_mode: 1 == c_status.is_ascii_mode,
            is_full_shape: 1 == c_status.is_full_shape,
            is_simplified: 1 == c_status.is_simplified,
            is_traditional: 1 == c_status.is_traditional,
            is_ascii_punct: 1 == c_status.is_ascii_punct,
            schema_id: unsafe { std::ffi::CStr::from_ptr(c_status.schema_id) }
                .to_str()
                .unwrap()
                .to_owned(),
            schema_name: unsafe { std::ffi::CStr::from_ptr(c_status.schema_name) }
                .to_str()
                .unwrap()
                .to_owned(),
        };
        unsafe {
            c_free_status(&mut c_status);
        }
        status
    }
}

impl Drop for RimeSession<'_> {
    fn drop(&mut self) {
        unsafe {
            c_destory_session(self.api.c_rime_api, self.session_id);
        }
    }
}

pub struct RimeApi {
    c_rime_api: *mut CRimeApi,
    _user_data_dir: std::boxed::Box<std::ffi::CString>,
    _shared_data_dir: std::boxed::Box<std::ffi::CString>,
}

impl Drop for RimeApi {
    fn drop(&mut self) {
        unsafe { c_destory_rime_api(self.c_rime_api) };
    }
}

impl RimeApi {
    /// * `log_level` - will only be effective this first time this is run.
    /// See the comment in the definition of `c_setup_rime_api_once`.
    pub fn new<P1, P2>(user_data_dir: P1, shared_data_dir: P2, log_level: LogLevel) -> Self
    where
        P1: AsRef<std::path::Path>,
        P2: AsRef<std::path::Path>,
    {
        let user_data_dir =
            std::boxed::Box::new(c_string_from_path(user_data_dir.as_ref()).unwrap());
        let shared_data_dir =
            std::boxed::Box::new(c_string_from_path(shared_data_dir.as_ref()).unwrap());
        Self {
            c_rime_api: {
                let c_rime_api = unsafe { c_get_rime_api() };
                RIME_API_SETUP.call_once(|| unsafe {
                    c_setup_rime_api_once(
                        c_rime_api,
                        user_data_dir.as_ptr(),
                        shared_data_dir.as_ptr(),
                        match log_level {
                            LogLevel::Info => 0,
                            LogLevel::Warning => 1,
                            LogLevel::Error => 2,
                            LogLevel::Fatal => 3,
                            LogLevel::None => 4,
                        },
                    )
                });
                unsafe {
                    c_initialize_rime_api(
                        c_rime_api,
                        user_data_dir.as_ptr(),
                        shared_data_dir.as_ptr(),
                        match log_level {
                            LogLevel::Info => 0,
                            LogLevel::Warning => 1,
                            LogLevel::Error => 2,
                            LogLevel::Fatal => 3,
                            LogLevel::None => 4,
                        },
                    );
                }
                unsafe {
                    c_do_maintenance(c_rime_api);
                }
                c_rime_api
            },
            _user_data_dir: user_data_dir,
            _shared_data_dir: shared_data_dir,
        }
    }

    pub fn get_schema_list(&self) -> Vec<RimeSchema> {
        let mut schema_list = CRimeSchemaList {
            size: 0,
            list: std::ptr::null_mut(),
        };
        unsafe {
            c_get_schema_list(self.c_rime_api, &mut schema_list);
        }
        let return_value = (0..schema_list.size)
            .map(|index| rime_schema_from_c(unsafe { &*(schema_list.list.add(index)) }))
            .collect();
        unsafe { c_free_schema_list(self.c_rime_api, &mut schema_list) };
        return return_value;
    }

    pub fn get_shared_data_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(
            unsafe { std::ffi::CStr::from_ptr(c_get_shared_data_dir(self.c_rime_api)) }
                .to_str()
                .unwrap(),
        )
    }

    pub fn get_user_data_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(
            unsafe { std::ffi::CStr::from_ptr(c_get_user_data_dir(self.c_rime_api)) }
                .to_str()
                .unwrap(),
        )
    }
}

#[derive(Copy, Clone, clap::ValueEnum, Serialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Fatal,
    None,
}

#[cfg(test)]
mod test {
    use crate::testing_utilities::{temporary_directory_path, LOG_LEVEL};

    #[test]
    #[ignore = "not thread safe"]
    fn get_context() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        rime_session.process_key(109 /* m */, 0);
        assert_eq!("m", rime_session.get_context().composition.preedit);
    }

    #[test]
    #[ignore = "not thread safe"]
    fn process_return() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        rime_session.process_key(109 /* m */, 0);
        rime_session.process_key(110 /* n */, 0);
        rime_session.process_key(111 /* o */, 0);
        rime_session.process_key(0xff0d /* Return */, 0);
        assert_eq!("mno", rime_session.get_commit().text.unwrap());
    }

    #[test]
    #[ignore = "not thread safe"]
    fn process_backspace() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        rime_session.process_key(109 /* m */, 0);
        rime_session.process_key(105 /* i */, 0);
        assert_eq!("mi", rime_session.get_context().composition.preedit);
        rime_session.process_key(0xff08 /* Backspace */, 0);
        assert_eq!("m", rime_session.get_context().composition.preedit);
    }

    #[test]
    #[ignore = "not thread safe"]
    fn process_ctrl_grave() {
        let rime_api = crate::rime_api::RimeApi::new(
            temporary_directory_path(),
            "./test_shared_data",
            LOG_LEVEL,
        );
        let rime_session = crate::rime_api::RimeSession::new(&rime_api);
        rime_session.process_key(96 /* ` */, 1 << 2 /* Control */);
        rime_session.process_key(50 /* 2 */, 0);
        let context = rime_session.get_context();
        assert_eq!(context.composition.preedit, "〔方案選單〕");
        assert_eq!(context.menu.candidates[1].text, "中文");
        assert_eq!(
            context.menu.candidates[1].comment.clone().unwrap(),
            "→ 西文"
        );
    }
}
