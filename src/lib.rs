mod automaton;
mod derivative;
mod error;
mod lexer;
mod parser;
mod vm;

pub use error::{Error, Result};

#[derive(Debug)]
enum Regex {
    Dfa { dfa: automaton::dfa::Dfa },
    Vm { vm: vm::Vm },
    Derivative { derivative: derivative::Derivative },
}

#[derive(Debug)]
pub struct RustRegex {
    regex: Regex,
}

impl RustRegex {
    pub fn new(input: &str, method: &'static str) -> Result<RustRegex> {
        let mut lexer = lexer::Lexer::new(input);
        let mut parser = parser::Parser::new(&mut lexer);
        let ast = parser.parse()?;

        if method == "dfa" {
            let nfa =
                automaton::nfa::Nfa::new_from_node(ast, &mut automaton::nfa::NfaState::new())?;
            let dfa = automaton::dfa::Dfa::from_nfa(&nfa);

            Ok(RustRegex {
                regex: Regex::Dfa { dfa },
            })
        } else if method == "vm" {
            let vm = vm::Vm::new(ast)?;

            Ok(RustRegex {
                regex: Regex::Vm { vm },
            })
        } else if method == "derivative" {
            let derivative = derivative::Derivative::new(ast);

            Ok(RustRegex {
                regex: Regex::Derivative { derivative },
            })
        } else {
            Err(Error::InvalidMethod(method.to_string()))
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        match &self.regex {
            Regex::Dfa { dfa } => dfa.is_match(input),
            Regex::Vm { vm } => vm.is_match(input),
            Regex::Derivative { derivative } => {
                if input.is_empty() {
                    derivative.is_empty_match()
                } else {
                    derivative.is_match(input)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_dfa() {
        let regex = RustRegex::new("a|b*", "dfa").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a|b", "dfa").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a*", "dfa").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = RustRegex::new("(p(erl|ython|hp)|ruby)", "dfa").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = RustRegex::new("a(b|)", "dfa").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = RustRegex::new("ab(cd|)", "dfa").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));

        let regex = RustRegex::new("a+b", "dfa").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("aab"));
        assert!(regex.is_match("aaab"));
        assert!(!regex.is_match("a"));
    }

    #[test]
    fn with_escape_dfa() {
        let regex = RustRegex::new(r"a\|b", "dfa").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\*b", "dfa").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\+b", "dfa").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\?b", "dfa").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\|b\*", "dfa").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn nonascii_dfa() {
        let regex = RustRegex::new("あ|い*", "dfa").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("あ|い", "dfa").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("い*", "dfa").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("(ぱ(あ|い|う)|え)", "dfa").unwrap();
        assert!(regex.is_match("ぱあ"));
        assert!(regex.is_match("ぱい"));
        assert!(regex.is_match("ぱう"));
        assert!(regex.is_match("え"));
        assert!(!regex.is_match("お"));

        let regex = RustRegex::new("い(あ|)", "dfa").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("いあい"));

        let regex = RustRegex::new("いあ(うえ|)", "dfa").unwrap();
        assert!(regex.is_match("いあうえ"));
        assert!(regex.is_match("いあ"));
        assert!(!regex.is_match("いあう"));
        assert!(regex.is_match("いあうえ"));

