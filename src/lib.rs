mod automaton;
mod derivative;
mod error;
mod lexer;
mod parser;
mod vm;

pub use error::{Error, Result};

#[global_allocator]
static MIMALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug)]
enum Regex {
    Dfa { dfa: automaton::dfa::Dfa },
    Vm { vm: vm::Vm },
    Derivative { derivative: derivative::Derivative },
}

#[derive(Debug)]
pub struct Engine {
    regex: Regex,
}

impl Engine {
    pub fn new(input: &str, method: &'static str) -> Result<Engine> {
        let mut lexer = lexer::Lexer::new(input);
        let mut parser = parser::Parser::new(&mut lexer);
        let ast = parser.parse()?;

        match method {
            "dfa" => {
                let nfa =
                    automaton::nfa::Nfa::new_from_node(ast, &mut automaton::nfa::NfaState::new())?;
                let dfa = automaton::dfa::Dfa::from_nfa(&nfa);

                Ok(Engine {
                    regex: Regex::Dfa { dfa },
                })
            }
            "vm" => {
                let vm = vm::Vm::new(ast)?;

                Ok(Engine {
                    regex: Regex::Vm { vm },
                })
            }
            "derivative" => {
                let derivative = derivative::Derivative::new(ast);

                Ok(Engine {
                    regex: Regex::Derivative { derivative },
                })
            }
            _ => Err(Error::InvalidMethod(method.to_string())),
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        match &self.regex {
            Regex::Dfa { dfa } => dfa.is_match(input),
            Regex::Vm { vm } => vm.is_match(input),
            Regex::Derivative { derivative } => {
                if input.is_empty() {
                    return derivative.is_empty_match();
                }
                derivative.is_match(input)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_dfa() {
        let regex = Engine::new("a|b*", "dfa").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = Engine::new("a|b", "dfa").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = Engine::new("a*", "dfa").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = Engine::new("(p(erl|ython|hp)|ruby)", "dfa").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = Engine::new("a(b|)", "dfa").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = Engine::new("ab(cd|)", "dfa").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));

        let regex = Engine::new("a+b", "dfa").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("aab"));
        assert!(regex.is_match("aaab"));
        assert!(!regex.is_match("a"));
    }

    #[test]
    fn with_escape_dfa() {
        let regex = Engine::new(r"a\|b", "dfa").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\*b", "dfa").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\+b", "dfa").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\?b", "dfa").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\|b\*", "dfa").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn nonascii_dfa() {
        let regex = Engine::new("あ|い*", "dfa").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("あ|い", "dfa").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("い*", "dfa").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("(ぱ(あ|い|う)|え)", "dfa").unwrap();
        assert!(regex.is_match("ぱあ"));
        assert!(regex.is_match("ぱい"));
        assert!(regex.is_match("ぱう"));
        assert!(regex.is_match("え"));
        assert!(!regex.is_match("お"));

        let regex = Engine::new("い(あ|)", "dfa").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("いあい"));

        let regex = Engine::new("いあ(うえ|)", "dfa").unwrap();
        assert!(regex.is_match("いあうえ"));
        assert!(regex.is_match("いあ"));
        assert!(!regex.is_match("いあう"));
        assert!(regex.is_match("いあうえ"));

