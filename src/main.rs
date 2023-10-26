extern crate core;

use mp4::{Decode, File};

fn main() {
    let mmap = unsafe { memmap2::Mmap::map(&std::fs::File::open(r#"Z:\Valaphee\Videos\Balto - Auf der Spur der Wölfe.mp4"#).unwrap()) }.unwrap();
    let mut input: &[u8] = &mmap;
    let file = File::decode(&mut input).unwrap();
    println!("{:#?}", file);
}
