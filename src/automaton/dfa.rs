use foldhash::HashMapExt as _;

pub type DfaStateID = u64;
const DEAD: DfaStateID = DfaStateID::MAX;
const ACCEL_MIN_REMAINING: usize = 32;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Accel {
    loop_byte: Option<u8>,
    needles: [u8; 3],
    needle_len: u8,
}

impl Accel {
    fn is_enabled(self) -> bool {
        self.loop_byte.is_some() || self.needle_len > 0
    }

    fn memchr_fwd(&self, haystack: &[u8], at: usize) -> Option<usize> {
        if self.needle_len == 0 {
            return None;
        }
        let slice = &haystack[at..];
        let offset = match self.needle_len {
            1 => memchr::memchr(self.needles[0], slice)?,
            2 => memchr::memchr2(self.needles[0], self.needles[1], slice)?,
            3 => memchr::memchr3(self.needles[0], self.needles[1], self.needles[2], slice)?,
            _ => return None,
        };
        Some(at + offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dfa {
    start: DfaStateID,
    accepts: bit_set::BitSet,
    state_count: usize,
    ascii_table: Vec<DfaStateID>,
    unicode_table: Vec<foldhash::HashMap<char, DfaStateID>>,
    accels: Vec<Accel>,
}

impl Dfa {
    pub fn new(start: DfaStateID, accepts: bit_set::BitSet) -> Self {
        Dfa {
            start,
            accepts,
            state_count: 0,
            ascii_table: Vec::new(),
            unicode_table: Vec::new(),
            accels: Vec::new(),
        }
    }

    pub fn start(&self) -> DfaStateID {
        self.start
    }

    #[cfg(test)]
    pub fn accepts_contains(&self, state: DfaStateID) -> bool {
        self.accepts.contains(state as usize)
    }

    #[cfg(test)]
    pub fn accel(&self, state: DfaStateID) -> (Option<u8>, u8, [u8; 3]) {
        let accel = self.accels[state as usize];
        (accel.loop_byte, accel.needle_len, accel.needles)
    }

    #[cfg(test)]
    pub fn transitions(&self) -> std::collections::BTreeSet<(DfaStateID, char, DfaStateID)> {
        let mut result = std::collections::BTreeSet::new();
        for state in 0..self.state_count {
            for byte in 0u8..128 {
                let next = self.ascii_table[state * 128 + byte as usize];
                if next != DEAD {
                    result.insert((state as DfaStateID, byte as char, next));
                }
            }
            for (&c, &next) in &self.unicode_table[state] {
                result.insert((state as DfaStateID, c, next));
            }
        }
        result
    }

    pub fn from_nfa(nfa: &crate::automaton::nfa::Nfa) -> Self {
        let mut dfa_states = foldhash::HashMap::new();
        let mut queue = std::collections::VecDeque::new();

        let mut start_bitset = bit_set::BitSet::new();
        start_bitset.insert(nfa.start() as usize);
        let start_closure_bitset = nfa.epsilon_closure_with_bitset(&start_bitset);

        let start_states: std::collections::BTreeSet<_> = start_closure_bitset
            .iter()
            .map(|s| s as crate::automaton::nfa::NfaStateID)
            .collect();

        let start_id = dfa_states.len() as DfaStateID;
        dfa_states.insert(start_states.clone(), start_id);
        queue.push_back(start_states);

        let mut dfa = Dfa::new(start_id, bit_set::BitSet::new());
        let mut raw_transitions: Vec<(DfaStateID, char, DfaStateID)> = Vec::new();

        while let Some(current) = queue.pop_front() {
            let current_id = dfa_states[&current];

            if current.iter().any(|&state| nfa.accept().contains(&state)) {
                dfa.accepts.insert(current_id as usize);
            }

            let mut transitions_map: foldhash::HashMap<
                char,
                std::collections::BTreeSet<crate::automaton::nfa::NfaStateID>,
            > = foldhash::HashMap::new();

            for &state in &current {
                for &(from, label, to) in nfa.transitions() {
                    if from == state
                        && let Some(c) = label
                    {
                        transitions_map
                            .entry(c)
                            .or_default()
                            .extend(nfa.epsilon_closure([to].iter().cloned().collect()));
                    }
                }
            }

            for (c, next) in transitions_map {
                if next.is_empty() {
                    continue;
                }

                if !dfa_states.contains_key(&next) {
                    let next_id = dfa_states.len() as DfaStateID;
                    dfa_states.insert(next.clone(), next_id);
                    queue.push_back(next.clone());
                }

                let next_id = dfa_states[&next];
                raw_transitions.push((current_id, c, next_id));
            }
        }

        let state_count = dfa_states.len();
        dfa.state_count = state_count;
        dfa.ascii_table = vec![DEAD; state_count * 128];
        dfa.unicode_table = vec![foldhash::HashMap::new(); state_count];

        for (from, c, to) in raw_transitions {
            if c.is_ascii() {
                let idx = from as usize * 128 + c as usize;
                dfa.ascii_table[idx] = to;
            } else {
                dfa.unicode_table[from as usize].insert(c, to);
            }
        }

        dfa.accels = (0..state_count)
            .map(|state| build_accel(state, &dfa.ascii_table))
            .collect();

        dfa
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut state = self.start();

        if input.is_ascii() {
            let bytes = input.as_bytes();
            state = match if bytes.len() < ACCEL_MIN_REMAINING {
                self.step_ascii(bytes, state)
            } else {
                self.step_ascii_accel(bytes, state)
            } {
                Ok(state) => state,
                Err(()) => return false,
            };
        } else {
            let table = &self.ascii_table;
            let unicode = &self.unicode_table;
            for c in input.chars() {
                if c.is_ascii() {
                    let next = *unsafe { table.get_unchecked(state as usize * 128 + c as usize) };
                    if next == DEAD {
                        return false;
                    }
                    state = next;
                } else if let Some(&next) = unicode[state as usize].get(&c) {
                    state = next;
                } else {
                    return false;
                }
            }
        }

        self.accepts.contains(state as usize)
    }

    #[inline]
    fn step_ascii(&self, bytes: &[u8], mut state: DfaStateID) -> Result<DfaStateID, ()> {
        let table = &self.ascii_table;
        for &byte in bytes {
            let next = *unsafe { table.get_unchecked(state as usize * 128 + byte as usize) };
            if next == DEAD {
                return Err(());
            }
            state = next;
        }
        Ok(state)
    }

    #[inline]
    fn step_ascii_accel(&self, bytes: &[u8], mut state: DfaStateID) -> Result<DfaStateID, ()> {
        let table = &self.ascii_table;
        let mut at = 0usize;
        let len = bytes.len();

        while at < len {
            let remaining = len - at;
            if remaining >= ACCEL_MIN_REMAINING {
                let accel = self.accels[state as usize];
                if accel.is_enabled() {
                    if let Some(loop_byte) = accel.loop_byte {
                        let start = at;
                        while at < len && bytes[at] == loop_byte {
                            at += 1;
                        }
                        if at >= len {
                            break;
                        }
                        if at > start {
                            continue;
                        }
                    } else if let Some(hit) = accel.memchr_fwd(bytes, at) {
                        at = hit;
                        if at >= len {
                            break;
                        }
                    }
                }
            }

            let byte = bytes[at];
            let next = *unsafe { table.get_unchecked(state as usize * 128 + byte as usize) };
            if next == DEAD {
                return Err(());
            }
            state = next;
            at += 1;
        }

        Ok(state)
    }
}

fn build_accel(state: usize, table: &[DfaStateID]) -> Accel {
    let base = state * 128;
    let self_id = state as DfaStateID;

    let mut loop_bytes = Vec::new();
    let mut exit_bytes: Vec<u8> = Vec::new();

    for byte in 0u8..128 {
        let next = table[base + byte as usize];
        if next == self_id {
            loop_bytes.push(byte);
        } else if next != DEAD {
            exit_bytes.push(byte);
        }
    }

    let loop_byte = if loop_bytes.len() == 1 {
        Some(loop_bytes[0])
    } else {
        None
    };

    let mut unique_exits = Vec::new();
    for byte in exit_bytes {
        let next = table[base + byte as usize];
        if !unique_exits.iter().any(|&(_, id)| id == next) {
            unique_exits.push((byte, next));
        }
    }

    if unique_exits.len() > 3 {
        return Accel::default();
    }

    if loop_byte.is_none() && unique_exits.is_empty() {
        return Accel::default();
    }

    let mut needles = [0u8; 3];
    for (i, &(byte, _)) in unique_exits.iter().enumerate() {
        needles[i] = byte;
    }

    Accel {
        loop_byte,
        needles,
        needle_len: unique_exits.len() as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dfa_from_pattern(pattern: &str) -> Dfa {
        let mut lexer = crate::lexer::Lexer::new(pattern);
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let nfa = crate::automaton::nfa::Nfa::new_from_node(
            ast,
            &mut crate::automaton::nfa::NfaState::new(),
        )
        .unwrap();
        Dfa::from_nfa(&nfa)
    }

    #[test]
    fn test_dfa_from_nfa() {
        let nfa = crate::automaton::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Char('a'),
            &mut crate::automaton::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa);
        assert_eq!(dfa.start(), 0);
        assert!(dfa.accepts_contains(1));
        assert_eq!(dfa.transitions(), [(0, 'a', 1)].iter().cloned().collect());

        let nfa = crate::automaton::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Char('b')),
            ),
            &mut crate::automaton::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa);
        assert_eq!(dfa.start(), 0);
        assert!(dfa.accepts_contains(1));
        assert!(dfa.accepts_contains(2));

        let transitions = dfa.transitions();
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&(0, 'a', 1)) || transitions.contains(&(0, 'a', 2)));
        assert!(transitions.contains(&(0, 'b', 1)) || transitions.contains(&(0, 'b', 2)));
        assert!(transitions.contains(&(0, 'a', 1)) != transitions.contains(&(0, 'b', 1)));

        let nfa = crate::automaton::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Star(Box::new(
                    crate::parser::AstNode::Char('b'),
                ))),
            ),
            &mut crate::automaton::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa);
        assert_eq!(dfa.start(), 0);
        assert!(dfa.accepts_contains(0));
        assert!(dfa.accepts_contains(1));
        assert!(dfa.accepts_contains(2));

        let transitions = dfa.transitions();
        assert_eq!(transitions.len(), 3);

        let a_transitions: Vec<_> = transitions
            .iter()
            .filter(|(from, c, _)| *from == 0 && *c == 'a')
            .collect();
        let b_transitions: Vec<_> = transitions
            .iter()
            .filter(|(from, c, _)| *from == 0 && *c == 'b')
            .collect();

        assert_eq!(a_transitions.len(), 1);
        assert_eq!(b_transitions.len(), 1);

        let b_state = b_transitions[0].2;
        let b_loops: Vec<_> = transitions
            .iter()
            .filter(|(from, c, to)| *from == b_state && *c == 'b' && *to == b_state)
            .collect();

        assert_eq!(b_loops.len(), 1);
    }

    #[test]
    fn accel_a_plus_b() {
        let dfa = dfa_from_pattern("a+b");
        let mut saw_loop = false;
        for state in 0..dfa.state_count {
            let (loop_byte, needle_len, needles) = dfa.accel(state as DfaStateID);
            if loop_byte == Some(b'a') {
                saw_loop = true;
                assert_eq!(needle_len, 1);
                assert_eq!(needles[0], b'b');
            }
        }
        assert!(saw_loop);
        assert!(!dfa.is_match(&"a".repeat(1000)));
        assert!(dfa.is_match(&format!("{}b", "a".repeat(1000))));
        assert!(!dfa.is_match(&format!("{}c", "a".repeat(10))));
    }

    #[test]
    fn accel_star_b() {
        let dfa = dfa_from_pattern("b*");
        let mut saw_loop = false;
        for state in 0..dfa.state_count {
            let (loop_byte, needle_len, _) = dfa.accel(state as DfaStateID);
            if loop_byte == Some(b'b') && needle_len == 0 {
                saw_loop = true;
            }
        }
        assert!(saw_loop);
        assert!(dfa.is_match(""));
        assert!(dfa.is_match(&"b".repeat(1000)));
    }
}
