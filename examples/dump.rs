use memmap2::Mmap;

use mp4::marshal::{base::File, Decode};

fn main() {
    let mmap = unsafe { Mmap::map(&std::fs::File::open(r#""#).unwrap()) }.unwrap();
    let mp4 = File::decode(&mut mmap.as_ref()).unwrap();
    println!("{:#?}", mp4);
}
