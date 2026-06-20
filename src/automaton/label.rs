#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NfaLabel {
    Epsilon,
    Char(char),
    Class(crate::charclass::CharClass),
}
