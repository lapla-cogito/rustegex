pub const OP_CHAR: u8 = 0;
pub const OP_SPLIT: u8 = 1;
pub const OP_JMP: u8 = 2;
pub const OP_MATCH: u8 = 3;
pub const OP_CLASS: u8 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    opcodes: Vec<u8>,
    op1: Vec<u32>,
    op2: Vec<u32>,
    epsilon_masks: Vec<u64>,
    epsilon_lists: Vec<Vec<usize>>,
}

impl Program {
    pub fn len(&self) -> usize {
        self.opcodes.len()
    }

    #[inline(always)]
    pub fn opcode(&self, pc: usize) -> u8 {
        *unsafe { self.opcodes.get_unchecked(pc) }
    }

    #[inline(always)]
    pub fn operand1(&self, pc: usize) -> u32 {
        *unsafe { self.op1.get_unchecked(pc) }
    }

    #[cfg(test)]
    pub fn operand2(&self, pc: usize) -> u32 {
        *unsafe { self.op2.get_unchecked(pc) }
    }

    #[inline(always)]
    pub fn char_literal(&self, pc: usize) -> char {
        unsafe { char::from_u32_unchecked(self.operand1(pc)) }
    }

    #[inline(always)]
    pub fn char_class(&self, pc: usize) -> crate::charclass::CharClass {
        match self.operand1(pc) {
            0 => crate::charclass::CharClass::Any,
            1 => crate::charclass::CharClass::Digit,
            2 => crate::charclass::CharClass::Word,
            _ => crate::charclass::CharClass::Space,
        }
    }

    #[inline(always)]
    pub fn epsilon_mask(&self, pc: usize) -> u64 {
        *unsafe { self.epsilon_masks.get_unchecked(pc) }
    }

    #[inline(always)]
    pub fn epsilon_list(&self, pc: usize) -> &[usize] {
        unsafe { self.epsilon_lists.get_unchecked(pc) }
    }
}

pub struct ProgramBuilder {
    opcodes: Vec<u8>,
    op1: Vec<u32>,
    op2: Vec<u32>,
}

impl ProgramBuilder {
    pub fn new() -> Self {
        ProgramBuilder {
            opcodes: Vec::new(),
            op1: Vec::new(),
            op2: Vec::new(),
        }
    }

    pub fn pc(&self) -> usize {
        self.opcodes.len()
    }

    pub fn build(self) -> Program {
        let n = self.opcodes.len();
        let (epsilon_masks, epsilon_lists) = if n <= 64 {
            (
                compute_epsilon_masks(&self.opcodes, &self.op1, &self.op2, n),
                Vec::new(),
            )
        } else {
            (
                Vec::new(),
                compute_epsilon_lists(&self.opcodes, &self.op1, &self.op2, n),
            )
        };

        Program {
            opcodes: self.opcodes,
            op1: self.op1,
            op2: self.op2,
            epsilon_masks,
            epsilon_lists,
        }
    }

    fn emit(&mut self, opcode: u8, operand1: u32, operand2: u32) {
        self.opcodes.push(opcode);
        self.op1.push(operand1);
        self.op2.push(operand2);
    }

    fn patch(&mut self, pc: usize, operand1: u32, operand2: u32) {
        self.op1[pc] = operand1;
        self.op2[pc] = operand2;
    }

    pub fn emit_char(&mut self, c: char) {
        self.emit(OP_CHAR, c as u32, 0);
    }

    pub fn emit_class(&mut self, class: crate::charclass::CharClass) {
        let id = match class {
            crate::charclass::CharClass::Any => 0,
            crate::charclass::CharClass::Digit => 1,
            crate::charclass::CharClass::Word => 2,
            crate::charclass::CharClass::Space => 3,
        };
        self.emit(OP_CLASS, id, 0);
    }

    pub fn emit_split(&mut self, x: usize, y: usize) {
        self.emit(OP_SPLIT, x as u32, y as u32);
    }

    pub fn emit_jmp(&mut self, target: usize) {
        self.emit(OP_JMP, target as u32, 0);
    }

    pub fn emit_match(&mut self) {
        self.emit(OP_MATCH, 0, 0);
    }

    pub fn reserve_split(&mut self) -> usize {
        let pc = self.pc();
        self.emit_split(0, 0);
        pc
    }

