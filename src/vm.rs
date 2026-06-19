mod compile;
mod eval;
mod instruction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vm {
    bytecode: instruction::Program,
}

impl Vm {
    pub fn new(ast: crate::parser::AstNode) -> Result<Vm, crate::error::Error> {
        let mut compiler = compile::Compiler::new();
        compiler.compile(ast)?;

        Ok(Vm {
            bytecode: compiler.finish(),
        })
    }

    pub fn is_match(&self, input: &str) -> bool {
        eval::eval(&self.bytecode, input, 0, 0)
    }
}
