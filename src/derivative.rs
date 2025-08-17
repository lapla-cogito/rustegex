use foldhash::HashMapExt as _;

#[derive(Debug, Clone)]
pub struct Derivative {
    ast: crate::parser::AstNode,
    max_ast_size: usize,
}

impl Derivative {
    pub fn new(ast: crate::parser::AstNode) -> Self {
        let normalized_ast = normalize(ast);
        Derivative {
            ast: normalized_ast,
            max_ast_size: 1000,
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut ast = self.ast.clone();
        let mut memo = foldhash::HashMap::new();
        _match(&mut ast, input, &mut memo, self.max_ast_size)
    }

    pub fn is_empty_match(&self) -> bool {
        contain_epsilon(&self.ast)
    }
}

impl PartialEq for Derivative {
    fn eq(&self, other: &Self) -> bool {
        self.ast == other.ast
    }
}

fn ast_size(ast: &crate::parser::AstNode) -> usize {
    match ast {
        crate::parser::AstNode::Empty
        | crate::parser::AstNode::Epsilon
        | crate::parser::AstNode::Char(_) => 1,
        crate::parser::AstNode::Plus(inner)
        | crate::parser::AstNode::Star(inner)
        | crate::parser::AstNode::Question(inner) => 1 + ast_size(inner),
        crate::parser::AstNode::Or(left, right) | crate::parser::AstNode::Seq(left, right) => {
            1 + ast_size(left) + ast_size(right)
        }
    }
}

fn _match(
    ast: &mut crate::parser::AstNode,
    input: &str,
    memo: &mut foldhash::HashMap<(crate::parser::AstNode, char), crate::parser::AstNode>,
    max_size: usize,
) -> bool {
    let original_ast = ast.clone();
    for c in input.chars() {
        *ast = derivative_with_cache(ast, c, memo);
        *ast = normalize(ast.clone());

        if ast_size(ast) > max_size {
            return match_fallback(&original_ast, input);
        }
    }
    contain_epsilon(ast)
}

fn match_fallback(original_ast: &crate::parser::AstNode, input: &str) -> bool {
    let mut ast = original_ast.clone();
    for c in input.chars() {
        ast = derivative(&ast, c);
        ast = normalize(ast);
    }
    contain_epsilon(&ast)
}

fn derivative_with_cache(
    ast: &crate::parser::AstNode,
    c: char,
    memo: &mut foldhash::HashMap<(crate::parser::AstNode, char), crate::parser::AstNode>,
) -> crate::parser::AstNode {
    if let Some(cached) = memo.get(&(ast.clone(), c)) {
        return cached.clone();
    }

    let result = derivative(ast, c);
    let normalized_result = normalize(result);

    memo.insert((ast.clone(), c), normalized_result.clone());
    normalized_result
}

fn normalize(ast: crate::parser::AstNode) -> crate::parser::AstNode {
    match ast {
        crate::parser::AstNode::Or(left, right) => {
            let left = normalize(*left);
            let right = normalize(*right);

            match (&left, &right) {
                (crate::parser::AstNode::Empty, _) => right,
                (_, crate::parser::AstNode::Empty) => left,
                _ if left == right => left,
                _ => crate::parser::AstNode::Or(Box::new(left), Box::new(right)),
            }
        }
        crate::parser::AstNode::Seq(left, right) => {
            let left = normalize(*left);
            let right = normalize(*right);

            match (&left, &right) {
                (crate::parser::AstNode::Empty, _) | (_, crate::parser::AstNode::Empty) => {
                    crate::parser::AstNode::Empty
                }
                (crate::parser::AstNode::Epsilon, _) => right,
                (_, crate::parser::AstNode::Epsilon) => left,
                _ => crate::parser::AstNode::Seq(Box::new(left), Box::new(right)),
            }
        }
        crate::parser::AstNode::Plus(inner) => {
            let inner = normalize(*inner);
            match inner {
                crate::parser::AstNode::Empty => crate::parser::AstNode::Empty,
                _ => crate::parser::AstNode::Plus(Box::new(inner)),
            }
        }
        crate::parser::AstNode::Star(inner) => {
            let inner = normalize(*inner);
            match inner {
                crate::parser::AstNode::Empty | crate::parser::AstNode::Epsilon => {
                    crate::parser::AstNode::Epsilon
                }
                _ => crate::parser::AstNode::Star(Box::new(inner)),
            }
        }
        crate::parser::AstNode::Question(inner) => {
            let inner = normalize(*inner);
            match inner {
                crate::parser::AstNode::Empty => crate::parser::AstNode::Epsilon,
                _ => crate::parser::AstNode::Question(Box::new(inner)),
            }
        }
        _ => ast,
    }
}

fn derivative(ast: &crate::parser::AstNode, c: char) -> crate::parser::AstNode {
    match ast {
        crate::parser::AstNode::Empty | crate::parser::AstNode::Epsilon => {
            crate::parser::AstNode::Empty
        }
        crate::parser::AstNode::Char(c1) => {
            if c == *c1 {
                crate::parser::AstNode::Epsilon
            } else {
                crate::parser::AstNode::Empty
            }
        }
        crate::parser::AstNode::Plus(inner) => {
            let tmp = derivative(inner, c);
            normalize(crate::parser::AstNode::Seq(
                Box::new(tmp),
                Box::new(crate::parser::AstNode::Star(inner.clone())),
            ))
        }
        crate::parser::AstNode::Star(inner) => {
            let tmp = derivative(inner, c);
            normalize(crate::parser::AstNode::Seq(
                Box::new(tmp),
                Box::new(crate::parser::AstNode::Star(inner.clone())),
            ))
        }
        crate::parser::AstNode::Question(inner) => normalize(derivative(inner, c)),
        crate::parser::AstNode::Or(left, right) => {
            let left = derivative(left, c);
            let right = derivative(right, c);

            normalize(crate::parser::AstNode::Or(Box::new(left), Box::new(right)))
        }
        crate::parser::AstNode::Seq(left, right) => {
            let d1 = derivative(left, c);
            let d2 = derivative(right, c);

            normalize(crate::parser::AstNode::Or(
                Box::new(normalize(crate::parser::AstNode::Seq(
                    Box::new(d1),
                    Box::new(*right.clone()),
                ))),
                Box::new(normalize(crate::parser::AstNode::Seq(
                    Box::new(delta(left)),
                    Box::new(d2),
                ))),
            ))
        }
    }
}

fn delta(ast: &crate::parser::AstNode) -> crate::parser::AstNode {
    match contain_epsilon(ast) {
        true => crate::parser::AstNode::Epsilon,
        false => crate::parser::AstNode::Empty,
    }
}

fn contain_epsilon(ast: &crate::parser::AstNode) -> bool {
    match ast {
        crate::parser::AstNode::Empty => false,
        crate::parser::AstNode::Epsilon => true,
        crate::parser::AstNode::Char(_) => false,
        crate::parser::AstNode::Plus(inner) => contain_epsilon(inner),
        crate::parser::AstNode::Star(_) => true,
        crate::parser::AstNode::Question(_) => true,
        crate::parser::AstNode::Or(left, right) => contain_epsilon(left) || contain_epsilon(right),
        crate::parser::AstNode::Seq(left, right) => contain_epsilon(left) && contain_epsilon(right),
    }
}