    pub fn reserve_jmp(&mut self) -> usize {
        let pc = self.pc();
        self.emit_jmp(0);
        pc
    }

    pub fn patch_split(&mut self, pc: usize, x: usize, y: usize) {
        self.patch(pc, x as u32, y as u32);
    }

    pub fn patch_split_second(&mut self, pc: usize, y: usize) {
        self.op2[pc] = y as u32;
    }

    pub fn patch_jmp(&mut self, pc: usize, target: usize) {
        self.op1[pc] = target as u32;
    }
}

fn compute_epsilon_masks(opcodes: &[u8], op1: &[u32], op2: &[u32], n: usize) -> Vec<u64> {
    let mut masks = vec![0u64; n];
    for (start, mask) in masks.iter_mut().enumerate() {
        fill_epsilon_mask(start, mask, opcodes, op1, op2, n);
    }
    masks
}

fn fill_epsilon_mask(
    pc: usize,
    mask: &mut u64,
    opcodes: &[u8],
    op1: &[u32],
    op2: &[u32],
    n: usize,
) {
    if pc >= n {
        return;
    }
    let bit = 1u64 << pc;
    if *mask & bit != 0 {
        return;
    }
    *mask |= bit;

    match opcodes[pc] {
        OP_SPLIT => {
            let x = op1[pc] as usize;
            let y = op2[pc] as usize;
            fill_epsilon_mask(x, mask, opcodes, op1, op2, n);
            fill_epsilon_mask(y, mask, opcodes, op1, op2, n);
        }
        OP_JMP => {
            let x = op1[pc] as usize;
            fill_epsilon_mask(x, mask, opcodes, op1, op2, n);
        }
        _ => {}
    }
}

fn compute_epsilon_lists(opcodes: &[u8], op1: &[u32], op2: &[u32], n: usize) -> Vec<Vec<usize>> {
    let mut lists = Vec::with_capacity(n);
    let mut visited = vec![false; n];
    for start in 0..n {
        let mut list = Vec::new();
        visited.fill(false);
        fill_epsilon_list(start, &mut list, &mut visited, opcodes, op1, op2, n);
        lists.push(list);
    }
    lists
}

fn fill_epsilon_list(
    pc: usize,
    list: &mut Vec<usize>,
    visited: &mut [bool],
    opcodes: &[u8],
    op1: &[u32],
    op2: &[u32],
    n: usize,
) {
    if pc >= n {
        return;
    }
    if visited[pc] {
        return;
    }
    visited[pc] = true;

    match opcodes[pc] {
        OP_SPLIT => {
            let x = op1[pc] as usize;
            let y = op2[pc] as usize;
            fill_epsilon_list(x, list, visited, opcodes, op1, op2, n);
            fill_epsilon_list(y, list, visited, opcodes, op1, op2, n);
        }
        OP_JMP => {
            let x = op1[pc] as usize;
            fill_epsilon_list(x, list, visited, opcodes, op1, op2, n);
        }
        _ => {
            list.push(pc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program_from_ops(ops: &[(u8, u32, u32)]) -> Program {
        let mut builder = ProgramBuilder::new();
        for &(opcode, a, b) in ops {
            match opcode {
                OP_CHAR => builder.emit_char(char::from_u32(a).unwrap()),
                OP_SPLIT => builder.emit_split(a as usize, b as usize),
                OP_JMP => builder.emit_jmp(a as usize),
                OP_MATCH => builder.emit_match(),
                _ => panic!("unknown opcode"),
            }
        }
        builder.build()
    }

    #[test]
    fn epsilon_closure_masks() {
        let program = program_from_ops(&[
            (OP_SPLIT, 1, 3),
            (OP_CHAR, 'a' as u32, 0),
            (OP_JMP, 4, 0),
            (OP_CHAR, 'b' as u32, 0),
            (OP_MATCH, 0, 0),
        ]);

        assert_eq!(program.epsilon_mask(0), 0b01011);
        assert_eq!(program.epsilon_mask(1), 1 << 1);
        assert_eq!(program.epsilon_mask(2), 0b10100);
        assert_eq!(program.epsilon_mask(3), 1 << 3);
        assert_eq!(program.epsilon_mask(4), 1 << 4);
    }
}
