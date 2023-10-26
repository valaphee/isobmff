use std::io::Write;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U8F8};
use crate::{Decode, Encode, Result};
use crate::track::TrackBox;

// 8.1
#[derive(Debug)]
pub struct MovieBox {
    pub header: MovieHeaderBox,
    pub tracks: Vec<TrackBox>,
}

impl Encode for MovieBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"moov"))?;

        Ok(())
    }
}

impl Decode for MovieBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = Default::default();
        let mut tracks = Vec::default();
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();
            println!("File > Movie {} {}", size, std::str::from_utf8(&r#type).unwrap());

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"mvhd" => header = Some(Decode::decode(&mut data)?),
                b"trak" => tracks.push(Decode::decode(&mut data)?),
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            header: header.unwrap(),
            tracks
        })
    }
}

// 8.3
#[derive(Debug)]
pub struct MovieHeaderBox {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub rate: U16F16,
    pub volume: U8F8,
    pub matrix: [u32; 9],
    pub next_track_id: u32,
}

impl Encode for MovieHeaderBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mvhd"))?;

        Ok(())
    }
}

impl Decode for MovieHeaderBox {
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
        let rate = U16F16::from_bits(input.read_u32::<BigEndian>()?);
        let volume = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        input.read_u16::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
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
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        input.read_u32::<BigEndian>()?; // reserved
        let next_track_id = input.read_u32::<BigEndian>()?;

        Ok(Self {
            creation_time,
            modification_time,
            timescale,
            duration,
            rate,
            volume,
            matrix,
            next_track_id,
        })
    }
}
