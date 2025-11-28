fn _eval(
    inst: &[crate::vm::instruction::Instruction],
    input: &str,
    mut input_looking: usize,
    mut pc: usize,
    cache: &mut super::cache::Cache,
) -> bool {
    let mut stack = Vec::new();

    loop {
        loop {
            if pc >= inst.len() || cache.contains(input_looking, pc) {
                break;
            }
            cache.insert(input_looking, pc);

            match inst[pc] {
                crate::vm::instruction::Instruction::Char(c) => {
                    if input_looking >= input.len() {
                        break;
                    }

                    if c.is_ascii() {
                        if input.as_bytes()[input_looking] == c as u8 {
                            input_looking += 1;
                            pc += 1;
                        } else {
                            break;
                        }
                    } else {
                        let ch = input[input_looking..].chars().next().unwrap();
                        if ch == c {
                            input_looking += ch.len_utf8();
                            pc += 1;
                        } else {
                            break;
                        }
                    }
                }
                crate::vm::instruction::Instruction::Split(x, y) => {
                    stack.push((y, input_looking));
                    pc = x;
                }
                crate::vm::instruction::Instruction::Jmp(x) => {
                    pc = x;
                }
                crate::vm::instruction::Instruction::Match => {
                    if input_looking == input.len() {
                        return true;
                    }
                    break;
                }
            }
        }

        if let Some((next_pc, next_input_looking)) = stack.pop() {
            pc = next_pc;
            input_looking = next_input_looking;
        } else {
            return false;
        }
    }
}

pub fn eval(
    inst: &[crate::vm::instruction::Instruction],
    input: &str,
    input_looking: usize,
    pc: usize,
) -> bool {
    let program_size = inst.len();
    let input_size = input.len();

    super::cache::with_thread_cache(program_size, input_size, |cache| {
        _eval(inst, input, input_looking, pc, cache)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation() {
        let mut lexer = crate::lexer::Lexer::new("a|b*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "a", 0, 0));
        assert!(eval(&inst, "b", 0, 0));
        assert!(eval(&inst, "bb", 0, 0));
        assert!(!eval(&inst, "c", 0, 0));

        let mut lexer = crate::lexer::Lexer::new("a|b+");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "a", 0, 0));
        assert!(eval(&inst, "b", 0, 0));
        assert!(eval(&inst, "bb", 0, 0));
        assert!(!eval(&inst, "c", 0, 0));

        let mut lexer = crate::lexer::Lexer::new("a|b?");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "a", 0, 0));
        assert!(eval(&inst, "b", 0, 0));
        assert!(!eval(&inst, "bb", 0, 0));
        assert!(!eval(&inst, "c", 0, 0));
    }
}
