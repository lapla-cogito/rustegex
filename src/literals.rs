const MIN_PREFIX_BYTES: usize = 3;
const MIN_SHORT_PREFIX_BYTES: usize = 2;
const MAX_LITERALS: usize = 32;
const MAX_FIRST_BYTES: usize = 3;

#[derive(Debug)]
pub(crate) enum LiteralSearch {
    Single(Box<[u8]>),
    Packed(aho_corasick::packed::Searcher),
    Multi(aho_corasick::AhoCorasick),
}

impl LiteralSearch {
    fn from_literals(lits: Vec<Vec<u8>>) -> Option<Self> {
        if lits.len() == 1 {
            return Some(LiteralSearch::Single(
                lits.into_iter().next().unwrap().into(),
            ));
        }
        if let Some(packed) = aho_corasick::packed::Searcher::new(&lits) {
            return Some(LiteralSearch::Packed(packed));
        }
        let ac = aho_corasick::AhoCorasick::new(&lits).ok()?;
        Some(LiteralSearch::Multi(ac))
    }

    fn find_first(&self, haystack: &[u8]) -> Option<usize> {
        match self {
            LiteralSearch::Single(lit) => memchr::memmem::find(haystack, lit),
            LiteralSearch::Packed(searcher) => searcher.find(haystack).map(|m| m.start()),
            LiteralSearch::Multi(ac) => ac.find(haystack).map(|m| m.start()),
        }
    }

