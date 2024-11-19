mod dfa;
mod error;
mod lexer;
mod nfa;
mod parser;

pub use error::{Error, Result};

pub struct RustRegex {
    dfa: dfa::Dfa,
}

impl RustRegex {
    pub fn new(input: &str) -> std::result::Result<RustRegex, String> {
        let mut lexer = lexer::Lexer::new(input);
        let mut parser = parser::Parser::new(&mut lexer);
        let nfa = match nfa::Nfa::new_from_node(
            parser.parse().map_err(|e| e.to_string())?,
            &mut nfa::NfaState::new(),
        ) {
            Ok(nfa) => nfa,
            Err(e) => return Err(e.to_string()),
        };
        let dfa = dfa::Dfa::from_nfa(&nfa);

        Ok(RustRegex { dfa })
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut current = self.dfa.start();
        for c in input.chars() {
            if let Some(state) = self.dfa.next_transit(current, c) {
                current = state;
            } else {
                return false;
            }
        }

        self.dfa.accept().contains(&current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex() {
        let regex = RustRegex::new("a|b*").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a|b").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a*").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = RustRegex::new("(p(erl|ython|hp)|ruby)").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = RustRegex::new("a(b|)").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = RustRegex::new("ab(cd|)").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));
    }

    #[test]
    fn with_escape() {
        let regex = RustRegex::new(r"a\|b").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\*b").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\+b").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\?b").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\|b\*").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn invalid() {
        for test in ["a(b", "*", ")c", "|", "*", "+"] {
            let regex = RustRegex::new(test);
            assert!(regex.is_err());
        }
    }
}
