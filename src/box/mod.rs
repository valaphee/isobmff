use std::{
    fmt::{Debug, Formatter},
    io::Write,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U8F8};
use thiserror::Error;

pub mod file;
pub mod media;
pub mod movie;
pub mod sample_table;
pub mod track;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Encode {
    fn size(&self) -> u64;

    fn encode(&self, output: &mut impl Write) -> Result<()>;
}

pub trait Decode: Sized {
    fn decode(input: &mut &[u8]) -> Result<Self>;
}

impl Encode for u16 {
    fn size(&self) -> u64 {
        2
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u16::<BigEndian>(*self)?;
        Ok(())
    }
}

impl Decode for u16 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(input.read_u16::<BigEndian>()?)
    }
}

impl Encode for u32 {
    fn size(&self) -> u64 {
        4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(*self)?;
        Ok(())
    }
}

impl Decode for u32 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(input.read_u32::<BigEndian>()?)
    }
}

impl Encode for u64 {
    fn size(&self) -> u64 {
        8
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u64::<BigEndian>(*self)?;
        Ok(())
    }
}

impl Decode for u64 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(input.read_u64::<BigEndian>()?)
    }
}

impl Encode for U8F8 {
    fn size(&self) -> u64 {
        2
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u16::<BigEndian>(self.to_bits())?;
        Ok(())
    }
}

impl Decode for U8F8 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self::from_bits(input.read_u16::<BigEndian>()?))
    }
}

impl Encode for U16F16 {
    fn size(&self) -> u64 {
        4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(self.to_bits())?;
        Ok(())
    }
}

impl Decode for U16F16 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self::from_bits(input.read_u32::<BigEndian>()?))
    }
}

impl Encode for String {
    fn size(&self) -> u64 {
        if self.is_empty() {
            0
        } else {
            self.as_bytes().len() as u64 + 1
        }
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        if !self.is_empty() {
            output.write_all(self.as_bytes())?;
            output.write_u8(0)?;
        }
        Ok(())
    }
}

impl Decode for String {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let length = input.iter().position(|&c| c == 0).unwrap_or(0);
        let (data, remaining_data) = input.split_at(length);
        *input = remaining_data;
        Ok(String::from_utf8(data.to_owned()).unwrap())
    }
}

pub struct FourCC(u32);

impl Debug for FourCC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::str::from_utf8(&self.0.to_be_bytes()).unwrap())
    }
}

pub struct Language(u16);

impl Debug for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bytes = self.0.to_be_bytes();
        let c0 = (bytes[0] >> 2 & 0x1F) + 0x60;
        let c1 = (((bytes[0] & 0x3) << 3) | (bytes[1] >> 5)) + 0x60;
        let c2 = (bytes[1] & 0x1F) + 0x60;
        f.write_str(std::str::from_utf8(&[c0, c1, c2]).unwrap())
    }
}
