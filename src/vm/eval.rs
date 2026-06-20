fn for_each_set_bit(mut mask: u64, mut f: impl FnMut(usize)) {
    while mask != 0 {
        let pc = mask.trailing_zeros() as usize;
        f(pc);
        mask &= mask - 1;
    }
}

#[inline(never)]
fn pike_eval_bitmask(inst: &crate::vm::instruction::Program, input: &str) -> bool {
    let mut current: u64 = inst.epsilon_mask(0);

    if input.is_ascii() {
        for &byte in input.as_bytes() {
            if current == 0 {
                return false;
            }
            let mut next: u64 = 0;
            for_each_set_bit(current, |pc| match inst.opcode(pc) {
                crate::vm::instruction::OP_CHAR => {
                    let expected = inst.operand1(pc);
                    if expected <= 127 && expected as u8 == byte {
                        next |= inst.epsilon_mask(pc + 1);
                    }
                }
                crate::vm::instruction::OP_CLASS if inst.char_class(pc).matches(byte as char) => {
                    next |= inst.epsilon_mask(pc + 1);
                }
                _ => {}
            });
            current = next;
        }
    } else {
        for ch in input.chars() {
            if current == 0 {
                return false;
            }
            let mut next: u64 = 0;
            for_each_set_bit(current, |pc| match inst.opcode(pc) {
                crate::vm::instruction::OP_CHAR => {
                    if inst.char_literal(pc) == ch {
                        next |= inst.epsilon_mask(pc + 1);
                    }
                }
                crate::vm::instruction::OP_CLASS if inst.char_class(pc).matches(ch) => {
                    next |= inst.epsilon_mask(pc + 1);
                }
                _ => {}
            });
            current = next;
        }
    }

    let mut found = false;
    for_each_set_bit(current, |pc| {
        if inst.opcode(pc) == crate::vm::instruction::OP_MATCH {
            found = true;
        }
    });
    found
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

fn extend_epsilon_list(
    inst: &crate::vm::instruction::Program,
    pc: usize,
    list: &mut Vec<usize>,
    gen_arr: &mut [u32],
    cur_gen: u32,
) {
    for &target in inst.epsilon_list(pc) {
        let slot = unsafe { gen_arr.get_unchecked_mut(target) };
        if *slot != cur_gen {
            *slot = cur_gen;
            list.push(target);
        }
    }
}

thread_local! {
    static BUFFERS: std::cell::RefCell<PikeBuffers> = std::cell::RefCell::new(PikeBuffers::new(32));
}

#[inline(never)]
fn pike_eval_vec(inst: &crate::vm::instruction::Program, input: &str) -> bool {
    let program_size = inst.len();

    BUFFERS.with(|cell| {
        let bufs = &mut *cell.borrow_mut();
        bufs.ensure_capacity(program_size);
        bufs.current.clear();
        bufs.next.clear();

        let g = bufs.next_gen();
        extend_epsilon_list(inst, 0, &mut bufs.current, &mut bufs.gen_arr, g);

        if input.is_ascii() {
            for &byte in input.as_bytes() {
                if bufs.current.is_empty() {
                    return false;
                }
                let g = bufs.next_gen();
                let len = bufs.current.len();
                for i in 0..len {
                    let pc = *unsafe { bufs.current.get_unchecked(i) };
                    match inst.opcode(pc) {
                        crate::vm::instruction::OP_CHAR => {
                            let expected = inst.operand1(pc);
                            if expected <= 127 && expected as u8 == byte {
                                extend_epsilon_list(
                                    inst,
                                    pc + 1,
                                    &mut bufs.next,
                                    &mut bufs.gen_arr,
                                    g,
                                );
                            }
                        }
                        crate::vm::instruction::OP_CLASS
                            if inst.char_class(pc).matches(byte as char) =>
                        {
                            extend_epsilon_list(inst, pc + 1, &mut bufs.next, &mut bufs.gen_arr, g);
                        }
                        _ => {}
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
                    match inst.opcode(pc) {
                        crate::vm::instruction::OP_CHAR => {
                            if inst.char_literal(pc) == ch {
                                extend_epsilon_list(
                                    inst,
                                    pc + 1,
                                    &mut bufs.next,
                                    &mut bufs.gen_arr,
                                    g,
                                );
                            }
                        }
                        crate::vm::instruction::OP_CLASS if inst.char_class(pc).matches(ch) => {
                            extend_epsilon_list(inst, pc + 1, &mut bufs.next, &mut bufs.gen_arr, g);
                        }
                        _ => {}
                    }
                }
                std::mem::swap(&mut bufs.current, &mut bufs.next);
                bufs.next.clear();
            }
        }

        bufs.current
            .iter()
            .any(|&pc| inst.opcode(pc) == crate::vm::instruction::OP_MATCH)
    })
}

pub fn eval(
    inst: &crate::vm::instruction::Program,
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

    fn compile_and_eval(pattern: &str, input: &str) -> bool {
        let mut lexer = crate::lexer::Lexer::new(pattern);
        let mut parser = crate::parser::Parser::new(&mut lexer);
        let ast = parser.parse().unwrap();
        let mut compiler = crate::vm::compile::Compiler::new();
        compiler.compile(ast).unwrap();
        let inst = compiler.finish();
        eval(&inst, input, 0, 0)
    }

    #[test]
    fn evaluation() {
        assert!(compile_and_eval("a|b*", "a"));
        assert!(compile_and_eval("a|b*", "b"));
        assert!(compile_and_eval("a|b*", "bb"));
        assert!(!compile_and_eval("a|b*", "c"));

        assert!(compile_and_eval("a|b+", "a"));
        assert!(compile_and_eval("a|b+", "b"));
        assert!(compile_and_eval("a|b+", "bb"));
        assert!(!compile_and_eval("a|b+", "c"));

        assert!(compile_and_eval("a|b?", "a"));
        assert!(compile_and_eval("a|b?", "b"));
        assert!(!compile_and_eval("a|b?", "bb"));
        assert!(!compile_and_eval("a|b?", "c"));
    }

    #[test]
    fn evaluation_empty() {
        assert!(compile_and_eval("a*", ""));
        assert!(compile_and_eval("a*", "a"));
        assert!(compile_and_eval("a*", "aaa"));
        assert!(!compile_and_eval("a*", "b"));
    }

    #[test]
    fn evaluation_complex() {
        assert!(compile_and_eval("ab(cd|)ef", "abcdef"));
        assert!(compile_and_eval("ab(cd|)ef", "abef"));
        assert!(!compile_and_eval("ab(cd|)ef", "abc"));
    }

    #[test]
    fn evaluation_unicode() {
        assert!(compile_and_eval("正規表現(太郎|次郎)", "正規表現太郎"));
        assert!(compile_and_eval("正規表現(太郎|次郎)", "正規表現次郎"));
        assert!(!compile_and_eval("正規表現(太郎|次郎)", "正規表現三郎"));
    }

    #[test]
    fn evaluation_long_input() {
        assert!(!compile_and_eval("a+b", &"a".repeat(10000)));
        assert!(compile_and_eval("a+b", &format!("{}b", "a".repeat(10000))));
    }

    #[test]
    fn evaluation_empty_string() {
        assert!(compile_and_eval("a*", ""));
    }

    #[test]
    fn evaluation_concat() {
        assert!(compile_and_eval("abc", "abc"));
        assert!(!compile_and_eval("abc", "ab"));
        assert!(!compile_and_eval("abc", "abcd"));
        assert!(!compile_and_eval("abc", ""));
    }
}
