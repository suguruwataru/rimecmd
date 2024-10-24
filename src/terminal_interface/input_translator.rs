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
            Input::Cr => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Return").unwrap(),
                mask: 0,
            }),
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

#[cfg(test)]
mod test {

    use crate::terminal_interface::input_translator::InputTranslator;
    use crate::terminal_interface::Input;

    #[test]
    fn test_translate_unsupported_character() {
        let input_translator = InputTranslator::new();
        assert!(input_translator
            .translate_input(Input::Char('天'))
            .is_none())
    }

    #[test]
    fn test_translate_alpha() {
        let input_translator = InputTranslator::new();
        let result = input_translator.translate_input(Input::Char('a')).unwrap();
        assert_eq!(result.keycode, 0x61);
        assert_eq!(result.mask, 0);
    }

    #[test]
    fn test_translate_digit() {
        let input_translator = InputTranslator::new();
        let result = input_translator.translate_input(Input::Char('1')).unwrap();
        assert_eq!(result.keycode, 0x31);
        assert_eq!(result.mask, 0);
    }

    #[test]
    fn test_translate_punct() {
        let input_translator = InputTranslator::new();
        let result = input_translator.translate_input(Input::Char('!')).unwrap();
        assert_eq!(result.keycode, 0x21);
        assert_eq!(result.mask, 0);
    }

    #[test]
    fn test_translate_del() {
        let input_translator = InputTranslator::new();
        let result = input_translator.translate_input(Input::Del).unwrap();
        // del is the ascii code sent by the backspace key.
        assert_eq!(result.keycode, 0xff08);
        assert_eq!(result.mask, 0);
    }

    #[test]
    fn test_translate_cr() {
        let input_translator = InputTranslator::new();
        let result = input_translator.translate_input(Input::Cr).unwrap();
        // del is the ascii code sent by the backspace key.
        assert_eq!(result.keycode, 0xff0d);
        assert_eq!(result.mask, 0);
    }
}
