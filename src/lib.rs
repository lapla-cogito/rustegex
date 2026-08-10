mod automaton;
mod charclass;
mod derivative;
mod error;
mod lexer;
mod literals;
mod parser;
mod vm;

pub use error::{Error, Result};

#[global_allocator]
static MIMALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug)]
enum Regex {
    Dfa {
        dfa: automaton::dfa::Dfa,
        unanchored: automaton::dfa::Dfa,
    },
    Vm {
        vm: vm::Vm,
    },
    Derivative {
        derivative: derivative::Derivative,
    },
}

#[derive(Debug)]
pub struct Engine {
    regex: Regex,
    literal: Option<Box<[u8]>>,
    prefilter: Option<literals::Prefilter>,
}

impl Engine {
    pub fn new(input: &str, method: &'static str) -> Result<Engine> {
        let mut lexer = lexer::Lexer::new(input);
        let mut parser = parser::Parser::new(&mut lexer);
        let ast = parser.parse()?;
        let literal = ast.as_literal().map(|s| s.into_bytes().into_boxed_slice());
        let prefilter = if literal.is_none() {
            literals::build_prefilter(&ast)
        } else {
            None
        };

        match method {
            "dfa" => {
                let mut nfa_state = automaton::nfa::NfaState::new();
                let nfa = automaton::nfa::Nfa::new_from_node(ast, &mut nfa_state)?;
                let dfa = automaton::dfa::Dfa::from_nfa(&nfa);
                let unanchored =
                    automaton::dfa::Dfa::from_nfa(&nfa.with_unanchored_start(&mut nfa_state));

                Ok(Engine {
                    regex: Regex::Dfa { dfa, unanchored },
                    literal,
                    prefilter,
                })
            }
            "vm" => {
                let vm = vm::Vm::new(ast)?;

                Ok(Engine {
                    regex: Regex::Vm { vm },
                    literal,
                    prefilter,
                })
            }
            "derivative" => {
                let derivative = derivative::Derivative::new(ast);

                Ok(Engine {
                    regex: Regex::Derivative { derivative },
                    literal,
                    prefilter,
                })
            }
            _ => Err(Error::InvalidMethod(method.to_string())),
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        match &self.regex {
            Regex::Dfa { dfa, .. } => dfa.is_match(input),
            Regex::Vm { vm } => vm.is_match(input),
            Regex::Derivative { derivative } => {
                if input.is_empty() {
                    return derivative.is_empty_match();
                }
                derivative.is_match(input)
            }
        }
    }

    pub fn is_partial_match(&self, input: &str) -> bool {
        if let Some(lit) = &self.literal {
            return if lit.is_empty() {
                true
            } else {
                memchr::memmem::find(input.as_bytes(), lit).is_some()
            };
        }

        let input = if let Some(prefilter) = &self.prefilter {
            let Some(at) = prefilter.find_first(input.as_bytes()) else {
                return false;
            };
            &input[at..]
        } else {
            input
        };

        match &self.regex {
            Regex::Dfa { unanchored, .. } => unanchored.find_accept(input),
            Regex::Vm { vm } => vm.is_partial_match(input),
            Regex::Derivative { derivative } => derivative.is_partial_match(input),
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
        assert!(!regex.is_match("b"));
        assert!(!regex.is_match(""));

        let regex = Engine::new("a+", "dfa").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match(""));
        assert!(!regex.is_match("b"));
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
        assert!(!regex.is_match("b"));
        assert!(!regex.is_match(""));

        let regex = Engine::new("a+", "vm").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match(""));
        assert!(!regex.is_match("b"));
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
        assert!(!regex.is_match("b"));
        assert!(!regex.is_match(""));

        let regex = Engine::new("a+", "derivative").unwrap();
        assert!(regex.is_match("a"));
        assert!(regex.is_match("aaa"));
        assert!(!regex.is_match(""));
        assert!(!regex.is_match("b"));
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

    fn assert_match_all(method: &'static str, pattern: &str, yes: &[&str], no: &[&str]) {
        let engine = Engine::new(pattern, method).unwrap();
        for input in yes {
            assert!(
                engine.is_match(input),
                "method={method} pattern={pattern:?} input={input:?}"
            );
        }
        for input in no {
            assert!(
                !engine.is_match(input),
                "method={method} pattern={pattern:?} input={input:?}"
            );
        }
    }

    #[test]
    fn metacharacters_dfa() {
        let cases = [
            (r"\d", &["0", "9"] as &[&str], &["", "a"] as &[&str]),
            (
                r"\w",
                &["a", "Z", "_", "9"] as &[&str],
                &["", "-", "♥"] as &[&str],
            ),
            (r"\s", &[" ", "\t"] as &[&str], &["", "a"] as &[&str]),
            (
                r"\d+",
                &["0", "42", "999"] as &[&str],
                &["a", "4a"] as &[&str],
            ),
            (
                r"\w+",
                &["a", "Z9", "foo_bar"] as &[&str],
                &["-", "♥"] as &[&str],
            ),
            (r"\s+", &[" ", "\t\n"] as &[&str], &["a", "a b"] as &[&str]),
            (
                "a.b",
                &["a b", "a\tb", "a0b"] as &[&str],
                &["ab", "a\nb"] as &[&str],
            ),
        ];
        for (pattern, yes, no) in cases {
            assert_match_all("dfa", pattern, yes, no);
        }
    }

    #[test]
    fn metacharacters_vm() {
        let cases = [
            (r"\d", &["0", "9"] as &[&str], &["", "a"] as &[&str]),
            (
                r"\w",
                &["a", "Z", "_", "9"] as &[&str],
                &["", "-", "♥"] as &[&str],
            ),
            (r"\s", &[" ", "\t"] as &[&str], &["", "a"] as &[&str]),
            (
                r"\d+",
                &["0", "42", "999"] as &[&str],
                &["a", "4a"] as &[&str],
            ),
            (
                r"\w+",
                &["a", "Z9", "foo_bar"] as &[&str],
                &["-", "♥"] as &[&str],
            ),
            (r"\s+", &[" ", "\t\n"] as &[&str], &["a", "a b"] as &[&str]),
            (
                "a.b",
                &["a b", "a\tb", "a0b"] as &[&str],
                &["ab", "a\nb"] as &[&str],
            ),
        ];
        for (pattern, yes, no) in cases {
            assert_match_all("vm", pattern, yes, no);
        }
    }

    #[test]
    fn metacharacters_derivative() {
        let cases = [
            (r"\d", &["0", "9"] as &[&str], &["", "a"] as &[&str]),
            (
                r"\w",
                &["a", "Z", "_", "9"] as &[&str],
                &["", "-", "♥"] as &[&str],
            ),
            (r"\s", &[" ", "\t"] as &[&str], &["", "a"] as &[&str]),
            (
                r"\d+",
                &["0", "42", "999"] as &[&str],
                &["a", "4a"] as &[&str],
            ),
            (
                r"\w+",
                &["a", "Z9", "foo_bar"] as &[&str],
                &["-", "♥"] as &[&str],
            ),
            (r"\s+", &[" ", "\t\n"] as &[&str], &["a", "a b"] as &[&str]),
            (
                "a.b",
                &["a b", "a\tb", "a0b"] as &[&str],
                &["ab", "a\nb"] as &[&str],
            ),
        ];
        for (pattern, yes, no) in cases {
            assert_match_all("derivative", pattern, yes, no);
        }
    }

    fn assert_partial_all(method: &'static str, pattern: &str, yes: &[&str], no: &[&str]) {
        let engine = Engine::new(pattern, method).unwrap();
        let re = regex::Regex::new(pattern).unwrap();
        for input in yes {
            assert!(
                engine.is_partial_match(input),
                "method={method} pattern={pattern:?} input={input:?} expected partial match"
            );
            assert!(
                re.is_match(input),
                "oracle regex disagrees (yes) pattern={pattern:?} input={input:?}"
            );
        }
        for input in no {
            assert!(
                !engine.is_partial_match(input),
                "method={method} pattern={pattern:?} input={input:?} expected no partial match"
            );
            assert!(
                !re.is_match(input),
                "oracle regex disagrees (no) pattern={pattern:?} input={input:?}"
            );
        }
    }

    fn partial_cases() -> [(
        &'static str,
        &'static [&'static str],
        &'static [&'static str],
    ); 10] {
        [
            (
                "abc",
                &["abc", "xyzabcdef", "ababc", "bar\nfooabc"][..],
                &["ab", "xyz", ""][..],
            ),
            (
                "a+b",
                &["ab", "xxaaabxx", "aaab"][..],
                &["aaa", "b", ""][..],
            ),
            (
                "a+b|ac",
                &["aaac", "xxaaabxx", "yac", "ac"][..],
                &["aaa", ""][..],
            ),
            ("a*", &["", "xyz", "aaa"][..], &[][..]),
            (
                "(p(erl|ython|hp)|ruby)",
                &["perl", "I write python code", "ruby is fun"][..],
                &["I write rust code", ""][..],
            ),
            (
                r"\d+",
                &["0", "age: 42 years", "x9"][..],
                &["no digits", ""][..],
            ),
            (
                "a.b",
                &["aXb", "xxaXbxx", "a b"][..],
                &["ab", "a\nb", ""][..],
            ),
            (
                "foo",
                &["foo", "bar\nfoo", "fffoo"][..],
                &["fo", "bar\nf o"][..],
            ),
            (
                "正規表現(太郎|次郎)",
                &["正規表現太郎", "私は正規表現次郎です"][..],
                &["正規表現三郎", ""][..],
            ),
            ("aa", &["aa", "aaa", "baab"][..], &["a", ""][..]),
        ]
    }

    #[test]
    fn partial_dfa() {
        for (pattern, yes, no) in partial_cases() {
            assert_partial_all("dfa", pattern, yes, no);
        }
    }

    #[test]
    fn partial_vm() {
        for (pattern, yes, no) in partial_cases() {
            assert_partial_all("vm", pattern, yes, no);
        }
    }

    #[test]
    fn partial_derivative() {
        for (pattern, yes, no) in partial_cases() {
            assert_partial_all("derivative", pattern, yes, no);
        }
    }

    #[test]
    fn partial_does_not_change_full_match() {
        for method in ["dfa", "vm", "derivative"] {
            let engine = Engine::new("abc", method).unwrap();
            assert!(engine.is_match("abc"));
            assert!(!engine.is_match("xabc"));
            assert!(!engine.is_match("abcx"));
            assert!(engine.is_partial_match("xabc"));
            assert!(engine.is_partial_match("abcx"));
        }
    }

    #[test]
    fn partial_prefilter_alt_in_lorem() {
        let hay_yes = format!("{}python", "lorem ipsum ".repeat(200));
        let hay_no = "lorem ipsum ".repeat(200);
        for method in ["dfa", "vm", "derivative"] {
            let engine = Engine::new("(p(erl|ython|hp)|ruby)", method).unwrap();
            assert!(engine.prefilter.is_some(), "method={method}");
            assert!(engine.is_partial_match(&hay_yes), "method={method}");
            assert!(!engine.is_partial_match(&hay_no), "method={method}");
            assert!(engine.is_partial_match("xxphpxx"), "method={method}");
            assert!(engine.is_partial_match("ruby"), "method={method}");
        }
    }

    #[test]
    fn partial_prefilter_plus_suffix() {
        for method in ["dfa", "vm", "derivative"] {
            let engine = Engine::new("abc+d", method).unwrap();
            assert!(engine.prefilter.is_some(), "method={method}");
            assert!(engine.is_partial_match("xxabcabcdyy"), "method={method}");
            assert!(engine.is_partial_match("abcd"), "method={method}");
            assert!(!engine.is_partial_match("abcabc"), "method={method}");
            assert!(!engine.is_partial_match("abxd"), "method={method}");
        }
    }
}
