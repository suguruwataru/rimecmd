use super::Input;
use crate::rime_api::key_mappings::{
    rime_character_to_key_name_map, rime_key_name_to_key_code_map,
};
use std::collections::HashMap;

pub struct InputTranslator {
    rime_character_to_key_name_map: HashMap<char, &'static str>,
    rime_key_name_to_key_code_map: HashMap<&'static str, usize>,
}

pub struct RimeKey {
    pub keycode: usize,
    pub mask: usize,
}

impl InputTranslator {
    pub fn new() -> Self {
        Self {
            rime_key_name_to_key_code_map: rime_key_name_to_key_code_map(),
            rime_character_to_key_name_map: rime_character_to_key_name_map(),
        }
    }

    pub fn translate_input(&self, input: Input) -> Option<RimeKey> {
        match input {
            Input::Etx => unreachable!(),
            Input::Nul => unimplemented!(),
            Input::Del => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("BackSpace").unwrap(),
                mask: 0,
            }),
            Input::Char(character) => self
                .rime_character_to_key_name_map
                .get(&character)
                .and_then(|key_name| {
                    self.rime_key_name_to_key_code_map
                        .get(key_name)
                        .and_then(|keycode| {
                            Some(RimeKey {
                                keycode: *keycode,
                                mask: 0,
                            })
                        })
                }),
        }
    }
}
