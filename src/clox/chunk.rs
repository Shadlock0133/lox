use super::value::{Value, ValueArray};

macro_rules! opcodes {
    ( $vis:vis enum $name:ident { $($variant:ident ( $const:ident ),)* } ) => {
        $vis enum $name {
            $($variant,)*
        }

        impl $name {
            $($vis const $const: u8 = $name::$variant as u8;)*
        }
    };
}

opcodes!(
    pub enum Opcode {
        Constant(CONSTANT),
        ConstantLong(CONSTANT_LONG),

        Add(ADD),
        Substract(SUBSTRACT),
        Multiply(MULTIPLY),
        Divide(DIVIDE),
        Negate(NEGATE),

        Return(RETURN),
    }
);

#[derive(Default)]
pub struct Chunk {
    pub(super) code: Vec<u8>,
    line_lens: Vec<u8>,
    lines: Vec<usize>,
    pub(super) constants: ValueArray,
}

impl Chunk {
    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
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

    pub fn add_constant(&mut self, value: Value) -> usize {
        let position = self.constants.values.iter().position(|x| *x == value);
        if let Some(i) = position {
            i
        } else {
            self.constants.write(value);
            self.constants.values.len() - 1
        }
    }

    pub fn write_constant(&mut self, value: Value, line: usize) {
        let index = self.add_constant(value);
        if index <= 0xff {
            self.write(Opcode::CONSTANT, line);
            self.write(index as u8, line);
        } else if index <= 0xff_ffff {
            self.write(Opcode::CONSTANT_LONG, line);
            for &x in index.to_le_bytes()[..3].iter() {
                self.write(x, line);
            }
        } else {
            panic!("index too big for constant: {}", index);
        }
    }
}
