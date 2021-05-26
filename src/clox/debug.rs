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
    let constant = chunk.constants.values[index as usize];
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
    let constant = chunk.constants.values[index as usize];
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
    match instruction {
        Opcode::CONSTANT => constant_instruction("OP_CONSTANT", chunk, offset),
        Opcode::CONSTANT_LONG => {
            constant_long_instruction("OP_CONSTANT_LONG", chunk, offset)
        }
        Opcode::ADD => simple_instruction("OP_ADD", offset),
        Opcode::SUBSTRACT => simple_instruction("OP_SUBSTRACT", offset),
        Opcode::MULTIPLY => simple_instruction("OP_MULTIPLY", offset),
        Opcode::DIVIDE => simple_instruction("OP_DIVIDE", offset),
        Opcode::NEGATE => simple_instruction("OP_NEGATE", offset),
        Opcode::RETURN => simple_instruction("OP_RETURN", offset),
        _ => {
            println!("Unknown opcode {}", instruction);
            offset + 1
        }
    }
}
