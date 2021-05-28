use super::{
    chunk::{Chunk, Opcode},
    debug,
    value::{Number, Value},
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
pub enum ErrorKind {
    #[error("Compile error")]
    Compile,
    #[error("Runtime error")]
    Runtime,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}: {1}")]
pub struct Error(ErrorKind, String);

impl Error {
    fn runtime(msg: impl Into<String>) -> Self {
        Self(ErrorKind::Runtime, msg.into())
    }
}

type Result = std::result::Result<(), Error>;

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
        self.chunk.constants.values[self.read_byte() as usize].clone()
    }

    fn read_constant_long(&mut self) -> Value {
        let mut bytes = [0; std::mem::size_of::<usize>()];
        for i in 0..3 {
            bytes[i] = self.read_byte();
        }
        let index = usize::from_le_bytes(bytes);
        self.chunk.constants.values[index].clone()
    }

    fn bin_op(&mut self, op: impl Fn(f64, f64) -> Value) -> Result {
        let b = self.state.stack.pop();
        let a = self.state.stack.pop();
        match (a, b) {
            (
                Some(Value::Number(Number(a))),
                Some(Value::Number(Number(b))),
            ) => {
                self.state.stack.push(op(a, b));
                Ok(())
            }
            (Some(_), Some(_)) => {
                Err(self.runtime_error("Operands must be numbers."))
            }
            (None, Some(_)) => {
                Err(self.runtime_error("Missing operand on stack"))
            }
            _ => Err(self.runtime_error("Missing operands on stack")),
        }
    }

    fn runtime_error(&self, msg: impl AsRef<str>) -> Error {
        let line = self.chunk.get_line(self.state.ip - 1).unwrap_or(0);
        Error::runtime(format!("[line {}] {}", line, msg.as_ref()))
    }

    pub fn interpret(&mut self) -> Result {
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
                Opcode::NIL => self.state.stack.push(Value::nil()),
                Opcode::TRUE => self.state.stack.push(Value::bool(true)),
                Opcode::FALSE => self.state.stack.push(Value::bool(false)),
                Opcode::EQUAL => {
                    let b = self.state.stack.pop();
                    let a = self.state.stack.pop();
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            self.state.stack.push(Value::bool(a == b))
                        }
                        _ => {
                            return Err(
                                self.runtime_error("Missing operand on stack")
                            )
                        }
                    }
                }
                Opcode::GREATER => self.bin_op(|l, r| Value::bool(l > r))?,
                Opcode::LESS => self.bin_op(|l, r| Value::bool(l < r))?,
                Opcode::ADD => self.bin_op(|l, r| Value::number(l + r))?,
                Opcode::SUBSTRACT => {
                    self.bin_op(|l, r| Value::number(l - r))?
                }
                Opcode::MULTIPLY => self.bin_op(|l, r| Value::number(l * r))?,
                Opcode::DIVIDE => self.bin_op(|l, r| Value::number(l / r))?,
                Opcode::NOT => {
                    let value = self.state.stack.pop();
                    match value {
                        Some(value) => self
                            .state
                            .stack
                            .push(Value::bool(value.is_falsey())),
                        None => {
                            return Err(
                                self.runtime_error("Missing operand on stack")
                            )
                        }
                    }
                }
                Opcode::NEGATE => {
                    let value = self.state.stack.pop();
                    match value {
                        Some(Value::Number(Number(value))) => {
                            self.state.stack.push(Value::number(-value));
                        }
                        Some(_) => {
                            return Err(
                                self.runtime_error("Operand must be a number")
                            )
                        }
                        None => {
                            return Err(
                                self.runtime_error("Missing operand on stack")
                            )
                        }
                    }
                }
                Opcode::RETURN => {
                    println!("{:?}", self.state.stack.pop().unwrap());
                    return Ok(());
                }
                _ => return Err(self.runtime_error("Unimplemented opcode")),
            }
        }
    }
}
