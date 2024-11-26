#[derive(Debug, Clone, PartialEq)]
pub struct Derivative {
    ast: crate::parser::AstNode,
}

impl Derivative {
    pub fn new(ast: crate::parser::AstNode) -> Self {
        Derivative { ast }
    }

    pub fn is_match(&self, input: &str) -> bool {
        let mut ast = self.ast.clone();
        _match(&mut ast, input)
    }

    pub fn is_empty_match(&self) -> bool {
        is_contain_epsilon(&self.ast)
    }
}

fn _match(ast: &mut crate::parser::AstNode, input: &str) -> bool {
    for c in input.chars() {
        *ast = derivative(ast, c);
    }

    is_contain_epsilon(ast)
}

fn derivative(ast: &crate::parser::AstNode, c: char) -> crate::parser::AstNode {
    match ast {
        crate::parser::AstNode::Empty => crate::parser::AstNode::Empty,
        crate::parser::AstNode::Epsilon => crate::parser::AstNode::Empty,
        crate::parser::AstNode::Char(c1) => {
            if c == *c1 {
                crate::parser::AstNode::Epsilon
            } else {
                crate::parser::AstNode::Empty
            }
        }
        crate::parser::AstNode::Plus(inner) => {
            let tmp = derivative(inner, c);
            crate::parser::AstNode::Seq(
                Box::new(tmp),
                Box::new(crate::parser::AstNode::Star(inner.clone())),
            )
        }
        crate::parser::AstNode::Star(inner) => {
            let tmp = derivative(inner, c);
            crate::parser::AstNode::Seq(
                Box::new(tmp),
                Box::new(crate::parser::AstNode::Star(inner.clone())),
            )
        }
        crate::parser::AstNode::Question(inner) => derivative(inner, c),
        crate::parser::AstNode::Or(left, right) => {
            let left = derivative(left, c);
            let right = derivative(right, c);

            crate::parser::AstNode::Or(Box::new(left), Box::new(right))
        }
        crate::parser::AstNode::Seq(left, right) => {
            let d1 = derivative(left, c);
            let d2 = derivative(right, c);

            crate::parser::AstNode::Or(
                Box::new(crate::parser::AstNode::Seq(
                    Box::new(d1),
                    Box::new(*right.clone()),
                )),
                Box::new(crate::parser::AstNode::Seq(
                    Box::new(delta(left)),
                    Box::new(d2),
                )),
            )
        }
    }
}

fn delta(ast: &crate::parser::AstNode) -> crate::parser::AstNode {
    if is_contain_epsilon(ast) {
        crate::parser::AstNode::Epsilon
    } else {
        crate::parser::AstNode::Empty
    }
}

fn is_contain_epsilon(ast: &crate::parser::AstNode) -> bool {
    match ast {
        crate::parser::AstNode::Empty => false,
        crate::parser::AstNode::Epsilon => true,
        crate::parser::AstNode::Char(_) => false,
        crate::parser::AstNode::Plus(inner) => is_contain_epsilon(inner),
        crate::parser::AstNode::Star(_) => true,
        crate::parser::AstNode::Question(_) => true,
        crate::parser::AstNode::Or(left, right) => {
            is_contain_epsilon(left) || is_contain_epsilon(right)
        }
        crate::parser::AstNode::Seq(left, right) => {
            is_contain_epsilon(left) && is_contain_epsilon(right)
        }
    }
}
