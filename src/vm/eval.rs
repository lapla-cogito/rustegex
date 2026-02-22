use crate::vm::instruction::Instruction;

fn add_state_mask(inst: &[Instruction], pc: usize, mask: &mut u64) {
    if pc >= inst.len() {
        return;
    }
    let bit = 1u64 << pc;
    if *mask & bit != 0 {
        return;
    }
    *mask |= bit;

    match inst[pc] {
        Instruction::Split(x, y) => {
            add_state_mask(inst, x, mask);
            add_state_mask(inst, y, mask);
        }
        Instruction::Jmp(x) => {
            add_state_mask(inst, x, mask);
        }
        _ => {}
    }
}

fn for_each_set_bit(mut mask: u64, mut f: impl FnMut(usize)) {
    while mask != 0 {
        let pc = mask.trailing_zeros() as usize;
        f(pc);
        mask &= mask - 1; // clear lowest set bit
    }
}

#[inline(never)]
fn pike_eval_bitmask(inst: &[Instruction], input: &str) -> bool {
    let mut current: u64 = 0;
    add_state_mask(inst, 0, &mut current);

    if input.is_ascii() {
        // ascii fast path
        for &byte in input.as_bytes() {
            if current == 0 {
                return false;
            }
            let mut next: u64 = 0;
            for_each_set_bit(current, |pc| {
                if let Instruction::Char(expected) = inst[pc]
                    && expected as u32 <= 127
                    && expected as u8 == byte
                {
                    add_state_mask(inst, pc + 1, &mut next);
                }
            });
            current = next;
        }
    } else {
        for ch in input.chars() {
            if current == 0 {
                return false;
            }
            let mut next: u64 = 0;
            for_each_set_bit(current, |pc| {
                if let Instruction::Char(expected) = inst[pc]
                    && expected == ch
                {
                    add_state_mask(inst, pc + 1, &mut next);
                }
            });
            current = next;
        }
    }

    let mut found = false;
    for_each_set_bit(current, |pc| {
        if matches!(inst[pc], Instruction::Match) {
            found = true;
        }
    });
    found
}

fn add_state_vec(
    inst: &[Instruction],
    pc: usize,
    list: &mut Vec<usize>,
    gen_arr: &mut [u32],
    cur_gen: u32,
) {
    if pc >= inst.len() {
        return;
    }
    // SAFETY: pc < inst.len() and gen_arr.len() >= inst.len()
    let slot = unsafe { gen_arr.get_unchecked_mut(pc) };
    if *slot == cur_gen {
        return;
    }
    *slot = cur_gen;

    match *unsafe { inst.get_unchecked(pc) } {
        Instruction::Split(x, y) => {
            add_state_vec(inst, x, list, gen_arr, cur_gen);
            add_state_vec(inst, y, list, gen_arr, cur_gen);
        }
        Instruction::Jmp(x) => {
            add_state_vec(inst, x, list, gen_arr, cur_gen);
        }
        _ => {
            list.push(pc);
        }
    }
}

struct PikeBuffers {
    current: Vec<usize>,
    next: Vec<usize>,
    gen_arr: Vec<u32>,
    gen_counter: u32,
}

impl PikeBuffers {
    fn new(cap: usize) -> Self {
        PikeBuffers {
            current: Vec::with_capacity(cap),
            next: Vec::with_capacity(cap),
            gen_arr: vec![0u32; cap],
            gen_counter: 0,
        }
    }

    #[inline(always)]
    fn ensure_capacity(&mut self, program_size: usize) {
        if self.gen_arr.len() < program_size {
            self.gen_arr.resize(program_size, 0);
        }
    }

    #[inline(always)]
    fn next_gen(&mut self) -> u32 {
        self.gen_counter = self.gen_counter.wrapping_add(1);
        if self.gen_counter == 0 {
            self.gen_arr.fill(0);
            self.gen_counter = 1;
        }
        self.gen_counter
    }
}

thread_local! {
    static BUFFERS: std::cell::RefCell<PikeBuffers> = std::cell::RefCell::new(PikeBuffers::new(32));
}

