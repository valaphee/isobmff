mod file;
mod track;
mod movie;
mod media;

extern crate core;

use std::fs::File;
use std::io::{Write};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use thiserror::Error;
use crate::file::FileBox;

fn main() {
    let mmap = unsafe { memmap2::Mmap::map(&File::open(r#"Z:\Valaphee\Videos\Balto - Auf der Spur der Wölfe.mp4"#).unwrap()) }.unwrap();
    let mut input: &[u8] = &mmap;
    let file = FileBox::decode(&mut input).unwrap();
    println!("{:#?}", file);
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Encode {
    fn encode(&self, output: &mut impl Write) -> Result<()>;
}

pub trait Decode: Sized {
    fn decode(input: &mut &[u8]) -> Result<Self>;
}

#[derive(Debug)]
struct FreeSpaceBox(u32);

impl Encode for FreeSpaceBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8 + self.0)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"free"))?;

        for _ in 0..self.0 {
            output.write_u8(0)?;
        }

        Ok(())
    }
}

impl Decode for FreeSpaceBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self(input.len() as u32))
    }
}

#[derive(Debug)]
pub struct HandlerBox {
    pub handler_type: u32,
}

impl Encode for HandlerBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"hdlr"))?;

        Ok(())
    }
}

impl Decode for HandlerBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        input.read_u32::<BigEndian>()?;
        let handler_type = input.read_u32::<BigEndian>()?;
        input.read_u32::<BigEndian>()?;
        input.read_u32::<BigEndian>()?;
        input.read_u32::<BigEndian>()?;

        println!("{}", std::str::from_utf8( &handler_type.to_ne_bytes()).unwrap());

        Ok(Self {
            handler_type,
        })
    }
}
