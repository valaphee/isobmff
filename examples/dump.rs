use memmap2::Mmap;

use mp4::marshal::{mp4::File, Decode};

fn main() {
    let mmap = unsafe {
        Mmap::map(
            &std::fs::File::open(r#"C:\Users\valaphee\Videos\2023-10-28 15-05-30.mp4"#).unwrap(),
        )
    }
    .unwrap();
    let mp4 = File::decode(&mut mmap.as_ref()).unwrap();
    println!("{:#?}", mp4);
}
