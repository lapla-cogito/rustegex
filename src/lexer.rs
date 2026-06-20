#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Character(char),
    Class(crate::charclass::CharClass),
    UnionOperator,
    StarOperator,
    PlusOperator,
    QuestionOperator,
    LeftParen,
    RightParen,
    InvalidEscape,
    Empty,
}

#[derive(Debug)]
pub struct Lexer<'a> {
    input: std::str::Chars<'a>,
}

impl Lexer<'_> {
    pub fn new(string: &'_ str) -> Lexer<'_> {
        Lexer {
            input: string.chars(),
        }
    }

    pub fn scan(&mut self) -> Token {
        let Some(char) = self.input.next() else {
            return Token::Empty;
        };

        match char {
            '\\' => match self.input.next() {
                Some(escaped) => {
                    if let Some(class) = crate::charclass::CharClass::from_escape(escaped) {
                        Token::Class(class)
                    } else {
                        Token::Character(escaped)
                    }
                }
                None => Token::InvalidEscape,
            },
            '|' => Token::UnionOperator,
            '(' => Token::LeftParen,
            ')' => Token::RightParen,
            '*' => Token::StarOperator,
            '+' => Token::PlusOperator,
            '?' => Token::QuestionOperator,
            '.' => Token::Class(crate::charclass::CharClass::Any),
            _ => Token::Character(char),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Character(c) => write!(f, "{c}"),
            Token::Class(crate::charclass::CharClass::Any) => write!(f, "."),
            Token::Class(crate::charclass::CharClass::Digit) => write!(f, r"\d"),
            Token::Class(crate::charclass::CharClass::Word) => write!(f, r"\w"),
            Token::Class(crate::charclass::CharClass::Space) => write!(f, r"\s"),
            Token::UnionOperator => write!(f, "|"),
            Token::StarOperator => write!(f, "*"),
            Token::PlusOperator => write!(f, "+"),
            Token::QuestionOperator => write!(f, "?"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::InvalidEscape => write!(f, r"[invalid escape]"),
            Token::Empty => write!(f, "[empty]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan() {
        let mut lexer = Lexer::new("a|b");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new("a|b*");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::StarOperator);
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new("a|b+");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::PlusOperator);
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new("a|b?");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::QuestionOperator);
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new("a|b()");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::LeftParen);
        assert_eq!(lexer.scan(), Token::RightParen);
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new("abc|def");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::Character('c'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('d'));
        assert_eq!(lexer.scan(), Token::Character('e'));
        assert_eq!(lexer.scan(), Token::Character('f'));
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new("a|(b|c)");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::LeftParen);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('c'));
        assert_eq!(lexer.scan(), Token::RightParen);
        assert_eq!(lexer.scan(), Token::Empty);
    }

    #[test]
    fn with_escape() {
        let mut lexer = Lexer::new(r"a|\|\\(\)");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('|'));
        assert_eq!(lexer.scan(), Token::Character('\\'));
        assert_eq!(lexer.scan(), Token::LeftParen);
        assert_eq!(lexer.scan(), Token::Character(')'));
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new(r"a|b\*");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::Character('*'));
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new(r"a|b\+");
        assert_eq!(lexer.scan(), Token::Character('a'));
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Character('b'));
        assert_eq!(lexer.scan(), Token::Character('+'));
        assert_eq!(lexer.scan(), Token::Empty);
    }

    #[test]
    fn metacharacters() {
        let mut lexer = Lexer::new(r"\d|\w|\s|.");
        assert_eq!(
            lexer.scan(),
            Token::Class(crate::charclass::CharClass::Digit)
        );
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(
            lexer.scan(),
            Token::Class(crate::charclass::CharClass::Word)
        );
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(
            lexer.scan(),
            Token::Class(crate::charclass::CharClass::Space)
        );
        assert_eq!(lexer.scan(), Token::UnionOperator);
        assert_eq!(lexer.scan(), Token::Class(crate::charclass::CharClass::Any));
        assert_eq!(lexer.scan(), Token::Empty);

        let mut lexer = Lexer::new(r"\.");
        assert_eq!(lexer.scan(), Token::Character('.'));
        assert_eq!(lexer.scan(), Token::Empty);
    }

    #[test]
    fn empty() {
        let mut lexer = Lexer::new(r"");
        assert_eq!(lexer.scan(), Token::Empty);
    }
}
