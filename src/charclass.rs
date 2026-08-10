#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharClass {
    Any,
    Digit,
    Word,
    Space,
    DotAll,
}

impl CharClass {
    #[inline(always)]
    pub fn matches(self, c: char) -> bool {
        match self {
            CharClass::Any => c != '\n',
            CharClass::Digit => c.is_ascii_digit(),
            CharClass::Word => c == '_' || c.is_ascii_alphanumeric(),
            CharClass::Space => matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c' | '\x0b'),
            CharClass::DotAll => true,
        }
    }

    #[inline(always)]
    pub fn matches_byte(self, b: u8) -> bool {
        match self {
            CharClass::Any => b != b'\n',
            CharClass::Digit => b.is_ascii_digit(),
            CharClass::Word => b == b'_' || b.is_ascii_alphanumeric(),
            CharClass::Space => matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b),
            CharClass::DotAll => true,
        }
    }

    #[inline]
    pub fn find_byte(self, haystack: &[u8], at: usize) -> Option<usize> {
        match self {
            CharClass::Digit => find_digit(haystack, at),
            CharClass::Word => find_unrolled(haystack, at, is_ascii_word_u8),
            CharClass::Space => find_unrolled(haystack, at, is_ascii_space_u8),
            CharClass::Any => find_unrolled(haystack, at, |b| b != b'\n'),
            CharClass::DotAll => {
                if at < haystack.len() {
                    Some(at)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub fn skip_bytes(self, haystack: &[u8], at: usize) -> usize {
        match self {
            CharClass::Digit => skip_digits(haystack, at),
            CharClass::Word => skip_unrolled(haystack, at, is_ascii_word_u8),
            CharClass::Space => skip_unrolled(haystack, at, is_ascii_space_u8),
            CharClass::Any => skip_unrolled(haystack, at, |b| b != b'\n'),
            CharClass::DotAll => haystack.len(),
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

#[inline(always)]
fn is_ascii_digit_u8(b: u8) -> bool {
    b.wrapping_sub(b'0') < 10
}

#[inline(always)]
fn is_ascii_word_u8(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

#[inline(always)]
fn is_ascii_space_u8(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b)
}

#[inline]
fn find_unrolled(haystack: &[u8], at: usize, pred: impl Fn(u8) -> bool) -> Option<usize> {
    let slice = haystack.get(at..)?;
    let mut i = 0;
    while i + 8 <= slice.len() {
        let chunk = unsafe { slice.get_unchecked(i..i + 8) };
        if pred(chunk[0]) {
            return Some(at + i);
        }
        if pred(chunk[1]) {
            return Some(at + i + 1);
        }
        if pred(chunk[2]) {
            return Some(at + i + 2);
        }
        if pred(chunk[3]) {
            return Some(at + i + 3);
        }
        if pred(chunk[4]) {
            return Some(at + i + 4);
        }
        if pred(chunk[5]) {
            return Some(at + i + 5);
        }
        if pred(chunk[6]) {
            return Some(at + i + 6);
        }
        if pred(chunk[7]) {
            return Some(at + i + 7);
        }
        i += 8;
    }
    while i < slice.len() {
        if pred(slice[i]) {
            return Some(at + i);
        }
        i += 1;
    }
    None
}

#[inline]
fn skip_unrolled(haystack: &[u8], mut at: usize, pred: impl Fn(u8) -> bool) -> usize {
    let len = haystack.len();
    while at + 8 <= len {
        let chunk = unsafe { haystack.get_unchecked(at..at + 8) };
        if !pred(chunk[0]) {
            return at;
        }
        if !pred(chunk[1]) {
            return at + 1;
        }
        if !pred(chunk[2]) {
            return at + 2;
        }
        if !pred(chunk[3]) {
            return at + 3;
        }
        if !pred(chunk[4]) {
            return at + 4;
        }
        if !pred(chunk[5]) {
            return at + 5;
        }
        if !pred(chunk[6]) {
            return at + 6;
        }
        if !pred(chunk[7]) {
            return at + 7;
        }
        at += 8;
    }
    while at < len && pred(haystack[at]) {
        at += 1;
    }
    at
}

#[inline]
pub fn find_digit(haystack: &[u8], at: usize) -> Option<usize> {
    find_unrolled(haystack, at, is_ascii_digit_u8)
}

#[inline]
pub fn skip_digits(haystack: &[u8], at: usize) -> usize {
    skip_unrolled(haystack, at, is_ascii_digit_u8)
}

#[inline]
pub fn skip_equal_bytes(haystack: &[u8], mut at: usize, byte: u8) -> usize {
    let len = haystack.len();
    let splat = u64::from_ne_bytes([byte; 8]);
    while at + 8 <= len {
        let chunk = u64::from_ne_bytes(unsafe { *(haystack.as_ptr().add(at) as *const [u8; 8]) });
        if chunk != splat {
            for j in 0..8 {
                if haystack[at + j] != byte {
                    return at + j;
                }
            }
        }
        at += 8;
    }
    while at < len && haystack[at] == byte {
        at += 1;
    }
    at
}

#[inline]
pub fn find_exit_mask(haystack: &[u8], at: usize, exit_mask: u128) -> Option<usize> {
    if exit_mask == 0 {
        return None;
    }
    let slice = haystack.get(at..)?;
    for (i, &b) in slice.iter().enumerate() {
        if b < 128 && (exit_mask & (1u128 << b)) != 0 {
            return Some(at + i);
        }
    }
    None
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
        assert!(CharClass::DotAll.matches('x'));
        assert!(CharClass::DotAll.matches('\n'));
    }

    #[test]
    fn find_digit_swar() {
        let s = b"abcdefgh12xy";
        assert_eq!(find_digit(s, 0), Some(8));
        assert_eq!(find_digit(s, 9), Some(9));
        assert_eq!(find_digit(b"nodigits!!!!", 0), None);
        let long = [b'x'; 100];
        assert_eq!(find_digit(&long, 0), None);
        let mut with = long;
        with[64] = b'7';
        assert_eq!(find_digit(&with, 0), Some(64));
    }

    #[test]
    fn skip_digits_swar() {
        assert_eq!(skip_digits(b"12345abc", 0), 5);
        assert_eq!(skip_digits(b"abc", 0), 0);
        let digits = [b'0'; 40];
        assert_eq!(skip_digits(&digits, 0), 40);
    }

    #[test]
    fn skip_equal_bytes_runs() {
        let s = [b'a'; 100];
        assert_eq!(skip_equal_bytes(&s, 0, b'a'), 100);
        let mut s2 = s;
        s2[50] = b'b';
        assert_eq!(skip_equal_bytes(&s2, 0, b'a'), 50);
    }

    #[test]
    fn digit_class_find_byte() {
        assert_eq!(CharClass::Digit.find_byte(b"xxx9yy", 0), Some(3));
        assert_eq!(CharClass::Digit.skip_bytes(b"99x", 0), 2);
    }

    #[test]
    fn find_digit_after_slashes() {
        assert_eq!(find_digit(b"////0///", 0), Some(4));
        assert_eq!(find_digit(b"ab9defgh", 0), Some(2));
    }
}
