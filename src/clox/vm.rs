use super::{
    chunk::{Chunk, Opcode},
    debug,
    table::Table,
    value::Value,
};

pub struct Vm<'chunk, 'state> {
    chunk: &'chunk Chunk,
    state: &'state mut VmState,
    ip: usize,
    stack: Vec<Value>,
}

#[derive(Default)]
pub struct VmState {
    globals: Table<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Undefined variable {0}.")]
    UndefinedVariable(String),
    #[error("Global name isn't a string")]
    NonStringGlobalName,
    #[error("Operand muust be a number.")]
    ExpectedNumber,
    #[error("Unknown opcode: {0:#x}")]
    UnknownOpcode(u8),
}

#[derive(Debug, thiserror::Error)]
#[error("[line {line}] {kind}")]
pub struct Error {
    kind: ErrorKind,
    line: usize,
}

type Result<T = ()> = std::result::Result<T, Error>;

enum ControlFlow {
    Return,
}

impl<'chunk, 'state> Vm<'chunk, 'state> {
    pub fn new(chunk: &'chunk Chunk, state: &'state mut VmState) -> Self {
        Self {
            chunk,
            state,
            ip: 0,
            stack: vec![],
        }
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        self.chunk.constants.values[self.read_byte() as usize].clone()
    }

    fn read_constant_long(&mut self) -> Value {
        let mut bytes = [0; std::mem::size_of::<usize>()];
        for b in &mut bytes[..3] {
            *b = self.read_byte();
        }
        let index = usize::from_le_bytes(bytes);
        self.chunk.constants.values[index].clone()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value)
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| self.report(ErrorKind::StackUnderflow))
    }

    fn top(&self) -> Result<&Value> {
        self.stack
            .last()
            .ok_or_else(|| self.report(ErrorKind::StackUnderflow))
    }

    fn get_global(&mut self, name: Value) -> Result {
        if let Some(name) = name.into_obj_string() {
            match self.state.globals.get(&name) {
                Some(value) => {
                    let value = value.clone();
                    self.push(value);
                    Ok(())
                }
                None => Err(self.report(ErrorKind::UndefinedVariable(
                    name.0.clone().into_string(),
                ))),
            }
        } else {
            Err(self.report(ErrorKind::NonStringGlobalName))
        }
    }

    fn define_global(&mut self, name: Value) -> Result {
        let value = self.pop()?;
        if let Some(name) = name.into_obj_string() {
            self.state.globals.insert(name, value);
            Ok(())
        } else {
            Err(self.report(ErrorKind::NonStringGlobalName))
        }
    }

    fn set_global(&mut self, name: Value) -> Result {
        if let Some(name) = name.into_obj_string() {
            let top = self.top()?.clone();
            match self.state.globals.get_mut(&name) {
                Some(global) => {
                    *global = top;
                    Ok(())
                }
                None => Err(self.report(ErrorKind::UndefinedVariable(
                    name.0.clone().into_string(),
                ))),
            }
        } else {
            Err(self.report(ErrorKind::NonStringGlobalName))
        }
    }

    fn bin_op(&mut self, op: impl Fn(f64, f64) -> Value) -> Result {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                self.push(op(a, b));
                Ok(())
            }
            _ => Err(self.report(ErrorKind::ExpectedNumber)),
        }
    }

    fn report(&self, kind: ErrorKind) -> Error {
        let line = self.chunk.get_line(self.ip - 1).unwrap_or(0);
        Error { kind, line }
    }

    pub fn interpret(&mut self, debug: bool) -> Result {
        if debug {
            debug::disassembly_chunk(self.chunk, "code");
            println!("---- execution ----");
        }
        loop {
            if debug {
                println!("{:?}", self.stack);
                debug::disassembly_instruction(self.chunk, self.ip);
            }
            if let Some(ControlFlow::Return) = self.step()? {
                return Ok(());
            }
        }
    }

    fn step(&mut self) -> Result<Option<ControlFlow>> {
        let instruction = self.read_byte();
        match Opcode::check(instruction) {
            Some(Opcode::Constant) => {
                let constant = self.read_constant();
                self.push(constant);
            }
            Some(Opcode::ConstantLong) => {
                let constant = self.read_constant_long();
                self.push(constant);
            }
            Some(Opcode::Nil) => self.push(Value::nil()),
            Some(Opcode::True) => self.push(Value::bool(true)),
            Some(Opcode::False) => self.push(Value::bool(false)),
            Some(Opcode::Pop) => {
                self.pop()?;
            }
            Some(Opcode::GetGlobal) => {
                let name = self.read_constant();
                self.get_global(name)?;
            }
            Some(Opcode::GetGlobalLong) => {
                let name = self.read_constant_long();
                self.get_global(name)?;
            }
            Some(Opcode::DefineGlobal) => {
                let name = self.read_constant();
                self.define_global(name)?;
            }
            Some(Opcode::DefineGlobalLong) => {
                let name = self.read_constant_long();
                self.define_global(name)?;
            }
            Some(Opcode::SetGlobal) => {
                let name = self.read_constant();
                self.set_global(name)?;
            }
            Some(Opcode::SetGlobalLong) => {
                let name = self.read_constant_long();
                self.set_global(name)?;
            }
            Some(Opcode::Equal) => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::bool(a == b))
            }
            Some(Opcode::Greater) => self.bin_op(|l, r| Value::bool(l > r))?,
            Some(Opcode::Less) => self.bin_op(|l, r| Value::bool(l < r))?,
            Some(Opcode::Add) => {
                let b = self.pop()?;
                let a = self.pop()?;
                if let (Value::Number(a), Value::Number(b)) = (&a, &b) {
                    self.push(Value::number(a + b))
                } else if let (Some(a), Some(b)) =
                    (a.into_string(), b.into_string())
                {
                    self.push(Value::string(a + &b))
                }
            }
            Some(Opcode::Subtract) => {
                self.bin_op(|l, r| Value::number(l - r))?
            }
            Some(Opcode::Multiply) => {
                self.bin_op(|l, r| Value::number(l * r))?
            }
            Some(Opcode::Divide) => self.bin_op(|l, r| Value::number(l / r))?,
            Some(Opcode::Not) => {
                let value = self.pop()?;
                self.push(Value::bool(value.is_falsey()))
            }
            Some(Opcode::Negate) => {
                let value = self.pop()?;
                match value {
                    Value::Number(value) => {
                        self.push(Value::number(-value));
                    }
                    _ => return Err(self.report(ErrorKind::ExpectedNumber)),
                }
            }
            Some(Opcode::Print) => {
                println!("{:?}", self.pop()?)
            }
            Some(Opcode::Return) => return Ok(Some(ControlFlow::Return)),
            None => {
                return Err(self.report(ErrorKind::UnknownOpcode(instruction)))
            }
        }
        Ok(None)
    }
}
