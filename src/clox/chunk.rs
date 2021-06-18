use super::value::{Value, ValueArray};

macro_rules! opcodes {
    ( $vis:vis enum $name:ident { $($variant:ident ( $const:ident ),)* } ) => {
        $vis enum $name {
            $($variant,)*
        }

        impl $name {
            $($vis const $const: u8 = $name::$variant as u8;)*

            $vis fn check(value: u8) -> Option<Self> {
                match value {
                    $(Self::$const => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

opcodes!(
    pub enum Opcode {
        Constant(CONSTANT),
        ConstantLong(CONSTANT_LONG),
        Nil(NIL),
        True(TRUE),
        False(FALSE),
        Pop(POP),
        GetGlobal(GET_GLOBAL),
        GetGlobalLong(GET_GLOBAL_LONG),
        DefineGlobal(DEFINE_GLOBAL),
        DefineGlobalLong(DEFINE_GLOBAL_LONG),
        SetGlobal(SET_GLOBAL),
        SetGlobalLong(SET_GLOBAL_LONG),

        Equal(EQUAL),
        Greater(GREATER),
        Less(LESS),
        Add(ADD),
        Subtract(SUBTRACT),
        Multiply(MULTIPLY),
        Divide(DIVIDE),
        Not(NOT),
        Negate(NEGATE),

        Print(PRINT),
        Return(RETURN),
    }
);

#[derive(Default)]
pub struct Chunk {
    pub(super) code: Vec<u8>,
    lines: Lines,
    pub(super) constants: ValueArray,
}

#[derive(Clone, Copy)]
pub struct ConstantIndex(usize);

impl Chunk {
    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn get_line(&self, offset: usize) -> Option<usize> {
        self.lines.get_line(offset)
    }

    pub fn add_constant(&mut self, value: Value) -> ConstantIndex {
        let position = self.constants.values.iter().position(|x| *x == value);
        ConstantIndex(if let Some(i) = position {
            i
        } else {
            self.constants.write(value);
            self.constants.values.len() - 1
        })
    }

    fn write_op_with_constant(
        &mut self,
        op_short: u8,
        op_long: u8,
        constant: ConstantIndex,
        line: usize,
    ) {
        let ConstantIndex(constant) = constant;
        if constant <= 0xff {
            self.write(op_short, line);
            self.write(constant as u8, line);
        } else if constant <= 0xff_ffff {
            self.write(op_long, line);
            for &x in constant.to_le_bytes()[..3].iter() {
                self.write(x, line);
            }
        } else {
            panic!("index too big for constant: {}", constant);
        }
    }

    pub fn write_constant(
        &mut self,
        value: Value,
        line: usize,
    ) -> ConstantIndex {
        let index = self.add_constant(value);
        self.write_op_with_constant(
            Opcode::CONSTANT,
            Opcode::CONSTANT_LONG,
            index,
            line,
        );
        index
    }
    pub fn set_global(&mut self, name: ConstantIndex, line: usize) {
        self.write_op_with_constant(
            Opcode::SET_GLOBAL,
            Opcode::SET_GLOBAL_LONG,
            name,
            line,
        );
    }

    pub fn define_global(&mut self, name: ConstantIndex, line: usize) {
        self.write_op_with_constant(
            Opcode::DEFINE_GLOBAL,
            Opcode::DEFINE_GLOBAL_LONG,
            name,
            line,
        );
    }

    pub fn get_global(&mut self, name: ConstantIndex, line: usize) {
        self.write_op_with_constant(
            Opcode::GET_GLOBAL,
            Opcode::GET_GLOBAL_LONG,
            name,
            line,
        );
    }
}

#[derive(Default)]
struct Lines {
    line_lens: Vec<u8>,
    lines: Vec<usize>,
}

impl Lines {
    pub fn push(&mut self, line: usize) {
        match self.line_lens.last_mut().zip(self.lines.last_mut()) {
            Some((len, line_no)) if *len < 255 && line == *line_no => *len += 1,
            _ => {
                self.line_lens.push(1);
                self.lines.push(line);
            }
        }
    }

    pub fn get_line(&self, offset: usize) -> Option<usize> {
        let mut counter = 0usize;
        for (len, line) in self.line_lens.iter().zip(&self.lines) {
            if (counter..counter + *len as usize).contains(&offset) {
                return Some(*line);
            }
            counter += *len as usize;
        }
        None
    }
}
