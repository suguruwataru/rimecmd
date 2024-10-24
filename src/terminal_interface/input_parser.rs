use super::Input;

enum ParserStateImpl {
    Start,
    Esc,
    Csi,
    Csi1NumParam(usize),
    Completed(Input),
    // Failed could mean unsupported sequence or invalid sequence
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
            ParserStateImpl::Failed => {
                ConsumeByteResult::Pending(ParserState(ParserStateImpl::Start))
            }
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
                0x04 => ParserStateImpl::Completed(Input::Eot),
                0x08 => ParserStateImpl::Completed(Input::Bs),
                0x09 => ParserStateImpl::Completed(Input::Ht),
                0x0a => ParserStateImpl::Completed(Input::Lf),
                0x0d => ParserStateImpl::Completed(Input::Cr),
                0x7f => ParserStateImpl::Completed(Input::Del),
                0x1b => ParserStateImpl::Esc,
                // All the other ASCII control character are not supported by rimecmd.
                _ if byte.is_ascii_control() => ParserStateImpl::Failed,
                _ => ParserStateImpl::Completed(Input::Char(char::from(byte))),
            },
            ParserStateImpl::Esc if byte == 0x5b => ParserStateImpl::Csi,
            ParserStateImpl::Csi if byte == 0x41 => ParserStateImpl::Completed(Input::Up),
            ParserStateImpl::Csi if byte == 0x42 => ParserStateImpl::Completed(Input::Down),
            ParserStateImpl::Csi if byte == 0x43 => ParserStateImpl::Completed(Input::Right),
            ParserStateImpl::Csi if byte == 0x44 => ParserStateImpl::Completed(Input::Left),
            ParserStateImpl::Csi if byte == 0x46 => ParserStateImpl::Completed(Input::End),
            ParserStateImpl::Csi if byte == 0x48 => ParserStateImpl::Completed(Input::Home),
            ParserStateImpl::Csi if byte.is_ascii_digit() => ParserStateImpl::Csi1NumParam(
                char::from_u32(byte.into()).unwrap().to_digit(10).unwrap() as usize,
            ),
            ParserStateImpl::Csi1NumParam(param1) if byte.is_ascii_digit() => {
                ParserStateImpl::Csi1NumParam(
                    param1 * 10
                        + char::from_u32(byte.into()).unwrap().to_digit(10).unwrap() as usize,
                )
            }
            ParserStateImpl::Csi1NumParam(param1) if byte == 0x7e => match param1 {
                1 => ParserStateImpl::Completed(Input::KeypadHome),
                2 => ParserStateImpl::Completed(Input::Insert),
                3 => ParserStateImpl::Completed(Input::Delete),
                4 => ParserStateImpl::Completed(Input::KeypadEnd),
                5 => ParserStateImpl::Completed(Input::PageUp),
                6 => ParserStateImpl::Completed(Input::PageDown),
                _ => ParserStateImpl::Failed,
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
            _ => ParserStateImpl::Failed,
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
    fn up() {
        if let ParserStateImpl::Completed(Input::Up) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x41)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn down() {
        if let ParserStateImpl::Completed(Input::Down) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x42)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn right() {
        if let ParserStateImpl::Completed(Input::Right) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x43)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn left() {
        if let ParserStateImpl::Completed(Input::Left) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x44)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn insert() {
        if let ParserStateImpl::Completed(Input::Insert) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x32)
            .consume_byte(0x7e)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn delete() {
        if let ParserStateImpl::Completed(Input::Delete) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x33)
            .consume_byte(0x7e)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn page_up() {
        if let ParserStateImpl::Completed(Input::PageUp) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x35)
            .consume_byte(0x7e)
        {
        } else {
            panic!();
        }
    }

    #[test]
    fn page_down() {
        if let ParserStateImpl::Completed(Input::PageDown) = ParserStateImpl::Start
            .consume_byte(0x1b)
            .consume_byte(0x5b)
            .consume_byte(0x36)
            .consume_byte(0x7e)
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
