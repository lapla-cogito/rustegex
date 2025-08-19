fn _eval(
    inst: &[crate::vm::instruction::Instruction],
    input: &Vec<char>,
    mut input_looking: usize,
    mut pc: usize,
    cache: &mut super::cache::Cache,
) -> bool {
    loop {
        if pc >= inst.len() || cache.contains(input_looking, pc) {
            return false;
        }
        cache.insert(input_looking, pc);

        match inst[pc] {
            crate::vm::instruction::Instruction::Char(c) => {
                if input_looking >= input.len() || input[input_looking] != c {
                    return false;
                }

                input_looking += 1;
                pc += 1;
            }
            crate::vm::instruction::Instruction::Split(x, y) => {
                if _eval(inst, input, input_looking, x, cache) {
                    return true;
                }

                pc = y;
            }
            crate::vm::instruction::Instruction::Jmp(x) => {
                pc = x;
            }
            crate::vm::instruction::Instruction::Match => return input_looking == input.len(),
        }
    }
}

pub fn eval(
    inst: &[crate::vm::instruction::Instruction],
    input: &Vec<char>,
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
        assert!(eval(&inst.to_vec(), &"a".chars().collect(), 0, 0));
        assert!(eval(&inst.to_vec(), &"b".chars().collect(), 0, 0));
        assert!(eval(&inst.to_vec(), &"bb".chars().collect(), 0, 0));
        assert!(!eval(&inst.to_vec(), &"c".chars().collect(), 0, 0));

        let mut lexer = crate::lexer::Lexer::new("a|b+");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst.to_vec(), &"a".chars().collect(), 0, 0));
        assert!(eval(&inst.to_vec(), &"b".chars().collect(), 0, 0));
        assert!(eval(&inst.to_vec(), &"bb".chars().collect(), 0, 0));
        assert!(!eval(&inst.to_vec(), &"c".chars().collect(), 0, 0));

        let mut lexer = crate::lexer::Lexer::new("a|b?");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst.to_vec(), &"a".chars().collect(), 0, 0));
        assert!(eval(&inst.to_vec(), &"b".chars().collect(), 0, 0));
        assert!(!eval(&inst.to_vec(), &"bb".chars().collect(), 0, 0));
        assert!(!eval(&inst.to_vec(), &"c".chars().collect(), 0, 0));
    }
}
