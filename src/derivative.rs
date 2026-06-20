use foldhash::HashMapExt as _;
use foldhash::HashSetExt as _;

const DEFAULT_MAX_AST_SIZE: usize = 1000;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct AstId(u32);

impl AstId {
    fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum NodeKind {
    Empty,
    Epsilon,
    Char(char),
    Plus(AstId),
    Star(AstId),
    Question(AstId),
    Or(AstId, AstId),
    Seq(AstId, AstId),
}

struct AstArena {
    nodes: Vec<NodeKind>,
    interner: foldhash::HashMap<NodeKind, AstId>,
    nullable_cache: Vec<Option<bool>>,
    structural_size_cache: Vec<Option<usize>>,
    empty: AstId,
    epsilon: AstId,
}

impl AstArena {
    fn new() -> Self {
        let mut arena = AstArena {
            nodes: Vec::new(),
            interner: foldhash::HashMap::new(),
            nullable_cache: Vec::new(),
            structural_size_cache: Vec::new(),
            empty: AstId(0),
            epsilon: AstId(0),
        };

        let empty = arena.direct_intern(NodeKind::Empty);
        arena.empty = empty;
        let epsilon = arena.direct_intern(NodeKind::Epsilon);
        arena.epsilon = epsilon;

        arena
    }

    fn empty(&self) -> AstId {
        self.empty
    }

    fn epsilon(&self) -> AstId {
        self.epsilon
    }

    fn kind(&self, id: AstId) -> &NodeKind {
        &self.nodes[id.index()]
    }

    fn intern(&mut self, kind: NodeKind) -> AstId {
        if let Some(&id) = self.interner.get(&kind) {
            return id;
        }

        let id = AstId(self.nodes.len() as u32);
        self.nodes.push(kind.clone());
        self.nullable_cache.push(None);
        self.structural_size_cache.push(None);
        self.interner.insert(kind, id);
        id
    }

    fn direct_intern(&mut self, kind: NodeKind) -> AstId {
        let id = AstId(self.nodes.len() as u32);
        self.nodes.push(kind.clone());
        self.nullable_cache.push(None);
        self.structural_size_cache.push(None);
        self.interner.insert(kind, id);
        id
    }

    fn nullable_of(&mut self, id: AstId) -> bool {
        if let Some(value) = self.nullable_cache[id.index()] {
            return value;
        }

        let value = match self.kind(id).clone() {
            NodeKind::Empty => false,
            NodeKind::Epsilon => true,
            NodeKind::Char(_) => false,
            NodeKind::Plus(inner) => self.nullable_of(inner),
            NodeKind::Star(_) => true,
            NodeKind::Question(_) => true,
            NodeKind::Or(left, right) => self.nullable_of(left) || self.nullable_of(right),
            NodeKind::Seq(left, right) => self.nullable_of(left) && self.nullable_of(right),
        };
        self.nullable_cache[id.index()] = Some(value);
        value
    }

    fn structural_size_of(&mut self, root: AstId) -> usize {
        if let Some(size) = self.structural_size_cache[root.index()] {
            return size;
        }

        let mut visited = foldhash::HashSet::new();
        structural_size_dfs(self, root, &mut visited);
        let size = visited.len();
        self.structural_size_cache[root.index()] = Some(size);
        size
    }

