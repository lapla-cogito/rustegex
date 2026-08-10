const MIN_PREFIX_BYTES: usize = 3;
const MAX_LITERALS: usize = 32;

#[derive(Debug)]
pub enum Prefilter {
    Single(Box<[u8]>),
    Packed(aho_corasick::packed::Searcher),
    Multi(aho_corasick::AhoCorasick),
}

impl Prefilter {
    /// Byte offset of the leftmost occurrence of any extracted prefix.
    pub fn find_first(&self, haystack: &[u8]) -> Option<usize> {
        match self {
            Prefilter::Single(lit) => memchr::memmem::find(haystack, lit),
            Prefilter::Packed(searcher) => searcher.find(haystack).map(|m| m.start()),
            Prefilter::Multi(ac) => ac.find(haystack).map(|m| m.start()),
        }
    }
}

pub fn build_prefilter(ast: &crate::parser::AstNode) -> Option<Prefilter> {
    let lits = prefix_literals(ast)?;
    if lits.len() == 1 {
        return Some(Prefilter::Single(lits.into_iter().next().unwrap().into()));
    }
    if let Some(packed) = aho_corasick::packed::Searcher::new(&lits) {
        return Some(Prefilter::Packed(packed));
    }
    let ac = aho_corasick::AhoCorasick::new(&lits).ok()?;
    Some(Prefilter::Multi(ac))
}

fn prefix_literals(ast: &crate::parser::AstNode) -> Option<Vec<Vec<u8>>> {
    let set = prefix_set(ast)?;
    if set.is_empty() || set.len() > MAX_LITERALS {
        return None;
    }
    if set
        .iter()
        .any(|s| s.is_empty() || s.len() < MIN_PREFIX_BYTES)
    {
        return None;
    }
    Some(set.into_iter().map(String::into_bytes).collect())
}

fn is_finite_literal_expr(ast: &crate::parser::AstNode) -> bool {
    match ast {
        crate::parser::AstNode::Char(_) | crate::parser::AstNode::Epsilon => true,
        crate::parser::AstNode::Class(_)
        | crate::parser::AstNode::Star(_)
        | crate::parser::AstNode::Plus(_)
        | crate::parser::AstNode::Empty => false,
        crate::parser::AstNode::Question(inner) => is_finite_literal_expr(inner),
        crate::parser::AstNode::Or(left, right) | crate::parser::AstNode::Seq(left, right) => {
            is_finite_literal_expr(left) && is_finite_literal_expr(right)
        }
    }
}

fn prefix_set(ast: &crate::parser::AstNode) -> Option<Vec<String>> {
    match ast {
        crate::parser::AstNode::Empty => Some(Vec::new()),
        crate::parser::AstNode::Epsilon => Some(vec![String::new()]),
        crate::parser::AstNode::Char(c) => Some(vec![c.to_string()]),
        crate::parser::AstNode::Class(_) => None,
        crate::parser::AstNode::Star(_) => Some(vec![String::new()]),
        crate::parser::AstNode::Plus(inner) => prefix_set(inner),
        crate::parser::AstNode::Question(inner) => {
            let mut set = prefix_set(inner)?;
            if !set.iter().any(|s| s.is_empty()) {
                set.push(String::new());
            }
            Some(set)
        }
        crate::parser::AstNode::Or(left, right) => {
            union_sets(prefix_set(left)?, prefix_set(right)?)
        }
        crate::parser::AstNode::Seq(left, right) => {
            let left_set = prefix_set(left)?;
            if left_set.is_empty() {
                return Some(left_set);
            }
            if left_set.iter().all(String::is_empty) {
                if is_finite_literal_expr(left) {
                    return prefix_set(right);
                }
                return Some(left_set);
            }
            if is_finite_literal_expr(left) {
                match prefix_set(right) {
                    None => Some(left_set),
                    Some(right_set) => concat_sets(left_set, right_set),
                }
            } else {
                Some(left_set)
            }
        }
    }
}

fn union_sets(mut left: Vec<String>, right: Vec<String>) -> Option<Vec<String>> {
    for s in right {
        if !left.iter().any(|e| e == &s) {
            left.push(s);
        }
    }
    if left.len() > MAX_LITERALS {
        None
    } else {
        Some(left)
    }
}

fn concat_sets(left: Vec<String>, right: Vec<String>) -> Option<Vec<String>> {
    let n = left.len().saturating_mul(right.len());
    if n > MAX_LITERALS {
        return Some(left);
    }
    let mut out = Vec::with_capacity(n);
    for l in &left {
        for r in &right {
            let mut s = String::with_capacity(l.len() + r.len());
            s.push_str(l);
            s.push_str(r);
            if !out.iter().any(|e| e == &s) {
                out.push(s);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(pattern: &str) -> crate::parser::AstNode {
        let mut lexer = crate::lexer::Lexer::new(pattern);
        let mut parser = crate::parser::Parser::new(&mut lexer);
        parser.parse().unwrap()
    }

    fn lits(pattern: &str) -> Option<Vec<String>> {
        let set = prefix_set(&parse(pattern))?;
        if set.is_empty()
            || set
                .iter()
                .any(|s| s.is_empty() || s.len() < MIN_PREFIX_BYTES)
        {
            return None;
        }
        if set.len() > MAX_LITERALS {
            return None;
        }
        let mut set = set;
        set.sort();
        Some(set)
    }

    #[test]
    fn alt_of_words() {
        assert_eq!(
            lits("(p(erl|ython|hp)|ruby)"),
            Some(vec![
                "perl".into(),
                "php".into(),
                "python".into(),
                "ruby".into()
            ])
        );
    }

    #[test]
    fn simple_alt() {
        assert_eq!(lits("abc|def"), Some(vec!["abc".into(), "def".into()]));
    }

    #[test]
    fn short_alt_rejected() {
        assert_eq!(lits("ab|cd"), None);
        assert_eq!(lits("abc|d"), None);
    }

    #[test]
    fn concat_after_alt() {
        assert_eq!(lits("a(bc|de)f"), Some(vec!["abcf".into(), "adef".into()]));
    }

    #[test]
    fn question_expands() {
        assert_eq!(lits("ab?cd"), Some(vec!["abcd".into(), "acd".into()]));
    }

    #[test]
    fn star_blocks_prefix() {
        assert_eq!(lits("a*bcd"), None);
        assert_eq!(lits("ab*cd"), None);
    }

    #[test]
    fn plus_stops_extension() {
        assert_eq!(lits("abc+d"), Some(vec!["abc".into()]));
        assert_eq!(lits("a+bcd"), None);
    }

    #[test]
    fn class_unknown() {
        assert_eq!(lits(r"abc\d"), Some(vec!["abc".into()]));
        assert_eq!(lits(r"\dabc"), None);
    }

    #[test]
    fn unicode_alt() {
        assert_eq!(lits("太郎|次郎"), Some(vec!["太郎".into(), "次郎".into()]));
    }

    #[test]
    fn prefilter_skips_lorem() {
        let ast = parse("(p(erl|ython|hp)|ruby)");
        let pf = build_prefilter(&ast).unwrap();
        let hay = format!("{}python", "lorem ipsum ".repeat(100));
        let at = pf.find_first(hay.as_bytes()).unwrap();
        assert_eq!(&hay[at..], "python");
        assert!(pf.find_first(b"lorem ipsum lorem ipsum").is_none());
    }
}
