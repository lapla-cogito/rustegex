#[derive(Debug, PartialEq, Eq, Hash)]
pub enum AstNode {
    Char(char),
    Plus(Box<AstNode>),
    Star(Box<AstNode>),
    Question(Box<AstNode>),
    Or(Box<AstNode>, Box<AstNode>),
    Seq(Box<AstNode>, Box<AstNode>),
    Empty,
    Epsilon,
}

impl Clone for AstNode {
    fn clone(&self) -> Self {
        match self {
            AstNode::Char(c) => AstNode::Char(*c),
            AstNode::Plus(node) => AstNode::Plus(Box::new(*node.clone())),
            AstNode::Star(node) => AstNode::Star(Box::new(*node.clone())),
            AstNode::Question(node) => AstNode::Question(Box::new(*node.clone())),
            AstNode::Or(left, right) => {
                AstNode::Or(Box::new(*left.clone()), Box::new(*right.clone()))
            }
            AstNode::Seq(left, right) => {
                AstNode::Seq(Box::new(*left.clone()), Box::new(*right.clone()))
            }
            AstNode::Empty => AstNode::Empty,
            AstNode::Epsilon => AstNode::Epsilon,
        }
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: &'a mut crate::lexer::Lexer<'a>,
    looking: crate::lexer::Token,
}

impl Parser<'_> {
    pub fn new<'a>(lexer: &'a mut crate::lexer::Lexer<'a>) -> Parser<'a> {
        let looking = lexer.scan();
        Parser { lexer, looking }
    }

    fn consume(&mut self, token: crate::lexer::Token) -> crate::Result<()> {
        match &self.looking {
            look if look == &token => {
                self.looking = self.lexer.scan();
                Ok(())
            }
            _ => Err(crate::Error::Expected(token)),
        }
    }

    pub fn parse(&mut self) -> crate::Result<AstNode> {
        let ast = self.parse_expr()?;

        if self.looking != crate::lexer::Token::Empty {
            return Err(crate::Error::UnexpectedChar(self.looking));
        }

        Ok(ast)
    }

    fn parse_expr(&mut self) -> crate::Result<AstNode> {
        let mut ast = if self.looking == crate::lexer::Token::RightParen {
            AstNode::Epsilon
        } else {
            self.parse_term()?
        };

        if self.looking == crate::lexer::Token::UnionOperator {
            self.consume(crate::lexer::Token::UnionOperator)?;
            let right = self.parse_expr()?;
            ast = AstNode::Or(Box::new(ast), Box::new(right));
        }

        Ok(ast)
    }

    fn parse_term(&mut self) -> crate::Result<AstNode> {
        let mut nodes = vec![];

        while !matches!(
            self.looking,
            crate::lexer::Token::RightParen
                | crate::lexer::Token::UnionOperator
                | crate::lexer::Token::Empty
        ) {
            nodes.push(self.parse_factor()?);
        }

        if nodes.is_empty() {
            Ok(AstNode::Epsilon)
        } else if nodes.len() == 1 {
            Ok(nodes.pop().unwrap())
        } else {
            let mut iter = nodes.into_iter();
            let left = iter.next().unwrap();
            let right = iter.next().unwrap();

            let mut ast = AstNode::Seq(Box::new(left), Box::new(right));

            for node in iter {
                ast = AstNode::Seq(Box::new(ast), Box::new(node));
            }

            Ok(ast)
        }
    }

    fn parse_factor(&mut self) -> crate::Result<AstNode> {
        let mut ast = self.parse_atom()?;

        match self.looking {
            crate::lexer::Token::PlusOperator => {
                self.consume(crate::lexer::Token::PlusOperator)?;
                ast = AstNode::Plus(Box::new(ast));
            }
            crate::lexer::Token::StarOperator => {
                self.consume(crate::lexer::Token::StarOperator)?;
                ast = AstNode::Star(Box::new(ast));
            }
            crate::lexer::Token::QuestionOperator => {
                self.consume(crate::lexer::Token::QuestionOperator)?;
                ast = AstNode::Question(Box::new(ast));
            }
            _ => {}
        }

        Ok(ast)
    }

    fn parse_atom(&mut self) -> crate::Result<AstNode> {
        match self.looking {
            crate::lexer::Token::Character(c) => {
                self.consume(crate::lexer::Token::Character(c))?;

                Ok(AstNode::Char(c))
            }
            crate::lexer::Token::LeftParen => {
                self.consume(crate::lexer::Token::LeftParen)?;
                let ast = self.parse_expr()?;
                self.consume(crate::lexer::Token::RightParen)?;

                Ok(ast)
            }
            _ => Err(crate::Error::UnexpectedChar(self.looking)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let mut lexer = crate::lexer::Lexer::new("a|b");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Or(Box::new(AstNode::Char('a')), Box::new(AstNode::Char('b')))
        );

        let mut lexer = crate::lexer::Lexer::new("a|b*");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Or(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Star(Box::new(AstNode::Char('b'))))
            )
        );

        let mut lexer = crate::lexer::Lexer::new("a|b+");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Or(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Plus(Box::new(AstNode::Char('b'))))
            )
        );

        let mut lexer = crate::lexer::Lexer::new("a|b?");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Or(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Question(Box::new(AstNode::Char('b'))))
            )
        );

        let mut lexer = crate::lexer::Lexer::new("a|b|c");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Or(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Or(
                    Box::new(AstNode::Char('b')),
                    Box::new(AstNode::Char('c'))
                ))
            )
        );

        let mut lexer = crate::lexer::Lexer::new("a(b|c)");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Seq(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Or(
                    Box::new(AstNode::Char('b')),
                    Box::new(AstNode::Char('c'))
                ))
            )
        );

        let mut lexer = crate::lexer::Lexer::new("((a|b)+)*");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Star(Box::new(AstNode::Plus(Box::new(AstNode::Or(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Char('b'))
            )))))
        );

        let mut lexer = crate::lexer::Lexer::new("a|b*|c?");
        let mut parser = Parser::new(&mut lexer);
        assert_eq!(
            parser.parse().unwrap(),
            AstNode::Or(
                Box::new(AstNode::Char('a')),
                Box::new(AstNode::Or(
                    Box::new(AstNode::Star(Box::new(AstNode::Char('b')))),
                    Box::new(AstNode::Question(Box::new(AstNode::Char('c'))))
                ))
            )
        );
    }
}
