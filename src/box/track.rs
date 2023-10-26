use std::io::Write;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U8F8};

use crate::r#box::{media::Media, Decode, Encode, Result};

// 8.4
#[derive(Debug)]
pub struct Track {
    pub header: TrackHeader,
    pub edit: Option<Edit>,
    pub media: Media,
}

impl Encode for Track {
    fn size(&self) -> u64 {
        4 + 4 + self.header.size() + self.edit.as_ref().map_or(0, Encode::size) + self.media.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"trak").encode(output)?; // type

        self.header.encode(output)?;
        if let Some(edit) = &self.edit {
            edit.encode(output)?;
        }
        self.media.encode(output)
    }
}

impl Decode for Track {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut edit = None;
        let mut media = None;

        while !input.is_empty() {
            let size = u32::decode(input)?;
            let r#type: [u8; 4] = u32::decode(input)?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 4 - 4) as usize);
            match &r#type {
                b"tkhd" => {
                    assert!(header.is_none());
                    header = Some(Decode::decode(&mut data)?)
                }
                b"tref" => {}
                b"edts" => {
                    assert!(edit.is_none());
                    edit = Some(Decode::decode(&mut data)?)
                }
                b"mdia" => {
                    assert!(media.is_none());
                    media = Some(Decode::decode(&mut data)?)
                }
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            header: header.unwrap(),
            edit,
            media: media.unwrap(),
        })
    }
}

// 8.5
#[derive(Debug)]
pub struct TrackHeader {
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

impl Encode for TrackHeader {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 2 + 2 + 2 + 2 + 9 * 4 + 4 + 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"tkhd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.creation_time as u32).encode(output)?;
        (self.modification_time as u32).encode(output)?;
        self.track_id.encode(output)?;
        0u32.encode(output)?; // reserved
        (self.duration as u32).encode(output)?;
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        self.layer.encode(output)?;
        self.alternate_group.encode(output)?;
        self.volume.encode(output)?;
        0u16.encode(output)?; // reserved
        for value in self.matrix {
            value.encode(output)?;
        }
        self.width.encode(output)?;
        self.height.encode(output)
    }
}

impl Decode for TrackHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        input.read_u24::<BigEndian>()?; // flags

        let creation_time;
        let modification_time;
        let track_id;
        let duration;
        match version {
            0 => {
                creation_time = u32::decode(input)? as u64;
                modification_time = u32::decode(input)? as u64;
                track_id = Decode::decode(input)?;
                assert_eq!(u32::decode(input)?, 0); // reserved
                duration = u32::decode(input)? as u64;
            }
            1 => {
                creation_time = Decode::decode(input)?;
                modification_time = Decode::decode(input)?;
                track_id = Decode::decode(input)?;
                assert_eq!(u32::decode(input)?, 0); // reserved
                duration = Decode::decode(input)?;
            }
            _ => panic!(),
        }
        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        let layer = Decode::decode(input)?;
        let alternate_group = Decode::decode(input)?;
        let volume = Decode::decode(input)?;
        assert_eq!(u16::decode(input)?, 0); // reserved
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
        let width = Decode::decode(input)?;
        let height = Decode::decode(input)?;

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

// 8.26
#[derive(Debug)]
pub struct Edit {
    edit_list: Option<EditList>,
}

impl Encode for Edit {
    fn size(&self) -> u64 {
        4 + 4 + self.edit_list.as_ref().map_or(0, Encode::size)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"edts").encode(output)?; // type

        if let Some(edit_list) = &self.edit_list {
            edit_list.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for Edit {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut edit_list = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"elst" => edit_list = Some(Decode::decode(input)?),
                _ => {}
            }
            *input = remaining_data;
        }
        Ok(Self { edit_list })
    }
}

// 8.40.3.2
#[derive(Debug)]
pub struct EditList {
    pub entries: Vec<(u32, u32, U16F16)>,
}

impl Encode for EditList {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.entries.len() as u64 * (4 + 4 + 4)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"elst").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.entries.len() as u32).encode(output)?;
        for entry in &self.entries {
            entry.0.encode(output)?;
            entry.1.encode(output)?;
            entry.2.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for EditList {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let segment_duration = Decode::decode(input)?;
            let media_time = Decode::decode(input)?;
            let media_rate = Decode::decode(input)?;

            entries.push((segment_duration, media_time, media_rate))
        }

        Ok(Self { entries })
    }
}
