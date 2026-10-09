use r8asm::asm::{Assembler, Tokenizer};
fn main() {
 let args: Vec<_> = std::env::args().collect();
 let source = std::fs::read_to_string(&args[1]).unwrap();
 let mut assembler = Assembler::new(Tokenizer::new(&source).tokenize().unwrap());
 let bytes = assembler.assemble().unwrap();
 std::fs::write(&args[2], &bytes).unwrap();
 let mut labels: Vec<_> = assembler.labels.into_iter().collect();
 labels.sort_by_key(|(_, address)| *address);
 println!("SIZE {}", bytes.len());
 for (name, address) in labels { println!("{address:04X} {name}"); }
}
