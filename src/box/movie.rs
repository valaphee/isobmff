use std::io::Write;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U8F8};

use crate::r#box::{track::Track, Decode, Encode, Result};

// 8.1
#[derive(Debug)]
pub struct Movie {
    pub header: MovieHeader,
    pub tracks: Vec<Track>,
}

impl Encode for Movie {
    fn size(&self) -> u64 {
        4 + 4 + self.header.size() + self.tracks.iter().map(Encode::size).sum::<u64>()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"moov").encode(output)?; // type

        self.header.encode(output)?;
        for track in &self.tracks {
            track.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for Movie {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut tracks = vec![];

        while !input.is_empty() {
            let size = u32::decode(input)?;
            let r#type: [u8; 4] = u32::decode(input)?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 4 - 4) as usize);
            match &r#type {
                b"mvhd" => {
                    assert!(header.is_none());
                    header = Some(Decode::decode(&mut data)?)
                }
                b"trak" => tracks.push(Decode::decode(&mut data)?),
                b"mvex" => {}
                b"ipmc" => {}
                _ => {}
            }
            *input = remaining_data;
        }

        assert!(!tracks.is_empty());
        Ok(Self {
            header: header.unwrap(),
            tracks,
        })
    }
}

// 8.2
#[derive(Debug)]
pub struct MediaData {
    pub data: Vec<u8>,
}

impl Encode for MediaData {
    fn size(&self) -> u64 {
        4 + 4 + self.data.len() as u64
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"mdat").encode(output)?; // type

        output.write(&self.data)?;
        Ok(())
    }
}

impl Decode for MediaData {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let data = input.to_owned();
        *input = &input[input.len()..];

        Ok(Self { data })
    }
}

// 8.3
#[derive(Debug)]
pub struct MovieHeader {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub rate: U16F16,
    pub volume: U8F8,
    pub matrix: [u32; 9],
    pub next_track_id: u32,
}

impl Encode for MovieHeader {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + 4 + 4 + 4 + 4 + 2 + 2 + 2 * 4 + 9 * 4 + 6 * 4 + 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"mvhd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.creation_time as u32).encode(output)?;
        (self.modification_time as u32).encode(output)?;
        self.timescale.encode(output)?;
        (self.duration as u32).encode(output)?;
        self.rate.encode(output)?;
        self.volume.encode(output)?;
        0u16.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        for value in self.matrix {
            value.encode(output)?;
        }
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        self.next_track_id.encode(output)
    }
}

impl Decode for MovieHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        input.read_u24::<BigEndian>()?; // flags

        let creation_time;
        let modification_time;
        let timescale;
        let duration;
        match version {
            0 => {
                creation_time = u32::decode(input)? as u64;
                modification_time = u32::decode(input)? as u64;
                timescale = Decode::decode(input)?;
                duration = u32::decode(input)? as u64;
            }
            1 => {
                creation_time = Decode::decode(input)?;
                modification_time = Decode::decode(input)?;
                timescale = Decode::decode(input)?;
                duration = Decode::decode(input)?;
            }
            _ => panic!(),
        }
        let rate = Decode::decode(input)?;
        let volume = Decode::decode(input)?;
        assert_eq!(u16::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        let matrix = [
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
        ];
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
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
