pub struct Compiler {
    builder: crate::vm::instruction::ProgramBuilder,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            builder: crate::vm::instruction::ProgramBuilder::new(),
        }
    }

    pub fn finish(self) -> crate::vm::instruction::Program {
        self.builder.build()
    }

    fn _compile(&mut self, ast: crate::parser::AstNode) -> crate::Result<()> {
        match ast {
            crate::parser::AstNode::Char(c) => {
                self.builder.emit_char(c);
            }
            crate::parser::AstNode::Class(class) => {
                self.builder.emit_class(class);
            }
            crate::parser::AstNode::Plus(node) => {
                let split = self.builder.reserve_split();
                let start = self.builder.pc();
                self._compile(*node)?;
                self.builder.emit_jmp(split);
                let end = self.builder.pc();
                self.builder.patch_split(split, start, end);
            }
            crate::parser::AstNode::Star(node) => {
                let split = self.builder.pc();
                let body_start = split + 1;
                self.builder.emit_split(body_start, 0);
                self._compile(*node)?;
                self.builder.emit_jmp(split);
                let end = self.builder.pc();
                self.builder.patch_split_second(split, end);
            }
            crate::parser::AstNode::Question(node) => {
                let split = self.builder.reserve_split();
                let start = self.builder.pc();
                self._compile(*node)?;
                let end = self.builder.pc();
                self.builder.patch_split(split, start, end);
            }
            crate::parser::AstNode::Or(left, right) => {
                let split = self.builder.reserve_split();
                let start = self.builder.pc();
                self._compile(*left)?;
                let jump = self.builder.reserve_jmp();
                let right_start = self.builder.pc();
                self.builder.patch_split_second(split, right_start);
                self._compile(*right)?;
                self.builder.patch_jmp(jump, self.builder.pc());
                self.builder.patch_split(split, start, right_start);
            }
            crate::parser::AstNode::Seq(left, right) => {
                if let (crate::parser::AstNode::Epsilon, crate::parser::AstNode::Epsilon) =
                    (&*left, &*right)
                {
                    return Err(crate::error::Error::CompileError);
                }

                self._compile(*left)?;
                self._compile(*right)?;
            }
            crate::parser::AstNode::Empty | crate::parser::AstNode::Epsilon => {}
        }

        Ok(())
    }

    pub fn compile(&mut self, ast: crate::parser::AstNode) -> crate::Result<()> {
        self._compile(ast)?;
        self.builder.emit_match();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_pattern(pattern: &str) -> crate::vm::instruction::Program {
        let mut lexer = crate::lexer::Lexer::new(pattern);
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile(ast).unwrap();
        compiler.finish()
    }

    fn assert_program(program: &crate::vm::instruction::Program, expected: &[(u8, u32, u32)]) {
        assert_eq!(program.len(), expected.len());
        for (pc, &(opcode, op1, op2)) in expected.iter().enumerate() {
            assert_eq!(program.opcode(pc), opcode, "opcode mismatch at pc {pc}");
            assert_eq!(program.operand1(pc), op1, "op1 mismatch at pc {pc}");
            assert_eq!(program.operand2(pc), op2, "op2 mismatch at pc {pc}");
        }
    }

    #[test]
    fn compile() {
        assert_program(
            &compile_pattern("a|b"),
            &[
                (crate::vm::instruction::OP_SPLIT, 1, 3),
                (crate::vm::instruction::OP_CHAR, 'a' as u32, 0),
                (crate::vm::instruction::OP_JMP, 4, 0),
                (crate::vm::instruction::OP_CHAR, 'b' as u32, 0),
                (crate::vm::instruction::OP_MATCH, 0, 0),
            ],
        );

        assert_program(
            &compile_pattern("aa*bb*"),
            &[
                (crate::vm::instruction::OP_CHAR, 'a' as u32, 0),
                (crate::vm::instruction::OP_SPLIT, 2, 4),
                (crate::vm::instruction::OP_CHAR, 'a' as u32, 0),
                (crate::vm::instruction::OP_JMP, 1, 0),
                (crate::vm::instruction::OP_CHAR, 'b' as u32, 0),
                (crate::vm::instruction::OP_SPLIT, 6, 8),
                (crate::vm::instruction::OP_CHAR, 'b' as u32, 0),
                (crate::vm::instruction::OP_JMP, 5, 0),
                (crate::vm::instruction::OP_MATCH, 0, 0),
            ],
        );

        assert_program(
            &compile_pattern("a|b*"),
            &[
                (crate::vm::instruction::OP_SPLIT, 1, 3),
                (crate::vm::instruction::OP_CHAR, 'a' as u32, 0),
                (crate::vm::instruction::OP_JMP, 6, 0),
                (crate::vm::instruction::OP_SPLIT, 4, 6),
                (crate::vm::instruction::OP_CHAR, 'b' as u32, 0),
                (crate::vm::instruction::OP_JMP, 3, 0),
                (crate::vm::instruction::OP_MATCH, 0, 0),
            ],
        );
    }
}
