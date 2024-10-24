use super::input_translator::InputTranslator;
use super::Input;

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
