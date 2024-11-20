#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Char(char),
    Split(usize, usize),
    Jmp(usize),
    Match,
}
