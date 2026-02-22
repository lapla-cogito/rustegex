use foldhash::HashMapExt as _;

pub type DfaStateID = u64;
const DEAD: DfaStateID = DfaStateID::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dfa {
    start: DfaStateID,
    accepts: bit_set::BitSet,
    state_count: usize,
    ascii_table: Vec<DfaStateID>,
    unicode_table: Vec<foldhash::HashMap<char, DfaStateID>>,
}

impl Dfa {
    pub fn new(start: DfaStateID, accepts: bit_set::BitSet) -> Self {
        Dfa {
            start,
            accepts,
            state_count: 0,
            ascii_table: Vec::new(),
            unicode_table: Vec::new(),
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

        dfa
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut state = self.start();

        if input.is_ascii() {
            let table = &self.ascii_table;
            for &byte in input.as_bytes() {
                // SAFETY: `state < state_count` (invariant) and `byte < 128` (guaranteed by `input.is_ascii()`),
                // so `state as usize * 128 + byte as usize < state_count * 128 == table.len()`.
                let next = *unsafe { table.get_unchecked(state as usize * 128 + byte as usize) };
                if next == DEAD {
                    return false;
                }
                state = next;
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