    fn export(&self, id: AstId) -> crate::parser::AstNode {
        match self.kind(id) {
            NodeKind::Empty => crate::parser::AstNode::Empty,
            NodeKind::Epsilon => crate::parser::AstNode::Epsilon,
            NodeKind::Char(c) => crate::parser::AstNode::Char(*c),
            NodeKind::Plus(inner) => crate::parser::AstNode::Plus(Box::new(self.export(*inner))),
            NodeKind::Star(inner) => crate::parser::AstNode::Star(Box::new(self.export(*inner))),
            NodeKind::Question(inner) => {
                crate::parser::AstNode::Question(Box::new(self.export(*inner)))
            }
            NodeKind::Or(left, right) => crate::parser::AstNode::Or(
                Box::new(self.export(*left)),
                Box::new(self.export(*right)),
            ),
            NodeKind::Seq(left, right) => crate::parser::AstNode::Seq(
                Box::new(self.export(*left)),
                Box::new(self.export(*right)),
            ),
        }
    }
}

pub struct Derivative {
    arena: std::cell::RefCell<AstArena>,
    start: AstId,
    canonical: crate::parser::AstNode,
    max_ast_size: usize,
}

impl Derivative {
    pub fn new(ast: crate::parser::AstNode) -> Self {
        let mut arena = AstArena::new();
        let start = from_parser(&mut arena, &ast);
        let canonical = arena.export(start);

        Derivative {
            arena: std::cell::RefCell::new(arena),
            start,
            canonical,
            max_ast_size: DEFAULT_MAX_AST_SIZE,
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut arena = self.arena.borrow_mut();
        let mut memo: foldhash::HashMap<(AstId, char), AstId> = foldhash::HashMap::new();
        let mut state = self.start;

        for ch in input.chars() {
            state = derivative_with_cache(&mut arena, state, ch, &mut memo);

            if arena.structural_size_of(state) > self.max_ast_size {
                return match_fallback(&self.canonical, input);
            }
        }

        arena.nullable_of(state)
    }

    pub fn is_empty_match(&self) -> bool {
        self.arena.borrow_mut().nullable_of(self.start)
    }
}

impl Clone for Derivative {
    fn clone(&self) -> Self {
        let mut clone = Derivative::new(self.canonical.clone());
        clone.max_ast_size = self.max_ast_size;
        clone
    }
}

impl std::fmt::Debug for Derivative {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Derivative")
            .field("ast", &self.canonical)
            .field("max_ast_size", &self.max_ast_size)
            .finish()
    }
}

impl PartialEq for Derivative {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for Derivative {}

fn from_parser(arena: &mut AstArena, node: &crate::parser::AstNode) -> AstId {
    match node {
        crate::parser::AstNode::Empty => arena.empty(),
        crate::parser::AstNode::Epsilon => arena.epsilon(),
        crate::parser::AstNode::Char(c) => mk_char(arena, *c),
        crate::parser::AstNode::Plus(inner) => {
            let inner_id = from_parser(arena, inner);
            mk_plus(arena, inner_id)
        }
        crate::parser::AstNode::Star(inner) => {
            let inner_id = from_parser(arena, inner);
            mk_star(arena, inner_id)
        }
        crate::parser::AstNode::Question(inner) => {
            let inner_id = from_parser(arena, inner);
            mk_question(arena, inner_id)
        }
        crate::parser::AstNode::Or(left, right) => {
            let left_id = from_parser(arena, left);
            let right_id = from_parser(arena, right);
            mk_or(arena, left_id, right_id)
        }
        crate::parser::AstNode::Seq(left, right) => {
            let left_id = from_parser(arena, left);
            let right_id = from_parser(arena, right);
            mk_seq(arena, left_id, right_id)
        }
    }
}

fn derivative_with_cache(
    arena: &mut AstArena,
    id: AstId,
    c: char,
    memo: &mut foldhash::HashMap<(AstId, char), AstId>,
) -> AstId {
    if let Some(&cached) = memo.get(&(id, c)) {
        return cached;
    }

    let result = derivative_id(arena, id, c);
    memo.insert((id, c), result);
    result
}

fn derivative_id(arena: &mut AstArena, id: AstId, c: char) -> AstId {
    match arena.kind(id).clone() {
        NodeKind::Empty | NodeKind::Epsilon => arena.empty(),
        NodeKind::Char(ch) => {
            if ch == c {
                arena.epsilon()
            } else {
                arena.empty()
            }
        }
        NodeKind::Plus(inner) => {
            let head = derivative_id(arena, inner, c);
            let tail = mk_star(arena, inner);
            mk_seq(arena, head, tail)
        }
        NodeKind::Star(inner) => {
            let head = derivative_id(arena, inner, c);
            let tail = mk_star(arena, inner);
            mk_seq(arena, head, tail)
        }
        NodeKind::Question(inner) => derivative_id(arena, inner, c),
        NodeKind::Or(left, right) => {
            let dl = derivative_id(arena, left, c);
            let dr = derivative_id(arena, right, c);
            mk_or(arena, dl, dr)
        }
        NodeKind::Seq(left, right) => {
            let left_derivative = derivative_id(arena, left, c);
            let first = mk_seq(arena, left_derivative, right);

            let delta_left = delta_id(arena, left);
            let right_derivative = derivative_id(arena, right, c);
            let second = mk_seq(arena, delta_left, right_derivative);

            mk_or(arena, first, second)
        }
    }
}

fn delta_id(arena: &mut AstArena, id: AstId) -> AstId {
    if arena.nullable_of(id) {
        arena.epsilon()
    } else {
        arena.empty()
    }
}

fn structural_size_dfs(arena: &AstArena, id: AstId, visited: &mut foldhash::HashSet<AstId>) {
    if !visited.insert(id) {
        return;
    }

    match arena.kind(id) {
        NodeKind::Plus(inner) | NodeKind::Star(inner) | NodeKind::Question(inner) => {
            structural_size_dfs(arena, *inner, visited)
        }
        NodeKind::Or(left, right) | NodeKind::Seq(left, right) => {
            structural_size_dfs(arena, *left, visited);
            structural_size_dfs(arena, *right, visited);
        }
        NodeKind::Empty | NodeKind::Epsilon | NodeKind::Char(_) => {}
    }
}

fn mk_char(arena: &mut AstArena, c: char) -> AstId {
    arena.intern(NodeKind::Char(c))
}

fn mk_plus(arena: &mut AstArena, inner: AstId) -> AstId {
    if inner == arena.empty() {
        arena.empty()
    } else {
        arena.intern(NodeKind::Plus(inner))
    }
}

fn mk_star(arena: &mut AstArena, inner: AstId) -> AstId {
    if inner == arena.empty() || inner == arena.epsilon() {
        arena.epsilon()
    } else {
        arena.intern(NodeKind::Star(inner))
    }
}

fn mk_question(arena: &mut AstArena, inner: AstId) -> AstId {
    if inner == arena.empty() {
        arena.epsilon()
    } else {
        arena.intern(NodeKind::Question(inner))
    }
}

fn mk_seq(arena: &mut AstArena, left: AstId, right: AstId) -> AstId {
    if left == arena.empty() || right == arena.empty() {
        arena.empty()
    } else if left == arena.epsilon() {
        right
    } else if right == arena.epsilon() {
        left
    } else {
        arena.intern(NodeKind::Seq(left, right))
    }
}

fn mk_or(arena: &mut AstArena, left: AstId, right: AstId) -> AstId {
    if left == right {
        return left;
    }
    if left == arena.empty() {
        return right;
    }
    if right == arena.empty() {
        return left;
    }
    let (lo, hi) = ordered_pair(left, right);
    arena.intern(NodeKind::Or(lo, hi))
}

fn ordered_pair(a: AstId, b: AstId) -> (AstId, AstId) {
    if a > b { (b, a) } else { (a, b) }
}

fn match_fallback(original: &crate::parser::AstNode, input: &str) -> bool {
    let mut ast = original.clone();
    for ch in input.chars() {
        ast = derivative_parser(&ast, ch);
    }
    contain_epsilon_parser(&ast)
}

fn derivative_parser(ast: &crate::parser::AstNode, c: char) -> crate::parser::AstNode {
    let raw = match ast {
        crate::parser::AstNode::Empty | crate::parser::AstNode::Epsilon => {
            crate::parser::AstNode::Empty
        }
        crate::parser::AstNode::Char(ch) => {
            if *ch == c {
                crate::parser::AstNode::Epsilon
            } else {
                crate::parser::AstNode::Empty
            }
        }
        crate::parser::AstNode::Plus(inner) => crate::parser::AstNode::Seq(
            Box::new(derivative_parser(inner, c)),
            Box::new(crate::parser::AstNode::Star(inner.clone())),
        ),
        crate::parser::AstNode::Star(inner) => crate::parser::AstNode::Seq(
            Box::new(derivative_parser(inner, c)),
            Box::new(crate::parser::AstNode::Star(inner.clone())),
        ),
        crate::parser::AstNode::Question(inner) => derivative_parser(inner, c),
        crate::parser::AstNode::Or(left, right) => crate::parser::AstNode::Or(
            Box::new(derivative_parser(left, c)),
            Box::new(derivative_parser(right, c)),
        ),
        crate::parser::AstNode::Seq(left, right) => crate::parser::AstNode::Or(
            Box::new(crate::parser::AstNode::Seq(
                Box::new(derivative_parser(left, c)),
                Box::new((**right).clone()),
            )),
            Box::new(crate::parser::AstNode::Seq(
                Box::new(delta_parser(left)),
                Box::new(derivative_parser(right, c)),
            )),
        ),
    };

    normalize_parser(raw)
}

fn normalize_parser(ast: crate::parser::AstNode) -> crate::parser::AstNode {
    match ast {
        crate::parser::AstNode::Or(left, right) => {
            let left = normalize_parser(*left);
            let right = normalize_parser(*right);

            if matches!(left, crate::parser::AstNode::Empty) {
                return right;
            }
            if matches!(right, crate::parser::AstNode::Empty) {
                return left;
            }
            if left == right {
                return left;
            }

            crate::parser::AstNode::Or(Box::new(left), Box::new(right))
        }
        crate::parser::AstNode::Seq(left, right) => {
            let left = normalize_parser(*left);
            let right = normalize_parser(*right);

            if matches!(left, crate::parser::AstNode::Empty)
                || matches!(right, crate::parser::AstNode::Empty)
            {
                return crate::parser::AstNode::Empty;
            }
            if matches!(left, crate::parser::AstNode::Epsilon) {
                return right;
            }
            if matches!(right, crate::parser::AstNode::Epsilon) {
                return left;
            }

            crate::parser::AstNode::Seq(Box::new(left), Box::new(right))
        }
        crate::parser::AstNode::Plus(inner) => {
            let inner = normalize_parser(*inner);
            if matches!(inner, crate::parser::AstNode::Empty) {
                crate::parser::AstNode::Empty
            } else {
                crate::parser::AstNode::Plus(Box::new(inner))
            }
        }
        crate::parser::AstNode::Star(inner) => {
            let inner = normalize_parser(*inner);
            if matches!(inner, crate::parser::AstNode::Empty)
                || matches!(inner, crate::parser::AstNode::Epsilon)
            {
                crate::parser::AstNode::Epsilon
            } else {
                crate::parser::AstNode::Star(Box::new(inner))
            }
        }
        crate::parser::AstNode::Question(inner) => {
            let inner = normalize_parser(*inner);
            if matches!(inner, crate::parser::AstNode::Empty) {
                crate::parser::AstNode::Epsilon
            } else {
                crate::parser::AstNode::Question(Box::new(inner))
            }
        }
        other => other,
    }
}

fn delta_parser(ast: &crate::parser::AstNode) -> crate::parser::AstNode {
    if contain_epsilon_parser(ast) {
        crate::parser::AstNode::Epsilon
    } else {
        crate::parser::AstNode::Empty
    }
}

fn contain_epsilon_parser(ast: &crate::parser::AstNode) -> bool {
    match ast {
        crate::parser::AstNode::Epsilon
        | crate::parser::AstNode::Star(_)
        | crate::parser::AstNode::Question(_) => true,
        crate::parser::AstNode::Empty | crate::parser::AstNode::Char(_) => false,
        crate::parser::AstNode::Plus(inner) => contain_epsilon_parser(inner),
        crate::parser::AstNode::Or(left, right) => {
            contain_epsilon_parser(left) || contain_epsilon_parser(right)
        }
        crate::parser::AstNode::Seq(left, right) => {
            contain_epsilon_parser(left) && contain_epsilon_parser(right)
        }
    }
}