        let regex = Engine::new("い+あ", "dfa").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("いいあ"));
        assert!(regex.is_match("いいいあ"));
        assert!(!regex.is_match("い"));

        let regex = Engine::new("正規表現(太郎|次郎)", "dfa").unwrap();
        assert!(regex.is_match("正規表現太郎"));
        assert!(regex.is_match("正規表現次郎"));
        assert!(!regex.is_match("正規表現三郎"));

        let regex = Engine::new("あい|♥", "dfa").unwrap();
        assert!(regex.is_match("あい"));
        assert!(regex.is_match("♥"));
        assert!(!regex.is_match("♡"));
        assert!(!regex.is_match("👎️"));

        let regex = Engine::new("ගවයා|ng'ombe", "dfa").unwrap();
        assert!(regex.is_match("ගවයා"));
        assert!(regex.is_match("ng'ombe"));
        assert!(!regex.is_match("ගවයාng'ombe"));

        let regex = Engine::new("(පරිගණකය)*", "dfa").unwrap();
        assert!(regex.is_match("පරිගණකය"));
        assert!(regex.is_match(""));
    }

    #[test]
    fn invalid_dfa() {
        for test in ["a(b", "*", ")c", "+"] {
            let regex = Engine::new(test, "dfa");
            assert!(regex.is_err());
        }
    }

    #[test]
    fn regex_vm() {
        let regex = Engine::new("a|b*", "vm").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = Engine::new("a|b", "vm").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = Engine::new("a*", "vm").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = Engine::new("(p(erl|ython|hp)|ruby)", "vm").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = Engine::new("a(b|)", "vm").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = Engine::new("ab(cd|)", "vm").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));

        let regex = Engine::new("a+b", "vm").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("aab"));
        assert!(regex.is_match("aaab"));
        assert!(!regex.is_match("a"));
    }

    #[test]
    fn with_escape_vm() {
        let regex = Engine::new(r"a\|b", "vm").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\*b", "vm").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\+b", "vm").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\?b", "vm").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\|b\*", "vm").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn nonascii_vm() {
        let regex = Engine::new("あ|い*", "vm").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("あ|い", "vm").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("い*", "vm").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("(ぱ(あ|い|う)|え)", "vm").unwrap();
        assert!(regex.is_match("ぱあ"));
        assert!(regex.is_match("ぱい"));
        assert!(regex.is_match("ぱう"));
        assert!(regex.is_match("え"));
        assert!(!regex.is_match("お"));

        let regex = Engine::new("い(あ|)", "vm").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("いあい"));

        let regex = Engine::new("いあ(うえ|)", "vm").unwrap();
        assert!(regex.is_match("いあうえ"));
        assert!(regex.is_match("いあ"));
        assert!(!regex.is_match("いあう"));
        assert!(regex.is_match("いあうえ"));

        let regex = Engine::new("い+あ", "vm").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("いいあ"));
        assert!(regex.is_match("いいいあ"));
        assert!(!regex.is_match("い"));

        let regex = Engine::new("正規表現(太郎|次郎)", "vm").unwrap();
        assert!(regex.is_match("正規表現太郎"));
        assert!(regex.is_match("正規表現次郎"));
        assert!(!regex.is_match("正規表現三郎"));

        let regex = Engine::new("あい|♥", "vm").unwrap();
        assert!(regex.is_match("あい"));
        assert!(regex.is_match("♥"));
        assert!(!regex.is_match("♡"));
        assert!(!regex.is_match("👎️"));

        let regex = Engine::new("ගවයා|ng'ombe", "vm").unwrap();
        assert!(regex.is_match("ගවයා"));
        assert!(regex.is_match("ng'ombe"));
        assert!(!regex.is_match("ගවයාng'ombe"));

        let regex = Engine::new("(පරිගණකය)*", "vm").unwrap();
        assert!(regex.is_match("පරිගණකය"));
        assert!(regex.is_match(""));
    }

    #[test]
    fn invalid_vm() {
        for test in ["a(b", "*", ")c", "+"] {
            let regex = Engine::new(test, "vm");
            assert!(regex.is_err());
        }
    }

    #[test]
    fn regex_derivartive() {
        let regex = Engine::new("a|b*", "derivative").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(regex.is_match("bb"));
        assert!(regex.is_match("bbb"));
        assert!(!regex.is_match("c"));

        let regex = Engine::new("a|b", "derivative").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("b"));
        assert!(!regex.is_match("c"));

        let regex = Engine::new("a*", "derivative").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aa"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match("b"));

        let regex = Engine::new("(p(erl|ython|hp)|ruby)", "derivative").unwrap();
        assert!(regex.is_match("perl"));
        assert!(regex.is_match("python"));
        assert!(regex.is_match("php"));
        assert!(regex.is_match("ruby"));
        assert!(!regex.is_match("rust"));

        let regex = Engine::new("a(b|)", "derivative").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("a"));
        assert!(!regex.is_match("abb"));

        let regex = Engine::new("ab(cd|)", "derivative").unwrap();
        assert!(regex.is_match("abcd"));
        assert!(regex.is_match("ab"));
        assert!(!regex.is_match("abc"));
        assert!(regex.is_match("abcd"));

        let regex = Engine::new("a+b", "derivative").unwrap();
        assert!(regex.is_match("ab"));
        assert!(regex.is_match("aab"));
        assert!(regex.is_match("aaab"));
        assert!(!regex.is_match("a"));
    }

    #[test]
    fn with_escape_derivative() {
        let regex = Engine::new(r"a\|b", "derivative").unwrap();
        assert!(regex.is_match("a|b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\*b", "derivative").unwrap();
        assert!(regex.is_match("a*b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\+b", "derivative").unwrap();
        assert!(regex.is_match("a+b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\?b", "derivative").unwrap();
        assert!(regex.is_match("a?b"));
        assert!(!regex.is_match("ab"));

        let regex = Engine::new(r"a\|b\*", "derivative").unwrap();
        assert!(regex.is_match("a|b*"));
        assert!(!regex.is_match("ab"));
    }

    #[test]
    fn nonascii_derivative() {
        let regex = Engine::new("あ|い*", "derivative").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("あ|い", "derivative").unwrap();
        assert!(regex.is_match("あ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("い*", "derivative").unwrap();
        assert!(regex.is_match(""));
        assert!(regex.is_match("い"));
        assert!(regex.is_match("いい"));
        assert!(regex.is_match("いいい"));
        assert!(!regex.is_match("う"));

        let regex = Engine::new("(ぱ(あ|い|う)|え)", "derivative").unwrap();
        assert!(regex.is_match("ぱあ"));
        assert!(regex.is_match("ぱい"));
        assert!(regex.is_match("ぱう"));
        assert!(regex.is_match("え"));
        assert!(!regex.is_match("お"));

        let regex = Engine::new("い(あ|)", "derivative").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("い"));
        assert!(!regex.is_match("いあい"));

        let regex = Engine::new("いあ(うえ|)", "derivative").unwrap();
        assert!(regex.is_match("いあうえ"));
        assert!(regex.is_match("いあ"));
        assert!(!regex.is_match("いあう"));
        assert!(regex.is_match("いあうえ"));

        let regex = Engine::new("い+あ", "derivative").unwrap();
        assert!(regex.is_match("いあ"));
        assert!(regex.is_match("いいあ"));
        assert!(regex.is_match("いいいあ"));
        assert!(!regex.is_match("い"));

        let regex = Engine::new("正規表現(太郎|次郎)", "derivative").unwrap();
        assert!(regex.is_match("正規表現太郎"));
        assert!(regex.is_match("正規表現次郎"));
        assert!(!regex.is_match("正規表現三郎"));

        let regex = Engine::new("あい|♥", "derivative").unwrap();
        assert!(regex.is_match("あい"));
        assert!(regex.is_match("♥"));
        assert!(!regex.is_match("♡"));
        assert!(!regex.is_match("👎️"));

        let regex = Engine::new("ගවයා|ng'ombe", "derivative").unwrap();
        assert!(regex.is_match("ගවයා"));
        assert!(regex.is_match("ng'ombe"));
        assert!(!regex.is_match("ගවයාng'ombe"));

        let regex = Engine::new("(පරිගණකය)*", "derivative").unwrap();
        assert!(regex.is_match("පරිගණකය"));
        assert!(regex.is_match(""));
    }

    #[test]
    fn invalid_derivative() {
        for test in ["a(b", "*", ")c", "+"] {
            let regex = Engine::new(test, "derivative");
            assert!(regex.is_err());
        }
    }

    #[test]
    fn invalid_method_name() {
        let regex = Engine::new("a", "正規表現太郎");
        assert!(regex.is_err());
    }
}
