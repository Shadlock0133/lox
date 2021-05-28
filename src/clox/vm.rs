use super::{
    chunk::{Chunk, Opcode},
    debug,
    value::Value,
};

pub struct Vm<'chunk, 'state> {
    chunk: &'chunk Chunk,
    state: &'state mut VmState,
    debug: bool,
}

#[derive(Default)]
pub struct VmState {
    ip: usize,
    stack: Vec<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum InterpretError {
    #[error("Compile error")]
    Compile,
    #[error("Runtime error")]
    Runtime,
}

type InterpretResult = Result<(), InterpretError>;

impl<'chunk, 'state> Vm<'chunk, 'state> {
    pub fn new(
        chunk: &'chunk Chunk,
        state: &'state mut VmState,
        debug: bool,
    ) -> Self {
        Self {
            chunk,
            state,
            debug,
        }
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.state.ip];
        self.state.ip += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        self.chunk.constants.values[self.read_byte() as usize]
    }

    fn read_constant_long(&mut self) -> Value {
        let mut bytes = [0; std::mem::size_of::<usize>()];
        for i in 0..3 {
            bytes[i] = self.read_byte();
        }
        let index = usize::from_le_bytes(bytes);
        self.chunk.constants.values[index]
    }

    fn bin_op(&mut self, op: impl Fn(Value, Value) -> Value) {
        let b = self.state.stack.pop().unwrap();
        let a = self.state.stack.pop().unwrap();
        self.state.stack.push(op(a, b));
    }

    pub fn interpret(&mut self) -> InterpretResult {
        self.state.ip = 0;
        if self.debug {
            debug::disassembly_chunk(self.chunk, "code");
        }
        loop {
            if self.debug {
                println!("{:?}", self.state.stack);
                debug::disassembly_instruction(self.chunk, self.state.ip);
            }
            let instruction = self.read_byte();
            match instruction {
                Opcode::CONSTANT => {
                    let constant = self.read_constant();
                    self.state.stack.push(constant);
                }
                Opcode::CONSTANT_LONG => {
                    let constant = self.read_constant_long();
                    self.state.stack.push(constant);
                }
                Opcode::ADD => self.bin_op(|l, r| Value(l.0 + r.0)),
                Opcode::SUBSTRACT => self.bin_op(|l, r| Value(l.0 - r.0)),
                Opcode::MULTIPLY => self.bin_op(|l, r| Value(l.0 * r.0)),
                Opcode::DIVIDE => self.bin_op(|l, r| Value(l.0 / r.0)),
                Opcode::NEGATE => {
                    let value = self.state.stack.pop().unwrap();
                    self.state.stack.push(Value(-value.0));
                }
                Opcode::RETURN => {
                    println!("{:?}", self.state.stack.pop().unwrap());
                    return Ok(());
                }
                _ => return Err(InterpretError::Runtime),
            }
        }
    }
}
