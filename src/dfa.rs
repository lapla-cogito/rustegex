pub type DfaStateID = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dfa {
    pub start: DfaStateID,
    pub accepts: std::collections::HashSet<DfaStateID>,
    pub transitions: std::collections::BTreeSet<(DfaStateID, char, DfaStateID)>,
}

impl Dfa {
    pub fn new(start: DfaStateID, accepts: std::collections::HashSet<DfaStateID>) -> Self {
        Dfa {
            start,
            accepts,
            transitions: std::collections::BTreeSet::new(),
        }
    }

    pub fn start(&self) -> DfaStateID {
        self.start
    }

    pub fn accept(&self) -> &std::collections::HashSet<DfaStateID> {
        &self.accepts
    }

    pub fn transitions(&self) -> &std::collections::BTreeSet<(DfaStateID, char, DfaStateID)> {
        &self.transitions
    }

    pub fn next_transit(&self, current: DfaStateID, input: char) -> Option<DfaStateID> {
        self.transitions
            .iter()
            .find(|(from, label, _)| *from == current && *label == input)
            .map(|(_, _, to)| *to)
    }

    pub fn from_nfa(nfa: &crate::nfa::Nfa) -> Self {
        let mut dfa_states = std::collections::BTreeMap::new();
        let mut queue = std::collections::VecDeque::new();

        let start: std::collections::BTreeSet<_> =
            epsilon_closure(nfa, [nfa.start()].iter().cloned().collect())
                .into_iter()
                .collect();

        let start_id = dfa_states.len() as DfaStateID;
        dfa_states.insert(start.clone(), start_id);
        queue.push_back(start.clone());

        let mut dfa = Dfa::new(start_id, std::collections::HashSet::new());

        while let Some(current) = queue.pop_front() {
            let current_id = dfa_states[&current];

            if current.iter().any(|&state| nfa.accept().contains(&state)) {
                dfa.accepts.insert(current_id);
            }

            for c in 1..=u8::MAX {
                let mut next = std::collections::BTreeSet::new();
                for &state in current.iter() {
                    for &(from, label, to) in nfa.transitions() {
                        if from == state && Some(c as char) == label {
                            next.extend(epsilon_closure(nfa, [to].iter().cloned().collect()));
                        }
                    }
                }

                if next.is_empty() {
                    continue;
                }

                if !dfa_states.contains_key(&next) {
                    let next_id = dfa_states.len() as DfaStateID;
                    dfa_states.insert(next.clone(), next_id);
                    queue.push_back(next.clone());
                }

                let next_id = dfa_states[&next];
                dfa.transitions.insert((current_id, c as char, next_id));
            }
        }

        dfa
    }
}

fn epsilon_closure(
    nfa: &crate::nfa::Nfa,
    start: std::collections::BTreeSet<crate::nfa::NfaStateID>,
) -> std::collections::BTreeSet<crate::nfa::NfaStateID> {
    let mut visited = std::collections::BTreeSet::new();
    let mut to_visit = std::collections::VecDeque::new();

    for &state in start.iter() {
        if !visited.contains(&state) {
            to_visit.push_back(state);
        }
    }

    while let Some(state) = to_visit.pop_front() {
        if visited.contains(&state) {
            continue;
        }
        visited.insert(state);

        for &(from, label, to) in nfa.transitions() {
            if from == state && label.is_none() && !visited.contains(&to) {
                to_visit.push_back(to);
            }
        }
    }

    visited
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e_closure() {
        let mut lexer = crate::lexer::Lexer::new("a|b*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = crate::nfa::Nfa::new_from_node(
            parser.parse().unwrap(),
            &mut crate::nfa::NfaState::new(),
        )
        .unwrap();

        let closure = epsilon_closure(&nfa, [nfa.start()].iter().cloned().collect());
        assert_eq!(closure, [0, 2, 4, 5].iter().cloned().collect());

        let mut lexer = crate::lexer::Lexer::new("a|b|c");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = crate::nfa::Nfa::new_from_node(
            parser.parse().unwrap(),
            &mut crate::nfa::NfaState::new(),
        )
        .unwrap();

        let closure = epsilon_closure(&nfa, [nfa.start()].iter().cloned().collect());
        assert_eq!(closure, [0, 2, 4, 6, 7].iter().cloned().collect());
    }

    #[test]
    fn test_dfa_from_nfa() {
        let nfa = crate::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Char('a'),
            &mut crate::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa);
        assert_eq!(dfa.start(), 0);
        assert_eq!(
            dfa.accept(),
            &[1u64]
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<u64>>()
        );
        assert_eq!(dfa.transitions(), &[(0, 'a', 1)].iter().cloned().collect());

        let nfa = crate::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Char('b')),
            ),
            &mut crate::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa);
        assert_eq!(dfa.start(), 0);
        assert_eq!(
            dfa.accept(),
            &[1u64, 2u64]
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<u64>>()
        );
        assert_eq!(
            dfa.transitions(),
            &[(0, 'a', 1), (0, 'b', 2)].iter().cloned().collect()
        );

        let nfa = crate::nfa::Nfa::new_from_node(
            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Star(Box::new(
                    crate::parser::AstNode::Char('b'),
                ))),
            ),
            &mut crate::nfa::NfaState::new(),
        )
        .unwrap();
        let dfa = Dfa::from_nfa(&nfa);
        assert_eq!(dfa.start(), 0);
        assert_eq!(
            dfa.accept(),
            &[0u64, 1u64, 2u64]
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<u64>>()
        );
        assert_eq!(
            dfa.transitions(),
            &[(0, 'a', 1), (0, 'b', 2), (2, 'b', 2)]
                .iter()
                .cloned()
                .collect()
        );
    }
}
