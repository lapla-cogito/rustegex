use foldhash::HashMapExt as _;

pub type DfaStateID = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dfa {
    start: DfaStateID,
    accepts: bit_set::BitSet,
    transitions: std::collections::BTreeSet<(DfaStateID, char, DfaStateID)>,
    cache: foldhash::HashMap<(DfaStateID, char), DfaStateID>,
}

impl Dfa {
    pub fn new(start: DfaStateID, accepts: bit_set::BitSet) -> Self {
        Dfa {
            start,
            accepts,
            transitions: std::collections::BTreeSet::new(),
            cache: foldhash::HashMap::new(),
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
    pub fn transitions(&self) -> &std::collections::BTreeSet<(DfaStateID, char, DfaStateID)> {
        &self.transitions
    }

    pub fn next_transit(
        &self,
        current: DfaStateID,
        input: char,
        use_dfa_cache: bool,
    ) -> Option<DfaStateID> {
        if use_dfa_cache && let Some(&next_state) = self.cache.get(&(current, input)) {
            return Some(next_state);
        }

        self.transitions
            .iter()
            .find(|(from, label, _)| *from == current && *label == input)
            .map(|(_, _, to)| *to)
    }

    pub fn from_nfa(nfa: &crate::automaton::nfa::Nfa, use_dfa_cache: bool) -> Self {
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
                dfa.transitions.insert((current_id, c, next_id));
                if use_dfa_cache {
                    dfa.cache.insert((current_id, c), next_id);
                }
            }
        }

        dfa
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut state = self.start();
        let use_dfa_cache = crate::use_dfa_cache(input);
        for c in input.chars() {
            if let Some(next) = self.next_transit(state, c, use_dfa_cache) {
                state = next;
            } else {
                return false;
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
        let dfa = Dfa::from_nfa(&nfa, false);
        assert_eq!(dfa.start(), 0);
        assert!(dfa.accepts_contains(1));
        assert_eq!(dfa.transitions(), &[(0, 'a', 1)].iter().cloned().collect());

        let nfa = crate::automaton::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Char('b')),
            ),
            &mut crate::automaton::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa, false);
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
        let dfa = Dfa::from_nfa(&nfa, false);
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
