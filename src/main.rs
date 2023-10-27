use mp4::marshall::{Decode, Encode, File};

fn main() {
    let mmap = unsafe {
        memmap2::Mmap::map(
            &std::fs::File::open(r#"test.mp4"#)
                .unwrap(),
        )
    }
    .unwrap();
    let mut input: &[u8] = &mmap;
    let file = File::decode(&mut input).unwrap();
    file.encode(&mut std::fs::File::create("test2.mp4").unwrap())
        .unwrap();
    println!("{:#?}", file);
}
