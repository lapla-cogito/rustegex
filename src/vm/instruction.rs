pub const OP_CHAR: u8 = 0;
pub const OP_SPLIT: u8 = 1;
pub const OP_JMP: u8 = 2;
pub const OP_MATCH: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    opcodes: Vec<u8>,
    op1: Vec<u32>,
    op2: Vec<u32>,
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

    #[inline(always)]
    pub fn operand2(&self, pc: usize) -> u32 {
        *unsafe { self.op2.get_unchecked(pc) }
    }

    #[inline(always)]
    pub fn char_literal(&self, pc: usize) -> char {
        unsafe { char::from_u32_unchecked(self.operand1(pc)) }
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
        Program {
            opcodes: self.opcodes,
            op1: self.op1,
            op2: self.op2,
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
