use std::io::Write;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::U8F8;
use crate::r#box::{Decode, Encode, FourCC, Language, Result};
use crate::r#box::sample_table::SampleTable;

// 8.7
#[derive(Debug)]
pub struct Media {
    pub header: MediaHeader,
    pub handler: Handler,
    pub information: MediaInformation,
}

impl Encode for Media {
    fn size(&self) -> u64 {
        4 + 4 + self.header.size() + self.handler.size() + self.information.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"mdia").encode(output)?; // type

        self.header.encode(output)?;
        self.handler.encode(output)?;
        self.information.encode(output)
    }
}

impl Decode for Media {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut handler = None;
        let mut information = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"mdhd" => {
                    assert!(header.is_none());
                    header = Some(Decode::decode(&mut data)?)
                }
                b"hdlr" => {
                    assert!(handler.is_none());
                    handler = Some(Decode::decode(&mut data)?)
                }
                b"minf" => {
                    assert!(information.is_none());
                    information = Some(Decode::decode(&mut data)?)
                }
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            header: header.unwrap(),
            handler: handler.unwrap(),
            information: information.unwrap(),
        })
    }
}

// 8.8
#[derive(Debug)]
pub struct MediaHeader {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub language: Language,
}

impl Encode for MediaHeader {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + 4 + 4 + 4 + 2 + 2
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"mdhd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.creation_time as u32).encode(output)?;
        (self.modification_time as u32).encode(output)?;
        self.timescale.encode(output)?;
        (self.duration as u32).encode(output)?;
        self.language.0.encode(output)?;
        0u16.encode(output) // pre_defined
    }
}

impl Decode for MediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        input.read_u24::<BigEndian>()?; // flags

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
            _ => panic!(),
        }
        let language = Language(input.read_u16::<BigEndian>()?);
        assert_eq!(input.read_u16::<BigEndian>()?, 0); // pre_defined

        Ok(Self {
            creation_time,
            modification_time,
            timescale,
            duration,
            language,
        })
    }
}

// 8.9
#[derive(Debug)]
pub struct Handler {
    pub r#type: FourCC,
    pub name: String,
}

impl Encode for Handler {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + 4 + 4 + 4 + 4 + self.name.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"hdlr").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        0u32.encode(output)?; // pre_defined
        self.r#type.0.encode(output)?;
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        self.name.encode(output)
    }
}

impl Decode for Handler {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        assert_eq!(input.read_u32::<BigEndian>()?, 0); // pre_defined
        let r#type = FourCC(input.read_u32::<BigEndian>()?);
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        let name = Decode::decode(input)?;

        Ok(Self { r#type, name })
    }
}

// 8.10
#[derive(Debug)]
pub struct MediaInformation {
    pub header: MediaInformationHeader,
    pub data_information: DataInformation,
    pub sample_table: SampleTable,
}

impl Encode for MediaInformation {
    fn size(&self) -> u64 {
        4 + 4 + match &self.header {
            MediaInformationHeader::Video(header) => header.size(),
            MediaInformationHeader::Sound(header) => header.size(),
        } + self.data_information.size() + self.sample_table.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"minf").encode(output)?; // type

        match &self.header {
            MediaInformationHeader::Video(header) => header.encode(output),
            MediaInformationHeader::Sound(header) => header.encode(output)
        }?;
        self.data_information.encode(output)?;
        self.sample_table.encode(output)
    }
}

impl Decode for MediaInformation {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut data_information = None;
        let mut sample_table = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"vmhd" => {
                    assert!(header.is_none());
                    header = Some(MediaInformationHeader::Video(Decode::decode(&mut data)?))
                }
                b"smhd" => {
                    assert!(header.is_none());
                    header = Some(MediaInformationHeader::Sound(Decode::decode(&mut data)?))
                }
                b"hmhd" => {
                    assert!(header.is_none());
                }
                b"nmhd" => {
                    assert!(header.is_none());
                }
                b"dinf" => {
                    assert!(data_information.is_none());
                    data_information = Some(Decode::decode(&mut data)?)
                }
                b"stbl" => {
                    assert!(sample_table.is_none());
                    sample_table = Some(Decode::decode(&mut data)?)
                }
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            header: header.unwrap(),
            data_information: data_information.unwrap(),
            sample_table: sample_table.unwrap(),
        })
    }
}

