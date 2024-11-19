#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unexpected character: {0}")]
    UnexpectedChar(crate::lexer::Token),
    #[error("unexpected end of input")]
    UnexpectedEnd,
    #[error("expected {0}")]
    Expected(crate::lexer::Token),
    #[error("state id overflow")]
    StateIDOverflow(usize),
    #[error("invalid sequence")]
    InvalidSeq,
}

pub type Result<T> = std::result::Result<T, Error>;