#[inline(never)]
fn pike_eval_vec(inst: &[Instruction], input: &str) -> bool {
    let program_size = inst.len();

    BUFFERS.with(|cell| {
        let bufs = &mut *cell.borrow_mut();
        bufs.ensure_capacity(program_size);
        bufs.current.clear();
        bufs.next.clear();

        let g = bufs.next_gen();
        add_state_vec(inst, 0, &mut bufs.current, &mut bufs.gen_arr, g);

        if input.is_ascii() {
            for &byte in input.as_bytes() {
                if bufs.current.is_empty() {
                    return false;
                }
                let g = bufs.next_gen();
                let len = bufs.current.len();
                for i in 0..len {
                    let pc = *unsafe { bufs.current.get_unchecked(i) };
                    if let Instruction::Char(expected) = *unsafe { inst.get_unchecked(pc) }
                        && expected as u32 <= 127
                        && expected as u8 == byte
                    {
                        add_state_vec(inst, pc + 1, &mut bufs.next, &mut bufs.gen_arr, g);
                    }
                }
                std::mem::swap(&mut bufs.current, &mut bufs.next);
                bufs.next.clear();
            }
        } else {
            for ch in input.chars() {
                if bufs.current.is_empty() {
                    return false;
                }
                let g = bufs.next_gen();
                let len = bufs.current.len();
                for i in 0..len {
                    let pc = *unsafe { bufs.current.get_unchecked(i) };
                    if let Instruction::Char(expected) = *unsafe { inst.get_unchecked(pc) }
                        && expected == ch
                    {
                        add_state_vec(inst, pc + 1, &mut bufs.next, &mut bufs.gen_arr, g);
                    }
                }
                std::mem::swap(&mut bufs.current, &mut bufs.next);
                bufs.next.clear();
            }
        }

        bufs.current
            .iter()
            .any(|&pc| matches!(inst[pc], Instruction::Match))
    })
}

pub fn eval(
    inst: &[crate::vm::instruction::Instruction],
    input: &str,
    _input_looking: usize,
    _pc: usize,
) -> bool {
    let program_size = inst.len();
    if program_size == 0 {
        return false;
    }
    if program_size <= 64 {
        pike_eval_bitmask(inst, input)
    } else {
        pike_eval_vec(inst, input)
    }
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

    #[test]
    fn evaluation_empty() {
        let mut lexer = crate::lexer::Lexer::new("a*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "", 0, 0));
        assert!(eval(&inst, "a", 0, 0));
        assert!(eval(&inst, "aaa", 0, 0));
        assert!(!eval(&inst, "b", 0, 0));
    }

    #[test]
    fn evaluation_complex() {
        let mut lexer = crate::lexer::Lexer::new("ab(cd|)ef");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "abcdef", 0, 0));
        assert!(eval(&inst, "abef", 0, 0));
        assert!(!eval(&inst, "abc", 0, 0));
    }

    #[test]
    fn evaluation_unicode() {
        let mut lexer = crate::lexer::Lexer::new("正規表現(太郎|次郎)");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "正規表現太郎", 0, 0));
        assert!(eval(&inst, "正規表現次郎", 0, 0));
        assert!(!eval(&inst, "正規表現三郎", 0, 0));
    }

    #[test]
    fn evaluation_long_input() {
        let mut lexer = crate::lexer::Lexer::new("a+b");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(!eval(&inst, &"a".repeat(10000), 0, 0));
        assert!(eval(&inst, &format!("{}b", "a".repeat(10000)), 0, 0));
    }

    #[test]
    fn evaluation_empty_string() {
        let mut lexer = crate::lexer::Lexer::new("a*");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "", 0, 0));
    }

    #[test]
    fn evaluation_concat() {
        let mut lexer = crate::lexer::Lexer::new("abc");
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.instructions().to_vec();
        assert!(eval(&inst, "abc", 0, 0));
        assert!(!eval(&inst, "ab", 0, 0));
        assert!(!eval(&inst, "abcd", 0, 0));
        assert!(!eval(&inst, "", 0, 0));
    }
}
