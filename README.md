# rustegex

A hobby regular expression engine in Rust.

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
```

## test

```bash
$ cargo test
```
