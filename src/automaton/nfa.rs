pub type NfaStateID = u64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NfaState {
    id: NfaStateID,
}

impl NfaState {
    pub fn new() -> Self {
        NfaState { id: 0 }
    }

    fn new_state(&mut self) -> NfaStateID {
        let id = self.id;
        self.id += 1;
        id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nfa {
    start: NfaStateID,
    accept: std::collections::HashSet<NfaStateID>,
    transitions:
        std::collections::HashSet<(NfaStateID, crate::automaton::label::NfaLabel, NfaStateID)>,
}

impl Nfa {
    pub fn new(start: NfaStateID, accept: Vec<NfaStateID>) -> Self {
        Nfa {
            start,
            accept: accept.into_iter().collect(),
            transitions: std::collections::HashSet::new(),
        }
    }

    pub fn start(&self) -> NfaStateID {
        self.start
    }

    pub fn accept(&self) -> &std::collections::HashSet<NfaStateID> {
        &self.accept
    }

    pub fn transitions(
        &self,
    ) -> &std::collections::HashSet<(NfaStateID, crate::automaton::label::NfaLabel, NfaStateID)>
    {
        &self.transitions
    }

    fn add_transition(&mut self, from: NfaStateID, char: char, to: NfaStateID) {
        self.transitions
            .insert((from, crate::automaton::label::NfaLabel::Char(char), to));
    }

    fn add_class_transition(
        &mut self,
        from: NfaStateID,
        class: crate::charclass::CharClass,
        to: NfaStateID,
    ) {
        self.transitions
            .insert((from, crate::automaton::label::NfaLabel::Class(class), to));
    }

    fn add_epsilon_transition(&mut self, from: NfaStateID, to: NfaStateID) {
        self.transitions
            .insert((from, crate::automaton::label::NfaLabel::Epsilon, to));
    }

    fn merge_nfa(&mut self, other: &Nfa) {
        self.transitions.extend(other.transitions.clone());
        self.add_epsilon_transition(self.start, other.start);
        for accept in other.accept.iter() {
            self.accept.insert(*accept);
        }
    }

    pub fn new_from_node(node: crate::parser::AstNode, state: &mut NfaState) -> crate::Result<Nfa> {
        match node {
            crate::parser::AstNode::Char(c) => {
                let start = state.new_state();
                let accept = state.new_state();
                let mut nfa = Nfa::new(start, vec![accept]);
                nfa.add_transition(start, c, accept);

                Ok(nfa)
            }
            crate::parser::AstNode::Class(class) => {
                let start = state.new_state();
                let accept = state.new_state();
                let mut nfa = Nfa::new(start, vec![accept]);
                nfa.add_class_transition(start, class, accept);

                Ok(nfa)
            }
            crate::parser::AstNode::Epsilon => {
                let start = state.new_state();
                let accept = state.new_state();
                let mut nfa = Nfa::new(start, vec![accept]);
                nfa.add_epsilon_transition(start, accept);

                Ok(nfa)
            }
            crate::parser::AstNode::Plus(boxed) => {
                let remain = Nfa::new_from_node(*boxed, state)?;
                let start = state.new_state();
                let accept = state.new_state();
                let mut nfa = Nfa::new(start, vec![accept]);

                nfa.transitions.extend(remain.transitions.clone());
                nfa.add_epsilon_transition(start, remain.start);
                nfa.add_epsilon_transition(start, accept);
                for accept_state in remain.accept.iter() {
                    nfa.add_epsilon_transition(*accept_state, remain.start);
                    nfa.add_epsilon_transition(*accept_state, accept);
                }

                Ok(nfa)
            }
            crate::parser::AstNode::Star(boxed) => {
                let remain = Nfa::new_from_node(*boxed, state)?;
                let start = state.new_state();
                let mut accepts = remain.accept.clone();
                accepts.insert(start);

                let mut nfa = Nfa::new(start, accepts.into_iter().collect());
                nfa.merge_nfa(&remain);
                nfa.add_epsilon_transition(start, remain.start);

                for accept in &remain.accept {
                    nfa.add_epsilon_transition(*accept, remain.start);
                }

                Ok(nfa)
            }
            crate::parser::AstNode::Question(boxed) => {
                let remain = Nfa::new_from_node(*boxed, state)?;
                let start = state.new_state();
                let accept = remain
                    .accept
                    .union(&[start].into_iter().collect())
                    .cloned()
                    .collect();
                let mut nfa = Nfa::new(start, accept);
                nfa.merge_nfa(&remain);
                nfa.add_epsilon_transition(start, remain.start);

                for accept in &remain.accept {
                    nfa.add_epsilon_transition(*accept, remain.start);
                }

                Ok(nfa)
            }
            crate::parser::AstNode::Or(boxed1, boxed2) => {
                let remain1 = Nfa::new_from_node(*boxed1, state)?;
                let remain2 = Nfa::new_from_node(*boxed2, state)?;
                let start = state.new_state();

                let accept: std::collections::HashSet<NfaStateID> =
                    remain1.accept.union(&remain2.accept).cloned().collect();
                let mut nfa = Nfa::new(start, accept.into_iter().collect());
                nfa.merge_nfa(&remain1);
                nfa.merge_nfa(&remain2);
                nfa.add_epsilon_transition(start, remain1.start);
                nfa.add_epsilon_transition(start, remain2.start);

                Ok(nfa)
            }
            crate::parser::AstNode::Seq(left, right) => {
                let left_nfa = Nfa::new_from_node(*left, state)?;
                let right_nfa = Nfa::new_from_node(*right, state)?;

                let mut nfa = Nfa::new(left_nfa.start, right_nfa.accept.iter().copied().collect());
                nfa.transitions.extend(left_nfa.transitions);
                nfa.transitions.extend(right_nfa.transitions);
                for &accept in &left_nfa.accept {
                    nfa.add_epsilon_transition(accept, right_nfa.start);
                }

                Ok(nfa)
            }
            crate::parser::AstNode::Empty => unreachable!(),
        }
    }

    pub fn epsilon_closure_with_bitset(&self, start: &bit_set::BitSet) -> bit_set::BitSet {
        let mut visited = bit_set::BitSet::new();
        let mut to_visit = std::collections::VecDeque::new();

        for state in start.iter() {
            if !visited.contains(state) {
                to_visit.push_back(state as NfaStateID);
            }
        }

        while let Some(state) = to_visit.pop_front() {
            if !visited.contains(state as usize) {
                visited.insert(state as usize);
                for &(from, label, to) in self.transitions() {
                    if from == state
                        && label == crate::automaton::label::NfaLabel::Epsilon
                        && !visited.contains(to as usize)
                    {
                        to_visit.push_back(to);
                    }
                }
            }
        }

        visited
    }

    pub fn epsilon_closure(
        &self,
        start: std::collections::BTreeSet<NfaStateID>,
    ) -> std::collections::BTreeSet<NfaStateID> {
        let mut bit_start = bit_set::BitSet::new();
        for &state in &start {
            bit_start.insert(state as usize);
        }

        let bit_result = self.epsilon_closure_with_bitset(&bit_start);
        bit_result.iter().map(|s| s as NfaStateID).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::label::NfaLabel;

    #[test]
    fn class_digit() {
        let nfa = Nfa::new_from_node(
            crate::parser::AstNode::Class(crate::charclass::CharClass::Digit),
            &mut NfaState::new(),
        )
        .unwrap();
        assert_eq!(nfa.transitions.len(), 1);
        let (_, label, _) = nfa.transitions.iter().next().unwrap();
        assert_eq!(*label, NfaLabel::Class(crate::charclass::CharClass::Digit));
    }

    #[test]
    fn new_from_node() {
        // a
        let nfa =
            Nfa::new_from_node(crate::parser::AstNode::Char('a'), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 0);
        assert_eq!(nfa.accept, [1].into());
        assert_eq!(
            nfa.transitions,
            vec![(0, NfaLabel::Char('a'), 1)].into_iter().collect()
        );

        // [empty]
        let nfa =
            Nfa::new_from_node(crate::parser::AstNode::Epsilon, &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 0);
        assert_eq!(nfa.accept, [1].into());
        assert_eq!(
            nfa.transitions,
            vec![(0, NfaLabel::Epsilon, 1)].into_iter().collect()
        );

        // a*
        let nfa = Nfa::new_from_node(
            crate::parser::AstNode::Star(Box::new(crate::parser::AstNode::Char('a'))),
            &mut NfaState::new(),
        )
        .unwrap();
        assert_eq!(nfa.start, 2);
        assert_eq!(nfa.accept, [1, 2].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Epsilon, 0),
                (1, NfaLabel::Epsilon, 0)
            ]
            .into_iter()
            .collect()
        );

        // a|b
        let nfa = Nfa::new_from_node(
            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Char('b')),
            ),
            &mut NfaState::new(),
        )
        .unwrap();
        assert_eq!(nfa.start, 4);
        assert_eq!(nfa.accept, [1, 3].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Char('b'), 3),
                (4, NfaLabel::Epsilon, 0),
                (4, NfaLabel::Epsilon, 2)
            ]
            .into_iter()
            .collect()
        );

        // a?
        let nfa = Nfa::new_from_node(
            crate::parser::AstNode::Question(Box::new(crate::parser::AstNode::Char('a'))),
            &mut NfaState::new(),
        )
        .unwrap();
        assert_eq!(nfa.start, 2);
        assert_eq!(nfa.accept, [1, 2].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (1, NfaLabel::Epsilon, 0),
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Epsilon, 0)
            ]
            .into_iter()
            .collect()
        );

        // a+
        let nfa = Nfa::new_from_node(
            crate::parser::AstNode::Plus(Box::new(crate::parser::AstNode::Char('a'))),
            &mut NfaState::new(),
        )
        .unwrap();
        assert_eq!(nfa.start, 2);
        assert_eq!(nfa.accept, [3].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (1, NfaLabel::Epsilon, 3),
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Epsilon, 3),
                (1, NfaLabel::Epsilon, 0),
                (2, NfaLabel::Epsilon, 0)
            ]
            .into_iter()
            .collect()
        );

        // ab
        let nfa = Nfa::new_from_node(
            crate::parser::AstNode::Seq(
                Box::new(crate::parser::AstNode::Char('a')),
                Box::new(crate::parser::AstNode::Char('b')),
            ),
            &mut NfaState::new(),
        )
        .unwrap();
        assert_eq!(nfa.start, 0);
        assert_eq!(nfa.accept, [3].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Char('b'), 3),
                (1, NfaLabel::Epsilon, 2)
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn from_str_to_nfa() {
        let mut lexer = crate::lexer::Lexer::new("a|b");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 4);
        assert_eq!(nfa.accept, [1, 3].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Char('b'), 3),
                (4, NfaLabel::Epsilon, 0),
                (4, NfaLabel::Epsilon, 2)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("a|b*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 5);
        assert_eq!(nfa.accept, [1, 3, 4].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (5, NfaLabel::Epsilon, 0),
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Char('b'), 3),
                (3, NfaLabel::Epsilon, 2),
                (5, NfaLabel::Epsilon, 4),
                (4, NfaLabel::Epsilon, 2)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("a|b+");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 6);
        assert_eq!(nfa.accept, [1, 5].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (4, NfaLabel::Epsilon, 2),
                (3, NfaLabel::Epsilon, 2),
                (4, NfaLabel::Epsilon, 5),
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Char('b'), 3),
                (3, NfaLabel::Epsilon, 5),
                (6, NfaLabel::Epsilon, 4),
                (6, NfaLabel::Epsilon, 0)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("a|b?");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 5);
        assert_eq!(nfa.accept, [1, 3, 4].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (4, NfaLabel::Epsilon, 2),
                (5, NfaLabel::Epsilon, 0),
                (2, NfaLabel::Char('b'), 3),
                (5, NfaLabel::Epsilon, 4),
                (3, NfaLabel::Epsilon, 2),
                (0, NfaLabel::Char('a'), 1)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("a|b|c");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 7);
        assert_eq!(nfa.accept, [1, 3, 5].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (0, NfaLabel::Char('a'), 1),
                (6, NfaLabel::Epsilon, 4),
                (6, NfaLabel::Epsilon, 2),
                (4, NfaLabel::Char('c'), 5),
                (7, NfaLabel::Epsilon, 0),
                (2, NfaLabel::Char('b'), 3),
                (7, NfaLabel::Epsilon, 6)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("a(b|c)");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 0);
        assert_eq!(nfa.accept, [3, 5].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (1, NfaLabel::Epsilon, 6),
                (0, NfaLabel::Char('a'), 1),
                (2, NfaLabel::Char('b'), 3),
                (6, NfaLabel::Epsilon, 4),
                (6, NfaLabel::Epsilon, 2),
                (4, NfaLabel::Char('c'), 5)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("((a|b)+)*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 7);
        assert_eq!(nfa.accept, [6, 7].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (5, NfaLabel::Epsilon, 4),
                (1, NfaLabel::Epsilon, 4),
                (2, NfaLabel::Char('b'), 3),
                (7, NfaLabel::Epsilon, 5),
                (1, NfaLabel::Epsilon, 6),
                (5, NfaLabel::Epsilon, 6),
                (4, NfaLabel::Epsilon, 0),
                (4, NfaLabel::Epsilon, 2),
                (0, NfaLabel::Char('a'), 1),
                (3, NfaLabel::Epsilon, 6),
                (6, NfaLabel::Epsilon, 5),
                (3, NfaLabel::Epsilon, 4)
            ]
            .into_iter()
            .collect()
        );

        let mut lexer = crate::lexer::Lexer::new("a|b*|c?");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 9);
        assert_eq!(nfa.accept, [1, 3, 4, 6, 7].into());
        assert_eq!(
            nfa.transitions,
            vec![
                (8, NfaLabel::Epsilon, 4),
                (7, NfaLabel::Epsilon, 5),
                (6, NfaLabel::Epsilon, 5),
                (2, NfaLabel::Char('b'), 3),
                (9, NfaLabel::Epsilon, 8),
                (0, NfaLabel::Char('a'), 1),
                (9, NfaLabel::Epsilon, 0),
                (5, NfaLabel::Char('c'), 6),
                (8, NfaLabel::Epsilon, 7),
                (3, NfaLabel::Epsilon, 2),
                (4, NfaLabel::Epsilon, 2)
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn e_closure() {
        let mut lexer = crate::lexer::Lexer::new("a|b*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(
            parser.parse().unwrap(),
            &mut crate::automaton::nfa::NfaState::new(),
        )
        .unwrap();

        let closure = nfa.epsilon_closure([nfa.start()].iter().cloned().collect());
        assert_eq!(closure, [0, 2, 4, 5].iter().cloned().collect());

        let mut lexer = crate::lexer::Lexer::new("a|b|c");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let nfa = Nfa::new_from_node(parser.parse().unwrap(), &mut NfaState::new()).unwrap();

        let closure = nfa.epsilon_closure([nfa.start()].iter().cloned().collect());
        assert_eq!(closure, [0, 2, 4, 6, 7].iter().cloned().collect());
    }
}
