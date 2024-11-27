# rustegex

A hobby regular expression engine in Rust.

- supports Unicode characters
- engines
    - DFA-based engine
        - convert regex to NFA
        - convert NFA to DFA
    - VM-based engine
        - caching
    - Derivative-based engine

## example

DFA-based:

```rust
let regex = RustRegex::new("a|b*", "dfa").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

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

let regex = RustRegex::new(r"a\|b\*", "dfa").unwrap();
assert!(regex.is_match("a|b*"));
assert!(!regex.is_match("ab"));

let regex = RustRegex::new("正規表現(太郎|次郎)", "dfa").unwrap();
assert!(regex.is_match("正規表現太郎"));
assert!(regex.is_match("正規表現次郎"));
assert!(!regex.is_match("正規表現三郎"));
```

VM-based:

```rust
let regex = RustRegex::new("a|b*", "vm").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

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

let regex = RustRegex::new(r"a\|b\*", "vm").unwrap();
assert!(regex.is_match("a|b*"));
assert!(!regex.is_match("ab"));

let regex = RustRegex::new("正規表現(太郎|次郎)", "vm").unwrap();
assert!(regex.is_match("正規表現太郎"));
assert!(regex.is_match("正規表現次郎"));
assert!(!regex.is_match("正規表現三郎"));
```

Derivative-based:

```rust
let regex = RustRegex::new("a|b*", "derivative").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

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

let regex = RustRegex::new(r"a\|b\*", "derivative").unwrap();
assert!(regex.is_match("a|b*"));
assert!(!regex.is_match("ab"));

let regex = RustRegex::new("正規表現(太郎|次郎)", "derivative").unwrap();
assert!(regex.is_match("正規表現太郎"));
assert!(regex.is_match("正規表現次郎"));
assert!(!regex.is_match("正規表現三郎"));
```

## test

```bash
$ cargo test
```
