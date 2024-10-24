pub enum Input {
    Char(char),
}

enum ParserStateImpl {
    Start,
    Esc,
    Completed(Input),
    Failed,
    Pending3ByteUtf8(Vec<u8>),
}

pub struct ParserState(ParserStateImpl);

#[allow(dead_code)]
impl ParserState {
    fn new() -> Self {
        Self(ParserStateImpl::Start)
    }
}

#[allow(dead_code)]
impl ParserStateImpl {
    /// Arguments:
    /// * bytes - The bytes thats has been received for this UTF-8 character.
    /// * total_byte_count - The expected total byte count of the charater
    ///   that is being parsed.
    fn try_decode_utf8(bytes: &[u8]) -> Self {
        std::str::from_utf8(
            &std::iter::repeat(0u8)
                .take(4 - bytes.len())
                .chain(bytes.iter().map(|byte_ref| *byte_ref))
                .collect::<Vec<_>>(),
        )
        .ok()
        .and_then(|string| string.chars().nth(1))
        .map(|character| ParserStateImpl::Completed(Input::Char(character)))
        .unwrap_or(ParserStateImpl::Failed)
    }

    fn consume_byte(self, byte: u8) -> Self {
        match self {
            ParserStateImpl::Start if byte.is_ascii() => match byte {
                0x1b => ParserStateImpl::Esc,
                _ if byte.is_ascii_control() => unimplemented!(),
                _ => ParserStateImpl::Completed(Input::Char(char::from(byte))),
            },
            ParserStateImpl::Start if byte & 0b11100000 == 0b11100000 => {
                ParserStateImpl::Pending3ByteUtf8(vec![byte])
            }
            ParserStateImpl::Pending3ByteUtf8(bytes)
                if bytes.len() < 3 && byte & 0b10000000 == 0b10000000 =>
            {
                if bytes.len() == 2 {
                    Self::try_decode_utf8(&[0, bytes[0], bytes[1], byte])
                } else {
                    ParserStateImpl::Pending3ByteUtf8(
                        bytes.into_iter().chain(std::iter::once(byte)).collect(),
                    )
                }
            }
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ascii_char() {
        if let ParserStateImpl::Completed(Input::Char('c')) =
            ParserStateImpl::Start.consume_byte('c' as u8)
        {
        } else {
            panic!();
        }
    }
    #[test]
    fn utf8_3_bytes() {
        if let ParserStateImpl::Completed(Input::Char('好')) = "好"
            .as_bytes()
            .into_iter()
            .fold(ParserStateImpl::Start, |parser_state, byte| {
                parser_state.consume_byte(*byte)
            })
        {
        } else {
            panic!();
        }
    }
}
