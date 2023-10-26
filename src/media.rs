use std::io::Write;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::{Decode, Encode, HandlerBox, Result};

// 8.7
#[derive(Debug)]
pub struct MediaBox {
    header: MediaHeaderBox,
    handler: HandlerBox,
}

impl Encode for MediaBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mdia"))?;

        Ok(())
    }
}

impl Decode for MediaBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = Default::default();
        let mut handler = Default::default();

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();
            println!("File > Movie > Track > Media {} {}", size, std::str::from_utf8(&r#type).unwrap());

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"mdhd" => header = Some(Decode::decode(&mut data)?),
                b"hdlr" => handler = Some(Decode::decode(&mut data)?),
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            header: header.unwrap(),
            handler: handler.unwrap(),
        })
    }
}

// 8.8
#[derive(Debug)]
pub struct MediaHeaderBox {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub language: u16,
}

impl Encode for MediaHeaderBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mdhd"))?;

        Ok(())
    }
}

impl Decode for MediaHeaderBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        let creation_time;
        let modification_time;
        let timescale;
        let duration;
        match version {
            0 => {
                creation_time = input.read_u32::<BigEndian>()? as u64;
                modification_time = input.read_u32::<BigEndian>()? as u64;
                timescale = input.read_u32::<BigEndian>()?;
                duration = input.read_u32::<BigEndian>()? as u64;
            }
            1 => {
                creation_time = input.read_u64::<BigEndian>()?;
                modification_time = input.read_u64::<BigEndian>()?;
                timescale = input.read_u32::<BigEndian>()?;
                duration = input.read_u64::<BigEndian>()?;
            }
            _ => todo!()
        }
        let language = input.read_u16::<BigEndian>()?;
        input.read_u16::<BigEndian>()?; // pre_defined

        Ok(Self {
            creation_time,
            modification_time,
            timescale,
            duration,
            language,
        })
    }
}