        let regex = RustRegex::new("い+あ", "dfa").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("いいあ"));
        assert!(regex.is_match("いいいあ"));
        assert!(!regex.is_match("い"));

        let regex = RustRegex::new("正規表現(太郎|次郎)", "dfa").unwrap();
        assert!(regex.is_match("正規表現太郎"));
        assert!(regex.is_match("正規表現次郎"));
        assert!(!regex.is_match("正規表現三郎"));

        let regex = RustRegex::new("あい|♥", "dfa").unwrap();
        assert!(regex.is_match("あい"));
        assert!(regex.is_match("♥"));
        assert!(!regex.is_match("♡"));
        assert!(!regex.is_match("👎️"));

        let regex = RustRegex::new("ගවයා|ng'ombe", "dfa").unwrap();
        assert!(regex.is_match("ගවයා"));
        assert!(regex.is_match("ng'ombe"));
        assert!(!regex.is_match("ගවයාng'ombe"));

        let regex = RustRegex::new("(පරිගණකය)*", "dfa").unwrap();
        assert!(regex.is_match("පරිගණකය"));
        assert!(regex.is_match(""));
    }

    #[test]
    fn invalid_dfa() {
        for test in ["a(b", "*", ")c", "+"] {
            let regex = RustRegex::new(test, "dfa");
            assert!(regex.is_err());
        }
    }

    #[test]
    fn regex_vm() {
        let regex = RustRegex::new("a|b*", "vm").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a|b", "vm").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a*", "vm").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = RustRegex::new("(p(erl|ython|hp)|ruby)", "vm").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = RustRegex::new("a(b|)", "vm").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = RustRegex::new("ab(cd|)", "vm").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));

        let regex = RustRegex::new("a+b", "vm").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("aab"));
        assert!(regex.is_match("aaab"));
        assert!(!regex.is_match("a"));
    }

    #[test]
    fn with_escape_vm() {
        let regex = RustRegex::new(r"a\|b", "vm").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\*b", "vm").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\+b", "vm").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\?b", "vm").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\|b\*", "vm").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn nonascii_vm() {
        let regex = RustRegex::new("あ|い*", "vm").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("あ|い", "vm").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("い*", "vm").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("(ぱ(あ|い|う)|え)", "vm").unwrap();
        assert!(regex.is_match("ぱあ"));
        assert!(regex.is_match("ぱい"));
        assert!(regex.is_match("ぱう"));
        assert!(regex.is_match("え"));
        assert!(!regex.is_match("お"));

        let regex = RustRegex::new("い(あ|)", "vm").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("いあい"));

        let regex = RustRegex::new("いあ(うえ|)", "vm").unwrap();
        assert!(regex.is_match("いあうえ"));
        assert!(regex.is_match("いあ"));
        assert!(!regex.is_match("いあう"));
        assert!(regex.is_match("いあうえ"));

        let regex = RustRegex::new("い+あ", "vm").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("いいあ"));
        assert!(regex.is_match("いいいあ"));
        assert!(!regex.is_match("い"));

        let regex = RustRegex::new("正規表現(太郎|次郎)", "vm").unwrap();
        assert!(regex.is_match("正規表現太郎"));
        assert!(regex.is_match("正規表現次郎"));
        assert!(!regex.is_match("正規表現三郎"));

        let regex = RustRegex::new("あい|♥", "vm").unwrap();
        assert!(regex.is_match("あい"));
        assert!(regex.is_match("♥"));
        assert!(!regex.is_match("♡"));
        assert!(!regex.is_match("👎️"));

        let regex = RustRegex::new("ගවයා|ng'ombe", "vm").unwrap();
        assert!(regex.is_match("ගවයා"));
        assert!(regex.is_match("ng'ombe"));
        assert!(!regex.is_match("ගවයාng'ombe"));

        let regex = RustRegex::new("(පරිගණකය)*", "vm").unwrap();
        assert!(regex.is_match("පරිගණකය"));
        assert!(regex.is_match(""));
    }

    #[test]
    fn invalid_vm() {
        for test in ["a(b", "*", ")c", "+"] {
            let regex = RustRegex::new(test, "vm");
            assert!(regex.is_err());
        }
    }

    #[test]
    fn regex_derivartive() {
        let regex = RustRegex::new("a|b*", "derivative").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a|b", "derivative").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = RustRegex::new("a*", "derivative").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = RustRegex::new("(p(erl|ython|hp)|ruby)", "derivative").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = RustRegex::new("a(b|)", "derivative").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = RustRegex::new("ab(cd|)", "derivative").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));

        let regex = RustRegex::new("a+b", "derivative").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("aab"));
        assert!(regex.is_match("aaab"));
        assert!(!regex.is_match("a"));
    }

    #[test]
    fn with_escape_derivative() {
        let regex = RustRegex::new(r"a\|b", "derivative").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\*b", "derivative").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\+b", "derivative").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\?b", "derivative").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = RustRegex::new(r"a\|b\*", "derivative").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn nonascii_derivative() {
        let regex = RustRegex::new("あ|い*", "derivative").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("あ|い", "derivative").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("い*", "derivative").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = RustRegex::new("(ぱ(あ|い|う)|え)", "derivative").unwrap();
        assert!(regex.is_match("ぱあ"));
        assert!(regex.is_match("ぱい"));
        assert!(regex.is_match("ぱう"));
        assert!(regex.is_match("え"));
        assert!(!regex.is_match("お"));

        let regex = RustRegex::new("い(あ|)", "derivative").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("いあい"));

        let regex = RustRegex::new("いあ(うえ|)", "derivative").unwrap();
        assert!(regex.is_match("いあうえ"));
        assert!(regex.is_match("いあ"));
        assert!(!regex.is_match("いあう"));
        assert!(regex.is_match("いあうえ"));

        let regex = RustRegex::new("い+あ", "derivative").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("いいあ"));
        assert!(regex.is_match("いいいあ"));
        assert!(!regex.is_match("い"));

        let regex = RustRegex::new("正規表現(太郎|次郎)", "derivative").unwrap();
        assert!(regex.is_match("正規表現太郎"));
        assert!(regex.is_match("正規表現次郎"));
        assert!(!regex.is_match("正規表現三郎"));

        let regex = RustRegex::new("あい|♥", "derivative").unwrap();
        assert!(regex.is_match("あい"));
        assert!(regex.is_match("♥"));
        assert!(!regex.is_match("♡"));
        assert!(!regex.is_match("👎️"));

        let regex = RustRegex::new("ගවයා|ng'ombe", "derivative").unwrap();
        assert!(regex.is_match("ගවයා"));
        assert!(regex.is_match("ng'ombe"));
        assert!(!regex.is_match("ගවයාng'ombe"));

        let regex = RustRegex::new("(පරිගණකය)*", "derivative").unwrap();
        assert!(regex.is_match("පරිගණකය"));
        assert!(regex.is_match(""));
    }

    #[test]
    fn invalid_derivative() {
        for test in ["a(b", "*", ")c", "+"] {
            let regex = RustRegex::new(test, "derivative");
            assert!(regex.is_err());
        }
    }

    #[test]
    fn invalid_method_name() {
        let regex = RustRegex::new("a", "正規表現太郎");
        assert!(regex.is_err());
    }
}
