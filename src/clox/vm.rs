use crate::clox::value::Obj;

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
            (Some(Value::Number(a)), Some(Value::Number(b))) => {
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
            match Opcode::check(instruction) {
                Some(Opcode::Constant) => {
                    let constant = self.read_constant();
                    self.state.stack.push(constant);
                }
                Some(Opcode::ConstantLong) => {
                    let constant = self.read_constant_long();
                    self.state.stack.push(constant);
                }
                Some(Opcode::Nil) => self.state.stack.push(Value::nil()),
                Some(Opcode::True) => self.state.stack.push(Value::bool(true)),
                Some(Opcode::False) => {
                    self.state.stack.push(Value::bool(false))
                }
                Some(Opcode::Equal) => {
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
                Some(Opcode::Greater) => {
                    self.bin_op(|l, r| Value::bool(l > r))?
                }
                Some(Opcode::Less) => self.bin_op(|l, r| Value::bool(l < r))?,
                Some(Opcode::Add) => {
                    let b = self.state.stack.pop();
                    let a = self.state.stack.pop();
                    match (a, b) {
                        (Some(Value::Number(a)), Some(Value::Number(b))) => {
                            self.state.stack.push(Value::number(a + b))
                        }
                        (Some(Value::Obj(a)), Some(Value::Obj(b))) => {
                            match (*a, *b) {
                                (Obj::String(a), Obj::String(b)) => {
                                    self.state.stack.push(Value::string(a + &b))
                                }
                            }
                        }
                        _ => {
                            return Err(self.runtime_error(
                                "Operands must be two numbers or two strings.",
                            ))
                        }
                    }
                }
                Some(Opcode::Subtract) => {
                    self.bin_op(|l, r| Value::number(l - r))?
                }
                Some(Opcode::Multiply) => {
                    self.bin_op(|l, r| Value::number(l * r))?
                }
                Some(Opcode::Divide) => {
                    self.bin_op(|l, r| Value::number(l / r))?
                }
                Some(Opcode::Not) => {
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
                Some(Opcode::Negate) => {
                    let value = self.state.stack.pop();
                    match value {
                        Some(Value::Number(value)) => {
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
                Some(Opcode::Return) => {
                    println!("{:?}", self.state.stack.pop().unwrap());
                    return Ok(());
                }
                None => return Err(self.runtime_error("Unimplemented opcode")),
            }
        }
    }
}
