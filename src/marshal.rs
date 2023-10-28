use std::{
    fmt::{Debug, Formatter},
    io::{Read, Seek, Write},
    str::FromStr,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U2F30, U8F8};
use fixed_macro::types::{U16F16, U2F30};
use thiserror::Error;

pub mod av1;
pub mod iso;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Invalid {r#type} box quantity: {quantity}, expected: {expected}")]
    InvalidBoxQuantity {
        r#type: &'static str,
        quantity: usize,
        expected: usize,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Encode {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()>;
}

pub trait Decode: Sized {
    fn decode(input: &mut &[u8]) -> Result<Self>;
}

impl Encode for u16 {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_u16::<BigEndian>(*self)?;
        Ok(())
    }
}

impl Decode for u16 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(input.read_u16::<BigEndian>()?)
    }
}

impl Encode for U8F8 {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_u16::<BigEndian>(self.to_bits())?;
        Ok(())
    }
}

impl Decode for U8F8 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self::from_bits(input.read_u16::<BigEndian>()?))
    }
}

impl Encode for u32 {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_u32::<BigEndian>(*self)?;
        Ok(())
    }
}

impl Decode for u32 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(input.read_u32::<BigEndian>()?)
    }
}

impl Encode for U16F16 {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_u32::<BigEndian>(self.to_bits())?;
        Ok(())
    }
}

impl Decode for U16F16 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self::from_bits(input.read_u32::<BigEndian>()?))
    }
}

impl Encode for U2F30 {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_u32::<BigEndian>(self.to_bits())?;
        Ok(())
    }
}

impl Decode for U2F30 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self::from_bits(input.read_u32::<BigEndian>()?))
    }
}

impl Encode for u64 {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_u64::<BigEndian>(*self)?;
        Ok(())
    }
}

impl Decode for u64 {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(input.read_u64::<BigEndian>()?)
    }
}

impl Encode for String {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        output.write_all(self.as_bytes())?;
        output.write_u8(0)?;
        Ok(())
    }
}

impl Decode for String {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let length = input.iter().position(|&c| c == 0).unwrap();
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

impl FromStr for FourCC {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(u32::from_be_bytes(s.as_bytes().try_into().unwrap())))
    }
}

#[derive(Debug)]
pub struct Matrix {
    pub a: U16F16,
    pub b: U16F16,
    pub u: U2F30,
    pub c: U16F16,
    pub d: U16F16,
    pub v: U2F30,
    pub x: U16F16,
    pub y: U16F16,
    pub w: U2F30,
}

impl Matrix {
    pub fn identity() -> Self {
        Self {
            a: U16F16!(1),
            b: U16F16!(0),
            u: U2F30!(0),
            c: U16F16!(0),
            d: U16F16!(1),
            v: U2F30!(0),
            x: U16F16!(0),
            y: U16F16!(0),
            w: U2F30!(1),
        }
    }
}

impl Encode for Matrix {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        self.a.encode(output)?;
        self.b.encode(output)?;
        self.u.encode(output)?;
        self.c.encode(output)?;
        self.d.encode(output)?;
        self.v.encode(output)?;
        self.x.encode(output)?;
        self.y.encode(output)?;
        self.w.encode(output)
    }
}

impl Decode for Matrix {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            a: Decode::decode(input)?,
            b: Decode::decode(input)?,
            u: Decode::decode(input)?,
            c: Decode::decode(input)?,
            d: Decode::decode(input)?,
            v: Decode::decode(input)?,
            x: Decode::decode(input)?,
            y: Decode::decode(input)?,
            w: Decode::decode(input)?,
        })
    }
}