    #[cfg(test)]
    fn find_candidates(&self, haystack: &[u8], mut f: impl FnMut(usize) -> bool) -> bool {
        match self {
            LiteralSearch::Single(lit) => {
                for at in memchr::memmem::find_iter(haystack, lit) {
                    if f(at) {
                        return true;
                    }
                }
                false
            }
            LiteralSearch::Packed(searcher) => {
                for m in searcher.find_iter(haystack) {
                    if f(m.start()) {
                        return true;
                    }
                }
                false
            }
            LiteralSearch::Multi(ac) => {
                for m in ac.find_iter(haystack) {
                    if f(m.start()) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

/// How a prefilter constrains partial search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefilterRole {
    /// Hits are possible match-start positions.
    Prefix,
    /// Hits only prove a required substring exists.
    Required,
    /// Sparse start-byte candidates.
    StartByte,
}

#[derive(Debug)]
pub enum Prefilter {
    Prefix(LiteralSearch),
    Required {
        search: LiteralSearch,
        /// When true, a match can start at the required-literal hit.
        anchor_at_hit: bool,
    },
    Bytes {
        needles: [u8; 3],
        len: u8,
    },
}

impl Prefilter {
    pub fn role(&self) -> PrefilterRole {
        match self {
            Prefilter::Prefix(_) => PrefilterRole::Prefix,
            Prefilter::Required { .. } => PrefilterRole::Required,
            Prefilter::Bytes { .. } => PrefilterRole::StartByte,
        }
    }

    pub fn required_anchor_at_hit(&self) -> bool {
        matches!(
            self,
            Prefilter::Required {
                anchor_at_hit: true,
                ..
            }
        )
    }

    /// Byte offset of the leftmost prefilter hit.
    pub fn find_first(&self, haystack: &[u8]) -> Option<usize> {
        match self {
            Prefilter::Prefix(s) | Prefilter::Required { search: s, .. } => s.find_first(haystack),
            Prefilter::Bytes { needles, len } => match len {
                1 => memchr::memchr(needles[0], haystack),
                2 => memchr::memchr2(needles[0], needles[1], haystack),
                3 => memchr::memchr3(needles[0], needles[1], needles[2], haystack),
                _ => None,
            },
        }
    }

    #[cfg(test)]
    pub fn find_candidates(&self, haystack: &[u8], f: impl FnMut(usize) -> bool) -> bool {
        match self {
            Prefilter::Prefix(s) => s.find_candidates(haystack, f),
            Prefilter::Required {
                search: s,
                anchor_at_hit: true,
            } => s.find_candidates(haystack, f),
            Prefilter::Required {
                anchor_at_hit: false,
                ..
            }
            | Prefilter::Bytes { .. } => false,
        }
    }
}

pub fn build_prefilter(ast: &crate::parser::AstNode) -> Option<Prefilter> {
    if let Some(lits) = prefix_literals(ast, MIN_PREFIX_BYTES) {
        return LiteralSearch::from_literals(lits).map(Prefilter::Prefix);
    }
    if let Some((lits, anchor_at_hit)) = required_literals(ast) {
        return LiteralSearch::from_literals(lits).map(|search| Prefilter::Required {
            search,
            anchor_at_hit,
        });
    }
    if let Some(lits) = prefix_literals(ast, MIN_SHORT_PREFIX_BYTES) {
        return LiteralSearch::from_literals(lits).map(Prefilter::Prefix);
    }
    first_byte_prefilter(ast)
}

fn prefix_literals(ast: &crate::parser::AstNode, min_bytes: usize) -> Option<Vec<Vec<u8>>> {
    let set = prefix_set(ast)?;
    if set.is_empty() || set.len() > MAX_LITERALS {
        return None;
    }
    if set.iter().any(|s| s.is_empty() || s.len() < min_bytes) {
        return None;
    }
    Some(set.into_iter().map(String::into_bytes).collect())
}

fn first_byte_prefilter(ast: &crate::parser::AstNode) -> Option<Prefilter> {
    let set = prefix_set(ast)?;
    if set.is_empty() || set.iter().any(String::is_empty) {
        return None;
    }
    let mut needles = [0u8; 3];
    let mut len = 0u8;
    for s in &set {
        let &b = s.as_bytes().first()?;
        if !needles[..len as usize].contains(&b) {
            if len as usize >= MAX_FIRST_BYTES {
                return None;
            }
            needles[len as usize] = b;
            len += 1;
        }
    }
    if len == 0 {
        None
    } else {
        Some(Prefilter::Bytes { needles, len })
    }
}

/// Longest literal that every match must contain.
fn required_literals(ast: &crate::parser::AstNode) -> Option<(Vec<Vec<u8>>, bool)> {
    let (set, anchor_at_hit) = required_set(ast)?;
    if set.is_empty() || set.len() > MAX_LITERALS {
        return None;
    }
    if set
        .iter()
        .any(|s| s.is_empty() || s.len() < MIN_PREFIX_BYTES)
    {
        return None;
    }
    Some((
        set.into_iter().map(String::into_bytes).collect(),
        anchor_at_hit,
    ))
}

fn is_nullable_part(ast: &crate::parser::AstNode) -> bool {
    match ast {
        crate::parser::AstNode::Epsilon
        | crate::parser::AstNode::Star(_)
        | crate::parser::AstNode::Question(_) => true,
        crate::parser::AstNode::Or(left, right) => {
            is_nullable_part(left) || is_nullable_part(right)
        }
        crate::parser::AstNode::Seq(left, right) => {
            is_nullable_part(left) && is_nullable_part(right)
        }
        _ => false,
    }
}

fn flatten_seq<'a>(ast: &'a crate::parser::AstNode, out: &mut Vec<&'a crate::parser::AstNode>) {
    match ast {
        crate::parser::AstNode::Seq(left, right) => {
            flatten_seq(left, out);
            flatten_seq(right, out);
        }
        other => out.push(other),
    }
}

/// Forced contiguous char runs that every match must contain.
fn required_set(ast: &crate::parser::AstNode) -> Option<(Vec<String>, bool)> {
    match ast {
        crate::parser::AstNode::Or(left, right) => {
            let (l, la) = required_set(left)?;
            let (r, ra) = required_set(right)?;
            // Anchoring only if both sides can; union of literals.
            Some((union_sets(l, r)?, la && ra))
        }
        crate::parser::AstNode::Class(_) | crate::parser::AstNode::Empty => None,
        crate::parser::AstNode::Epsilon
        | crate::parser::AstNode::Star(_)
        | crate::parser::AstNode::Question(_) => Some((vec![String::new()], true)),
        crate::parser::AstNode::Char(c) => Some((vec![c.to_string()], true)),
        crate::parser::AstNode::Plus(inner) => required_set(inner),
        crate::parser::AstNode::Seq(_, _) => {
            let mut parts = Vec::new();
            flatten_seq(ast, &mut parts);
            required_runs_from_parts(&parts)
        }
    }
}

fn extend_run_from_parts(parts: &[&crate::parser::AstNode], start: usize) -> String {
    let mut cur = String::new();
    for part in &parts[start..] {
        match part {
            crate::parser::AstNode::Char(c) => cur.push(*c),
            crate::parser::AstNode::Plus(inner) => {
                if let crate::parser::AstNode::Char(c) = inner.as_ref() {
                    cur.push(*c);
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    cur
}

fn required_runs_from_parts(parts: &[&crate::parser::AstNode]) -> Option<(Vec<String>, bool)> {
    let mut runs: Vec<String> = Vec::new();
    let mut cur = String::new();

    let flush = |cur: &mut String, runs: &mut Vec<String>| {
        if !cur.is_empty() {
            runs.push(std::mem::take(cur));
        }
    };

    for part in parts {
        match part {
            crate::parser::AstNode::Char(c) => cur.push(*c),
            crate::parser::AstNode::Plus(inner) => match inner.as_ref() {
                crate::parser::AstNode::Char(c) => {
                    cur.push(*c);
                }
                _ => {
                    flush(&mut cur, &mut runs);
                    let (inner_runs, _) = required_set(inner)?;
                    for s in inner_runs {
                        if !s.is_empty() {
                            runs.push(s);
                        }
                    }
                }
            },
            crate::parser::AstNode::Star(_)
            | crate::parser::AstNode::Question(_)
            | crate::parser::AstNode::Epsilon => {
                flush(&mut cur, &mut runs);
            }
            crate::parser::AstNode::Or(_, _) => {
                flush(&mut cur, &mut runs);
                let (inner_runs, _) = required_set(part)?;
                for s in inner_runs {
                    if !s.is_empty() {
                        runs.push(s);
                    }
                }
            }
            crate::parser::AstNode::Class(_) => {
                flush(&mut cur, &mut runs);
                return None;
            }
            crate::parser::AstNode::Seq(_, _) => {
                flush(&mut cur, &mut runs);
                let (inner_runs, _) = required_set(part)?;
                for s in inner_runs {
                    if !s.is_empty() {
                        runs.push(s);
                    }
                }
            }
            crate::parser::AstNode::Empty => return None,
        }
    }
    flush(&mut cur, &mut runs);

    if runs.is_empty() {
        return Some((vec![String::new()], true));
    }

    let best_len = runs.iter().map(String::len).max().unwrap_or(0);
    let best: Vec<String> = runs.into_iter().filter(|s| s.len() == best_len).collect();
    let mut best = best;
    best.dedup();
    if best.len() > MAX_LITERALS {
        return None;
    }

    let mut i = 0;
    while i < parts.len() && is_nullable_part(parts[i]) {
        i += 1;
    }
    let leading = extend_run_from_parts(parts, i);
    let anchor_at_hit = best
        .iter()
        .any(|s| leading == *s || leading.starts_with(s.as_str()));

    Some((best, anchor_at_hit))
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
    fn short_alt_rejected_as_long_prefix() {
        assert_eq!(lits("ab|cd"), None);
        assert_eq!(lits("abc|d"), None);
    }

    #[test]
    fn short_prefix_prefilter() {
        let pf = build_prefilter(&parse("ab|cd")).unwrap();
        assert_eq!(pf.role(), PrefilterRole::Prefix);
        assert_eq!(pf.find_first(b"xxabxx"), Some(2));
        assert_eq!(pf.find_first(b"xxcdxx"), Some(2));
        assert!(pf.find_first(b"xxxx").is_none());
    }

    #[test]
    fn first_byte_prefilter_a_plus_b() {
        let pf = build_prefilter(&parse("a+b")).unwrap();
        assert_eq!(pf.role(), PrefilterRole::StartByte);
        assert_eq!(pf.find_first(b"xxxabc"), Some(3));
        assert!(pf.find_first(b"xxxxx").is_none());
    }

    #[test]
    fn required_literal_after_star() {
        let pf = build_prefilter(&parse("a*bcd")).unwrap();
        assert_eq!(pf.role(), PrefilterRole::Required);
        assert!(pf.required_anchor_at_hit());
        assert_eq!(pf.find_first(b"xxxbcdyyy"), Some(3));
        assert!(pf.find_first(b"xxx").is_none());
    }

    #[test]
    fn required_does_not_false_negative_with_prefix() {
        let pf = build_prefilter(&parse("a+bcd")).unwrap();
        assert_eq!(pf.role(), PrefilterRole::Required);
        assert!(pf.required_anchor_at_hit());
        assert_eq!(pf.find_first(b"aaabcd"), Some(2));
        assert_eq!(&b"aaabcd"[2..], b"abcd");
    }

    #[test]
    fn required_no_anchor_when_prefix_before_run() {
        let pf = build_prefilter(&parse("xa*bcd")).unwrap();
        assert_eq!(pf.role(), PrefilterRole::Required);
        assert!(!pf.required_anchor_at_hit());
        assert_eq!(pf.find_first(b"xxbcd"), Some(2));
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

    #[test]
    fn find_candidates_visits_all() {
        let pf = build_prefilter(&parse("abc")).unwrap();
        let mut hits = Vec::new();
        pf.find_candidates(b"abcXabc", |at| {
            hits.push(at);
            false
        });
        assert_eq!(hits, vec![0, 4]);
    }
}
