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
    transitions: std::collections::HashSet<(NfaStateID, Option<char>, NfaStateID)>,
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
    ) -> &std::collections::HashSet<(NfaStateID, Option<char>, NfaStateID)> {
        &self.transitions
    }

    fn add_transition(&mut self, from: NfaStateID, char: char, to: NfaStateID) {
        self.transitions.insert((from, Some(char), to));
    }

    fn add_epsilon_transition(&mut self, from: NfaStateID, to: NfaStateID) {
        self.transitions.insert((from, None, to));
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
                let mut remain_chain: Option<Nfa> = None;

                for node in [left, right].iter() {
                    let mut remain = Nfa::new_from_node(*node.clone(), state)?;
                    if let Some(mut chain) = remain_chain {
                        for accept in chain.accept.iter() {
                            remain.add_epsilon_transition(*accept, remain.start);
                        }
                        chain.accept.clear();
                        chain.merge_nfa(&remain);
                        remain_chain = Some(chain.clone());
                    } else {
                        remain_chain = Some(remain);
                    }
                }

                if let Some(remain) = remain_chain {
                    Ok(remain)
                } else {
                    Err(crate::Error::InvalidSeq)
                }
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
                    if from == state && label.is_none() && !visited.contains(to as usize) {
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

    #[test]
    fn new_from_node() {
        // a
        let nfa =
            Nfa::new_from_node(crate::parser::AstNode::Char('a'), &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 0);
        assert_eq!(nfa.accept, [1].into());
        assert_eq!(
            nfa.transitions,
            vec![(0, Some('a'), 1)].into_iter().collect()
        );

        // [empty]
        let nfa =
            Nfa::new_from_node(crate::parser::AstNode::Epsilon, &mut NfaState::new()).unwrap();
        assert_eq!(nfa.start, 0);
        assert_eq!(nfa.accept, [1].into());
        assert_eq!(nfa.transitions, vec![(0, None, 1)].into_iter().collect());

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
            vec![(0, Some('a'), 1), (2, None, 0), (1, None, 0)]
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
                (0, Some('a'), 1),
                (2, Some('b'), 3),
                (4, None, 0),
                (4, None, 2)
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
            vec![(1, None, 0), (0, Some('a'), 1), (2, None, 0)]
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
                (1, None, 3),
                (0, Some('a'), 1),
                (2, None, 3),
                (1, None, 0),
                (2, None, 0)
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
                (0, Some('a'), 1),
                (0, None, 2),
                (2, Some('b'), 3),
                (1, None, 2)
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
                (0, Some('a'), 1),
                (2, Some('b'), 3),
                (4, None, 0),
                (4, None, 2)
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
                (5, None, 0),
                (0, Some('a'), 1),
                (2, Some('b'), 3),
                (3, None, 2),
                (5, None, 4),
                (4, None, 2)
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
                (4, None, 2),
                (3, None, 2),
                (4, None, 5),
                (0, Some('a'), 1),
                (2, Some('b'), 3),
                (3, None, 5),
                (6, None, 4),
                (6, None, 0)
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
                (4, None, 2),
                (5, None, 0),
                (2, Some('b'), 3),
                (5, None, 4),
                (3, None, 2),
                (0, Some('a'), 1)
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
                (0, Some('a'), 1),
                (6, None, 4),
                (6, None, 2),
                (4, Some('c'), 5),
                (7, None, 0),
                (2, Some('b'), 3),
                (7, None, 6)
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
                (1, None, 6),
                (0, None, 6),
                (0, Some('a'), 1),
                (2, Some('b'), 3),
                (6, None, 4),
                (6, None, 2),
                (4, Some('c'), 5)
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
                (5, None, 4),
                (1, None, 4),
                (2, Some('b'), 3),
                (7, None, 5),
                (1, None, 6),
                (5, None, 6),
                (4, None, 0),
                (4, None, 2),
                (0, Some('a'), 1),
                (3, None, 6),
                (6, None, 5),
                (3, None, 4)
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
                (8, None, 4),
                (7, None, 5),
                (6, None, 5),
                (2, Some('b'), 3),
                (9, None, 8),
                (0, Some('a'), 1),
                (9, None, 0),
                (5, Some('c'), 6),
                (8, None, 7),
                (3, None, 2),
                (4, None, 2)
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
