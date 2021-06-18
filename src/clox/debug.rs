use super::chunk::{Chunk, Opcode};

pub fn disassembly_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassembly_instruction(chunk, offset);
    }
}

fn bytes(chunk: &Chunk, offset: usize, size: usize) {
    for i in 0..4 {
        if i < size {
            print!("{:02x} ", chunk.code[offset + i]);
        } else {
            print!("   ");
        }
    }
}

fn simple_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    bytes(chunk, offset, 1);
    println!("{}", name);
    offset + 1
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    bytes(chunk, offset, 2);
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
    bytes(chunk, offset, 4);
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
        Some(Opcode::Nil) => simple_instruction("OP_NIL", chunk, offset),
        Some(Opcode::True) => simple_instruction("OP_TRUE", chunk, offset),
        Some(Opcode::False) => simple_instruction("OP_FALSE", chunk, offset),
        Some(Opcode::Pop) => simple_instruction("OP_POP", chunk, offset),
        Some(Opcode::GetGlobal) => {
            constant_instruction("OP_GET_GLOBAL", chunk, offset)
        }
        Some(Opcode::GetGlobalLong) => {
            constant_long_instruction("OP_GET_GLOBAL_LONG", chunk, offset)
        }
        Some(Opcode::DefineGlobal) => {
            constant_instruction("OP_DEFINE_GLOBAL", chunk, offset)
        }
        Some(Opcode::DefineGlobalLong) => {
            constant_long_instruction("OP_DEFINE_GLOBAL_LONG", chunk, offset)
        }
        Some(Opcode::SetGlobal) => {
            constant_instruction("OP_SET_GLOBAL", chunk, offset)
        }
        Some(Opcode::SetGlobalLong) => {
            constant_long_instruction("OP_SET_GLOBAL_LONG", chunk, offset)
        }

        Some(Opcode::Equal) => simple_instruction("OP_EQUAL", chunk, offset),
        Some(Opcode::Greater) => {
            simple_instruction("OP_GREATER", chunk, offset)
        }
        Some(Opcode::Less) => simple_instruction("OP_LESS", chunk, offset),
        Some(Opcode::Add) => simple_instruction("OP_ADD", chunk, offset),
        Some(Opcode::Subtract) => {
            simple_instruction("OP_SUBSTRACT", chunk, offset)
        }
        Some(Opcode::Multiply) => {
            simple_instruction("OP_MULTIPLY", chunk, offset)
        }
        Some(Opcode::Divide) => simple_instruction("OP_DIVIDE", chunk, offset),
        Some(Opcode::Not) => simple_instruction("OP_NOT", chunk, offset),
        Some(Opcode::Negate) => simple_instruction("OP_NEGATE", chunk, offset),

        Some(Opcode::Print) => simple_instruction("OP_PRINT", chunk, offset),
        Some(Opcode::Return) => simple_instruction("OP_RETURN", chunk, offset),
        None => {
            println!("Unknown opcode {}", instruction);
            offset + 1
        }
    }
}
