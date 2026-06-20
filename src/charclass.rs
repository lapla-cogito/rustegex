#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharClass {
    Any,
    Digit,
    Word,
    Space,
}

impl CharClass {
    #[inline(always)]
    pub fn matches(self, c: char) -> bool {
        match self {
            CharClass::Any => c != '\n',
            CharClass::Digit => c.is_ascii_digit(),
            CharClass::Word => c == '_' || c.is_ascii_alphanumeric(),
            CharClass::Space => matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c' | '\x0b'),
        }
    }

    pub fn from_escape(c: char) -> Option<Self> {
        match c {
            'd' => Some(CharClass::Digit),
            'w' => Some(CharClass::Word),
            's' => Some(CharClass::Space),
            _ => None,
        }
    }

    pub fn expand_ascii(self) -> [bool; 128] {
        let mut table = [false; 128];
        for byte in 0u8..128 {
            table[byte as usize] = self.matches(byte as char);
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_basic() {
        assert!(CharClass::Digit.matches('0'));
        assert!(!CharClass::Digit.matches('a'));
        assert!(CharClass::Word.matches('_'));
        assert!(CharClass::Word.matches('Z'));
        assert!(!CharClass::Word.matches('-'));
        assert!(CharClass::Space.matches(' '));
        assert!(CharClass::Any.matches('x'));
        assert!(!CharClass::Any.matches('\n'));
    }
}
