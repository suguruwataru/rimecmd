use super::Input;

enum ParserStateImpl {
    Start,
    Esc,
    Completed(Input),
    Failed,
    Pending3ByteUtf8(Vec<u8>),
}

pub enum ConsumeByteResult {
    Pending(ParserState),
    Completed(Input),
}

pub struct ParserState(ParserStateImpl);

impl ParserState {
    pub fn new() -> Self {
        Self(ParserStateImpl::Start)
    }

    pub fn consume_byte(self, byte: u8) -> ConsumeByteResult {
        match self.0.consume_byte(byte) {
            ParserStateImpl::Completed(input) => ConsumeByteResult::Completed(input),
            ParserStateImpl::Failed => unimplemented!(),
            pending => ConsumeByteResult::Pending(Self(pending)),
        }
    }
}

impl ParserStateImpl {
    /// Arguments:
    /// * bytes - The bytes thats has been received for this UTF-8 character.
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
                0x00 => ParserStateImpl::Completed(Input::Nul),
                0x03 => ParserStateImpl::Completed(Input::Etx),
                0x0d => ParserStateImpl::Completed(Input::Cr),
                0x7f => ParserStateImpl::Completed(Input::Del),
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
    fn ascii_alpha() {
        if let ParserStateImpl::Completed(Input::Char('c')) =
            ParserStateImpl::Start.consume_byte('c' as u8)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn ascii_del() {
        // The ascii code sent by backspace key
        if let ParserStateImpl::Completed(Input::Del) = ParserStateImpl::Start.consume_byte(0x7f) {
        } else {
            panic!();
        }
    }

    #[test]
    fn ascii_nul() {
        // The ascii code sent by Ctrl-`
        if let ParserStateImpl::Completed(Input::Nul) = ParserStateImpl::Start.consume_byte(0x00) {
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
