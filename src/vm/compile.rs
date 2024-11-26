#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiler {
    pc: usize,
    instructions: Vec<crate::vm::instruction::Instruction>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            pc: 0,
            instructions: Vec::new(),
        }
    }

    pub fn instructions(&self) -> &Vec<crate::vm::instruction::Instruction> {
        &self.instructions
    }

    fn emit(&mut self, instruction: crate::vm::instruction::Instruction) {
        self.instructions.push(instruction);
        self.pc += 1;
    }

    fn patch(&mut self, pc: usize, instruction: crate::vm::instruction::Instruction) {
        self.instructions[pc] = instruction;
    }

    fn _compile(&mut self, ast: crate::parser::AstNode) -> crate::Result<()> {
        match ast {
            crate::parser::AstNode::Char(c) => {
                self.emit(crate::vm::instruction::Instruction::Char(c));
            }
            crate::parser::AstNode::Plus(node) => {
                let split = self.pc;
                self.emit(crate::vm::instruction::Instruction::Split(0, 0));
                let start = self.pc;
                self._compile(*node)?;
                self.emit(crate::vm::instruction::Instruction::Jmp(split));
                let end = self.pc;
                self.patch(
                    split,
                    crate::vm::instruction::Instruction::Split(start, end),
                );
            }
            crate::parser::AstNode::Star(node) => {
                let split = self.pc;
                self.pc += 1;
                self.instructions
                    .push(crate::vm::instruction::Instruction::Split(self.pc, 0));
                self._compile(*node)?;
                self.pc += 1;
                self.instructions
                    .push(crate::vm::instruction::Instruction::Jmp(split));

                if let Some(crate::vm::instruction::Instruction::Split(_, expr)) =
                    self.instructions.get_mut(split)
                {
                    *expr = self.pc;
                } else {
                    return Err(crate::error::Error::CompileError);
                }
            }
            crate::parser::AstNode::Question(node) => {
                let split = self.pc;
                self.emit(crate::vm::instruction::Instruction::Split(0, 0));
                let start = self.pc;
                self._compile(*node)?;
                let end = self.pc;
                self.patch(
                    split,
                    crate::vm::instruction::Instruction::Split(start, end),
                );
            }
            crate::parser::AstNode::Or(left, right) => {
                let split = self.pc;
                self.pc += 1;
                self.instructions
                    .push(crate::vm::instruction::Instruction::Split(self.pc, 0));
                self._compile(*left)?;
                let jump = self.pc;
                self.emit(crate::vm::instruction::Instruction::Jmp(0));

                if let Some(crate::vm::instruction::Instruction::Split(_, expr)) =
                    self.instructions.get_mut(split)
                {
                    *expr = self.pc;
                } else {
                    return Err(crate::error::Error::CompileError);
                }

                self._compile(*right)?;
                if let Some(crate::vm::instruction::Instruction::Jmp(expr)) =
                    self.instructions.get_mut(jump)
                {
                    *expr = self.pc;
                } else {
                    return Err(crate::error::Error::CompileError);
                }
            }
            crate::parser::AstNode::Seq(nodes) => {
                if nodes.is_empty() {
                    return Err(crate::error::Error::CompileError);
                }

                for node in nodes {
                    self._compile(node)?;
                }
            }
            crate::parser::AstNode::Empty | crate::parser::AstNode::Epsilon => {}
        }

        Ok(())
    }

    pub fn compile(&mut self, ast: crate::parser::AstNode) -> crate::Result<()> {
        self._compile(ast)?;
        self.pc += 1;
        self.instructions
            .push(crate::vm::instruction::Instruction::Match);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile() {
        let mut lexer = crate::lexer::Lexer::new("a|b");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile(ast).unwrap();
        assert_eq!(
            compiler.instructions,
            vec![
                crate::vm::instruction::Instruction::Split(1, 3),
                crate::vm::instruction::Instruction::Char('a'),
                crate::vm::instruction::Instruction::Jmp(4),
                crate::vm::instruction::Instruction::Char('b'),
                crate::vm::instruction::Instruction::Match,
            ]
        );

        let mut lexer = crate::lexer::Lexer::new("aa*bb*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile(ast).unwrap();
        assert_eq!(
            compiler.instructions,
            vec![
                crate::vm::instruction::Instruction::Char('a'),
                crate::vm::instruction::Instruction::Split(2, 4),
                crate::vm::instruction::Instruction::Char('a'),
                crate::vm::instruction::Instruction::Jmp(1),
                crate::vm::instruction::Instruction::Char('b'),
                crate::vm::instruction::Instruction::Split(6, 8),
                crate::vm::instruction::Instruction::Char('b'),
                crate::vm::instruction::Instruction::Jmp(5),
                crate::vm::instruction::Instruction::Match,
            ]
        );

        let mut lexer = crate::lexer::Lexer::new("a|b*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile(ast).unwrap();
        assert_eq!(
            compiler.instructions,
            vec![
                crate::vm::instruction::Instruction::Split(1, 3),
                crate::vm::instruction::Instruction::Char('a'),
                crate::vm::instruction::Instruction::Jmp(6),
                crate::vm::instruction::Instruction::Split(4, 6),
                crate::vm::instruction::Instruction::Char('b'),
                crate::vm::instruction::Instruction::Jmp(3),
                crate::vm::instruction::Instruction::Match,
            ]
        );
    }
}
