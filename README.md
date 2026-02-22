# rustegex

A hobby regular expression engine in Rust.

- Supports 3 types of engines:
    - DFA-based engine
        - Converts regex to NFA, then NFA to DFA via subset construction
        - Matching is a single linear scan over the input with no backtracking
    - VM-based engine
        - Pike VM (Thompson NFA lockstep simulation)
        - Processes all active NFA states simultaneously per input character
    - Derivative-based engine
        - Matches by repeatedly computing Brzozowski's derivative of the pattern
- All engines currently support `*`, `+`, `?`, `|`, `()`, `\` (escape), and Unicode characters.

## Example

DFA-based:

```rust
let regex = rustegex::Engine::new("a|b*", "dfa").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

let regex = rustegex::Engine::new("ab(cd|)", "dfa").unwrap();
assert!(regex.is_match("abcd"));
assert!(regex.is_match("ab"));
assert!(!regex.is_match("abc"));
assert!(regex.is_match("abcd"));

let regex = rustegex::Engine::new("a+b", "dfa").unwrap();
assert!(regex.is_match("ab"));
assert!(regex.is_match("aab"));
assert!(regex.is_match("aaab"));
assert!(!regex.is_match("a"));

let regex = rustegex::Engine::new(r"a\|b\*", "dfa").unwrap();
assert!(regex.is_match("a|b*"));
assert!(!regex.is_match("ab"));

let regex = rustegex::Engine::new("正規表現(太郎|次郎)", "dfa").unwrap();
assert!(regex.is_match("正規表現太郎"));
assert!(regex.is_match("正規表現次郎"));
assert!(!regex.is_match("正規表現三郎"));
```

VM-based:

```rust
let regex = rustegex::Engine::new("a|b*", "vm").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

let regex = rustegex::Engine::new("ab(cd|)", "vm").unwrap();
assert!(regex.is_match("abcd"));
assert!(regex.is_match("ab"));
assert!(!regex.is_match("abc"));
assert!(regex.is_match("abcd"));

let regex = rustegex::Engine::new("a+b", "vm").unwrap();
assert!(regex.is_match("ab"));
assert!(regex.is_match("aab"));
assert!(regex.is_match("aaab"));
assert!(!regex.is_match("a"));

let regex = rustegex::Engine::new(r"a\|b\*", "vm").unwrap();
assert!(regex.is_match("a|b*"));
assert!(!regex.is_match("ab"));

let regex = rustegex::Engine::new("正規表現(太郎|次郎)", "vm").unwrap();
assert!(regex.is_match("正規表現太郎"));
assert!(regex.is_match("正規表現次郎"));
assert!(!regex.is_match("正規表現三郎"));
```

Derivative-based:

```rust
let regex = rustegex::Engine::new("a|b*", "derivative").unwrap();
assert!(regex.is_match("a"));
assert!(regex.is_match("b"));
assert!(regex.is_match("bb"));
assert!(regex.is_match("bbb"));
assert!(!regex.is_match("c"));

let regex = rustegex::Engine::new("ab(cd|)", "derivative").unwrap();
assert!(regex.is_match("abcd"));
assert!(regex.is_match("ab"));
assert!(!regex.is_match("abc"));
assert!(regex.is_match("abcd"));

let regex = rustegex::Engine::new("a+b", "derivative").unwrap();
assert!(regex.is_match("ab"));
assert!(regex.is_match("aab"));
assert!(regex.is_match("aaab"));
assert!(!regex.is_match("a"));

let regex = rustegex::Engine::new(r"a\|b\*", "derivative").unwrap();
assert!(regex.is_match("a|b*"));
assert!(!regex.is_match("ab"));

let regex = rustegex::Engine::new("正規表現(太郎|次郎)", "derivative").unwrap();
assert!(regex.is_match("正規表現太郎"));
assert!(regex.is_match("正規表現次郎"));
assert!(!regex.is_match("正規表現三郎"));
```

## Test

```bash
$ cargo test
```

## Run Benchmarks

```bash
$ cargo bench
```

## License

MIT
