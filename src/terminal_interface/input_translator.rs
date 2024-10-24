use super::Input;
use crate::rime_api::key_mappings::{
    rime_character_to_key_name_map, rime_key_name_to_key_code_map,
    rime_modifier_name_to_modifier_mask,
};
use std::collections::HashMap;

pub struct InputTranslator {
    rime_character_to_key_name_map: HashMap<char, &'static str>,
    rime_key_name_to_key_code_map: HashMap<&'static str, usize>,
    rime_modifier_name_to_modifer_mask: HashMap<&'static str, usize>,
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
            rime_modifier_name_to_modifer_mask: rime_modifier_name_to_modifier_mask(),
        }
    }

    pub fn translate_input(&self, input: Input) -> Option<RimeKey> {
        match input {
            Input::Etx => unreachable!(),
            Input::Left => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Left").unwrap(),
                mask: 0,
            }),
            Input::Right => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Right").unwrap(),
                mask: 0,
            }),
            Input::Up => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Up").unwrap(),
                mask: 0,
            }),
            Input::Down => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Down").unwrap(),
                mask: 0,
            }),
            Input::Home => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Home").unwrap(),
                mask: 0,
            }),
            Input::End => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("End").unwrap(),
                mask: 0,
            }),
            Input::Delete => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Delete").unwrap(),
                mask: 0,
            }),
            Input::Insert => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Insert").unwrap(),
                mask: 0,
            }),
            Input::KeypadEnd => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("KP_End").unwrap(),
                mask: 0,
            }),
            Input::KeypadHome => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("KP_Home").unwrap(),
                mask: 0,
            }),
            Input::PageDown => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Page_Down").unwrap(),
                mask: 0,
            }),
            Input::PageUp => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Page_Up").unwrap(),
                mask: 0,
            }),
            Input::Cr => Some(RimeKey {
                keycode: *self.rime_key_name_to_key_code_map.get("Return").unwrap(),
                mask: 0,
            }),
            // In a common terminal, nul can be sent in various cases: Ctrl-`, Ctrl-@, Ctrl-2.
            // If I remember correctly, there are more.
            // Unfortunately, there isn't really a standard or documentation for this.
            // Since Ctrl-` is the most relevant key binding for rime, nul is translated to it.
            Input::Nul => {
                let key_name = self.rime_character_to_key_name_map.get(&'`').unwrap();
                let keycode = self.rime_key_name_to_key_code_map.get(key_name).unwrap();
                Some(RimeKey {
                    keycode: *keycode,
                    mask: *self
                        .rime_modifier_name_to_modifer_mask
                        .get("Control")
                        .unwrap(),
                })
            }
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
