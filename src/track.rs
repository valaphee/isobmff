use std::io::Write;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U8F8};
use crate::{Decode, Encode, Result};
use crate::media::MediaBox;

// 8.4
#[derive(Debug)]
pub struct TrackBox {
    pub header: TrackHeaderBox,
    pub media: MediaBox,
}

impl Encode for TrackBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"trak"))?;

        Ok(())
    }
}

impl Decode for TrackBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = Default::default();
        let mut media = Default::default();
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();
            println!("File > Movie > Track {} {}", size, std::str::from_utf8(&r#type).unwrap());

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"tkhd" => header = Some(Decode::decode(&mut data)?),
                b"mdia" => media = Some(Decode::decode(&mut data)?),
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            header: header.unwrap(),
            media: media.unwrap()
        })
    }
}

// 8.5
#[derive(Debug)]
pub struct TrackHeaderBox {
    pub creation_time: u64,
    pub modification_time: u64,
    pub track_id: u32,
    pub duration: u64,
    pub layer: u16,
    pub alternate_group: u16,
    pub volume: U8F8,
    pub matrix: [u32; 9],
    pub width: U16F16,
    pub height: U16F16,
}

impl Encode for TrackHeaderBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"tkhd"))?;

        Ok(())
    }
}

impl Decode for TrackHeaderBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        let creation_time;
        let modification_time;
        let track_id;
        let duration;
        match version {
            0 => {
                creation_time = input.read_u32::<BigEndian>()? as u64;
                modification_time = input.read_u32::<BigEndian>()? as u64;
                track_id = input.read_u32::<BigEndian>()?;
                input.read_u32::<BigEndian>()?; // reserved
                duration = input.read_u32::<BigEndian>()? as u64;
            }
            1 => {
                creation_time = input.read_u64::<BigEndian>()?;
                modification_time = input.read_u64::<BigEndian>()?;
                track_id = input.read_u32::<BigEndian>()?;
                input.read_u32::<BigEndian>()?; // reserved
                duration = input.read_u64::<BigEndian>()?;
            }
            _ => todo!()
        }
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        let layer = input.read_u16::<BigEndian>()?;
        let alternate_group = input.read_u16::<BigEndian>()?;
        let volume = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        input.read_u16::<BigEndian>()?; // reserved
        let matrix = [
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
            input.read_u32::<BigEndian>()?,
        ];
        let width = U16F16::from_bits(input.read_u32::<BigEndian>()?);
        let height = U16F16::from_bits(input.read_u32::<BigEndian>()?);

        Ok(Self {
            creation_time,
            modification_time,
            track_id,
            duration,
            layer,
            alternate_group,
            volume,
            matrix,
            width,
            height,
        })
    }
}
