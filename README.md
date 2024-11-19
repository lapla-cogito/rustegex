# rustegex

A hobby regular expression engine in Rust.

## example

```rust
let regex = RustRegex::new("a|b*").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

let regex = RustRegex::new(r"ab(cd|)").unwrap();
assert!(regex.is_match("abcd"));
assert!(regex.is_match("ab"));
assert!(!regex.is_match("abc"));
assert!(regex.is_match("abcd"));
```

## test

```bash
$ cargo test
```