// 8.11
#[derive(Debug)]
pub enum MediaInformationHeader {
    Video(VideoMediaHeader),
    Sound(SoundMediaHeader),
}

// 8.11.2
#[derive(Debug)]
pub struct VideoMediaHeader {
    pub graphicsmode: u16,
    pub opcolor: [u16; 3],
}

impl Encode for VideoMediaHeader {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 2 + 3 * 2
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"vmhd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.graphicsmode.encode(output)?;
        for value in self.opcolor {
            value.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for VideoMediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        Ok(Self {
            graphicsmode: Decode::decode(input)?,
            opcolor: [
                Decode::decode(input)?,
                Decode::decode(input)?,
                Decode::decode(input)?,
            ],
        })
    }
}

// 8.11.3
#[derive(Debug)]
pub struct SoundMediaHeader {
    pub balance: U8F8,
}

impl Encode for SoundMediaHeader {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 2 + 2
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"smhd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.balance.encode(output)?;
        0u16.encode(output) // reserved
    }
}

impl Decode for SoundMediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let balance = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        assert_eq!(input.read_u16::<BigEndian>()?, 0); // reserved

        Ok(Self { balance })
    }
}

// 8.12
#[derive(Debug)]
pub struct DataInformation {
    pub reference: DataReference,
}

impl Encode for DataInformation {
    fn size(&self) -> u64 {
        4 + 4 + self.reference.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"dinf").encode(output)?; // type

        self.reference.encode(output)
    }
}

impl Decode for DataInformation {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut reference = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"dref" => {
                    assert!(reference.is_none());
                    reference = Some(Decode::decode(&mut data)?)
                }
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            reference: reference.unwrap(),
        })
    }
}

// 8.13
#[derive(Debug)]
pub enum DataEntry {
    Url(DataEntryUrl),
    Urn(DataEntryUrn),
}

#[derive(Debug)]
pub struct DataEntryUrl {
    pub location: String,
}

impl Encode for DataEntryUrl {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + self.location.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"url ").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.location.encode(output)
    }
}

impl Decode for DataEntryUrl {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        Ok(Self {
            location: Decode::decode(input)?,
        })
    }
}

#[derive(Debug)]
pub struct DataEntryUrn {
    pub name: String,
    pub location: String,
}

impl Encode for DataEntryUrn {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + self.name.size() + self.location.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"urn ").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.name.encode(output)?;
        self.location.encode(output)
    }
}

impl Decode for DataEntryUrn {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        Ok(Self {
            name: Decode::decode(input)?,
            location: Decode::decode(input)?,
        })
    }
}

#[derive(Debug)]
pub struct DataReference {
    pub entries: Vec<DataEntry>,
}

impl Encode for DataReference {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.entries.iter().map(|entry| match entry {
            DataEntry::Url(entry) => entry.size(),
            DataEntry::Urn(entry) => entry.size(),
        }).sum::<u64>()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"dref").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.entries.len() as u32).encode(output)?;
        for entry in &self.entries {
            match entry {
                DataEntry::Url(entry) => entry.encode(output),
                DataEntry::Urn(entry) => entry.encode(output),
            }?;
        }
        Ok(())
    }
}

impl Decode for DataReference {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = input.read_u32::<BigEndian>()?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"url " => entries.push(DataEntry::Url(Decode::decode(&mut data)?)),
                b"urn " => entries.push(DataEntry::Urn(Decode::decode(&mut data)?)),
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self { entries })
    }
}
