use super::chunk::{Chunk, Opcode};

pub fn disassembly_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassembly_instruction(chunk, offset);
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let index = chunk.code[offset + 1];
    let constant = &chunk.constants.values[index as usize];
    println!("{:16} {:4} '{:?}'", name, index, constant);
    offset + 2
}

fn constant_long_instruction(
    name: &str,
    chunk: &Chunk,
    offset: usize,
) -> usize {
    let mut bytes = [0; std::mem::size_of::<usize>()];
    for i in 0..3 {
        bytes[i] = chunk.code[offset + i + 1];
    }
    let index = usize::from_le_bytes(bytes);
    let constant = &chunk.constants.values[index as usize];
    println!("{:16} {:4} '{:?}'", name, index, constant);
    offset + 4
}

pub fn disassembly_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);
    if offset > 0 && chunk.get_line(offset) == chunk.get_line(offset - 1) {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.get_line(offset).unwrap());
    }

    let instruction = chunk.code[offset];
    match Opcode::check(instruction) {
        Some(Opcode::Constant) => {
            constant_instruction("OP_CONSTANT", chunk, offset)
        }
        Some(Opcode::ConstantLong) => {
            constant_long_instruction("OP_CONSTANT_LONG", chunk, offset)
        }
        Some(Opcode::Nil) => simple_instruction("OP_NIL", offset),
        Some(Opcode::True) => simple_instruction("OP_TRUE", offset),
        Some(Opcode::False) => simple_instruction("OP_FALSE", offset),

        Some(Opcode::Equal) => simple_instruction("OP_EQUAL", offset),
        Some(Opcode::Greater) => simple_instruction("OP_GREATER", offset),
        Some(Opcode::Less) => simple_instruction("OP_LESS", offset),
        Some(Opcode::Add) => simple_instruction("OP_ADD", offset),
        Some(Opcode::Substract) => simple_instruction("OP_SUBSTRACT", offset),
        Some(Opcode::Multiply) => simple_instruction("OP_MULTIPLY", offset),
        Some(Opcode::Divide) => simple_instruction("OP_DIVIDE", offset),
        Some(Opcode::Not) => simple_instruction("OP_NOT", offset),
        Some(Opcode::Negate) => simple_instruction("OP_NEGATE", offset),

        Some(Opcode::Return) => simple_instruction("OP_RETURN", offset),
        None => {
            println!("Unknown opcode {}", instruction);
            offset + 1
        }
    }
}
