use super::{chunk::Chunk, scanner::Scanner};

pub fn compile(source: &str) -> Chunk {
    let mut scanner = Scanner::new(&source);
    let mut line = 0;
    while let Some(token) = scanner.next().unwrap() {
        if token.line != line {
            line = token.line;
            print!("{:4} ", line);
        } else {
            print!("   | ");
        }
        println!("{:?} {}", token.type_, token.lexeme);
    }
    todo!()
}
