use std::{
    fmt::{Debug, Formatter},
    io::Write,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::types::{U16F16, U8F8};
use thiserror::Error;

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

impl Encode for String {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_all(self.as_bytes())?;
        output.write_u8(0)?;

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

#[derive(Debug)]
pub struct File {
    pub file_type: FileType,
    pub movie: Movie,
}

impl Encode for File {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        self.file_type.encode(output)?;
        self.movie.encode(output)?;
        Ok(())
    }
}

impl Decode for File {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut file_type = None;
        let mut movie = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"ftyp" => {
                    assert!(file_type.is_none());
                    file_type = Some(Decode::decode(&mut data)?)
                }
                b"pdin" => {}
                b"moov" => {
                    assert!(movie.is_none());
                    movie = Some(Decode::decode(&mut data)?)
                }
                b"moof" => {}
                b"mfra" => {}
                b"mdat" => {}
                b"free" => {}
                b"skip" => {}
                b"meta" => {}
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            file_type: file_type.unwrap(),
            movie: movie.unwrap(),        
        })
    }
}

// 4.3
#[derive(Debug)]
pub struct FileType {
    pub major_brand: FourCC,
    pub minor_version: u32,
    pub compatible_brands: Vec<FourCC>,
}

impl Encode for FileType {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"ftyp"))?; // type

        output.write_u32::<BigEndian>(self.major_brand.0)?;
        output.write_u32::<BigEndian>(self.minor_version)?;
        for compatible_brand in &self.compatible_brands {
            output.write_u32::<BigEndian>(compatible_brand.0)?;
        }

        Ok(())
    }
}

impl Decode for FileType {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            major_brand: FourCC(input.read_u32::<BigEndian>()?),
            minor_version: input.read_u32::<BigEndian>()?,
            compatible_brands: input
                .chunks(4)
                .map(|chunk| FourCC(u32::from_be_bytes(chunk.try_into().unwrap())))
                .collect(),
        })
    }
}

// 8.1
#[derive(Debug)]
pub struct Movie {
    pub header: MovieHeader,
    pub tracks: Vec<Track>,
}

impl Encode for Movie {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"moov"))?; // type

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
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mvhd"))?; // type
        output.write_u8(1)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        output.write_u64::<BigEndian>(self.creation_time)?;
        output.write_u64::<BigEndian>(self.modification_time)?;
        output.write_u32::<BigEndian>(self.timescale)?;
        output.write_u64::<BigEndian>(self.duration)?;
        output.write_u32::<BigEndian>(self.rate.to_bits())?;
        output.write_u16::<BigEndian>(self.volume.to_bits())?;
        output.write_u16::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        for value in self.matrix {
            output.write_u32::<BigEndian>(value)?;
        }
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(0)?; // reserved
        output.write_u32::<BigEndian>(self.next_track_id)?;

        Ok(())
    }
}

impl Decode for MovieHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
        let rate = U16F16::from_bits(input.read_u32::<BigEndian>()?);
        let volume = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        assert_eq!(input.read_u16::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
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
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
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

// 8.4
#[derive(Debug)]
pub struct Track {
    pub header: TrackHeader,
    pub media: Media,
}

impl Encode for Track {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"trak"))?; // type

        todo!()
    }
}

impl Decode for Track {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut media = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"tkhd" => {
                    assert!(header.is_none());
                    header = Some(Decode::decode(&mut data)?)
                }
                b"tref" => {}
                b"edts" => {}
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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"tkhd"))?; // type

        todo!()
    }
}

impl Decode for TrackHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
            _ => panic!(),
        }
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        assert_eq!(input.read_u32::<BigEndian>()?, 0); // reserved
        let layer = input.read_u16::<BigEndian>()?;
        let alternate_group = input.read_u16::<BigEndian>()?;
        let volume = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        assert_eq!(input.read_u16::<BigEndian>()?, 0); // reserved
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

// 8.7
#[derive(Debug)]
pub struct Media {
    pub header: MediaHeader,
    pub handler: Handler,
    pub information: MediaInformation,
}

impl Encode for Media {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mdia"))?; // type

        todo!()
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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mdhd"))?; // type

        todo!()
    }
}

impl Decode for MediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"hdlr"))?; // type

        todo!()
    }
}

impl Decode for Handler {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"minf"))?; // type

        todo!()
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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"vmhd"))?; // type

        todo!()
    }
}

impl Decode for VideoMediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

        let graphicsmode = input.read_u16::<BigEndian>()?;
        let opcolor = [
            input.read_u16::<BigEndian>()?,
            input.read_u16::<BigEndian>()?,
            input.read_u16::<BigEndian>()?,
        ];

        Ok(Self {
            graphicsmode,
            opcolor,
        })
    }
}

// 8.11.3
#[derive(Debug)]
pub struct SoundMediaHeader {
    pub balance: U8F8,
}

impl Encode for SoundMediaHeader {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"smhd"))?; // type

        todo!()
    }
}

impl Decode for SoundMediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"dinf"))?; // type

        todo!()
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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"url "))?; // type

        todo!()
    }
}

impl Decode for DataEntryUrl {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"urn "))?; // type

        todo!()
    }
}

impl Decode for DataEntryUrn {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"dref"))?; // type

        todo!()
    }
}

impl Decode for DataReference {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

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

// 8.14
#[derive(Debug)]
pub struct SampleTable {
    pub description: SampleDescription,
    pub time_to_sample: TimeToSample,
    pub sample_to_chunk: SampleToChunk,
    pub chunk_offset: ChunkOffset,
}

impl Encode for SampleTable {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"stbl"))?; // type

        todo!()
    }
}

impl Decode for SampleTable {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut description = None;
        let mut time_to_sample = None;
        let mut sample_to_chunk = None;
        let mut chunk_offset = None;

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"stsd" => {
                    assert!(description.is_none());
                    description = Some(Decode::decode(&mut data)?)
                }
                b"stts" => {
                    assert!(time_to_sample.is_none());
                    time_to_sample = Some(Decode::decode(&mut data)?)
                }
                b"ctts" => {}
                b"stsc" => {
                    assert!(sample_to_chunk.is_none());
                    sample_to_chunk = Some(Decode::decode(&mut data)?)
                }
                b"stsz" => {}
                b"stz2" => {}
                b"stco" => {
                    assert!(chunk_offset.is_none());
                    chunk_offset = Some(Decode::decode(&mut data)?)
                }
                b"co64" => {}
                b"stss" => {}
                b"stsh" => {}
                b"padb" => {}
                b"stdp" => {}
                b"sdtp" => {}
                b"sbgp" => {}
                b"sgpd" => {}
                b"subs" => {}
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {
            description: description.unwrap(),
            time_to_sample: time_to_sample.unwrap(),
            sample_to_chunk: sample_to_chunk.unwrap(),
            chunk_offset: chunk_offset.unwrap(),
        })
    }
}

// 8.15.2
#[derive(Debug)]
pub struct TimeToSample {
    pub entries: Vec<(u32, u32)>,
}

impl Encode for TimeToSample {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"stts"))?; // type

        todo!()
    }
}

impl Decode for TimeToSample {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

        let entry_count = input.read_u32::<BigEndian>()?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let sample_count = input.read_u32::<BigEndian>()?;
            let sample_delta = input.read_u32::<BigEndian>()?;
            entries.push((sample_count, sample_delta))
        }

        Ok(Self { entries })
    }
}

// 8.16
#[derive(Debug)]
pub struct SampleDescription {}

impl Encode for SampleDescription {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"stsd"))?; // type

        todo!()
    }
}

impl Decode for SampleDescription {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

        let entry_count = input.read_u32::<BigEndian>()?;
        for _ in 0..entry_count {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self {})
    }
}

// 8.18
#[derive(Debug)]
pub struct SampleToChunk {
    pub entries: Vec<(u32, u32, u32)>,
}

impl Encode for SampleToChunk {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"stsc"))?; // type

        todo!()
    }
}

impl Decode for SampleToChunk {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

        let entry_count = input.read_u32::<BigEndian>()?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let first_chunk = input.read_u32::<BigEndian>()?;
            let samples_per_chunk = input.read_u32::<BigEndian>()?;
            let sample_description_index = input.read_u32::<BigEndian>()?;
            //entries.push((first_chunk, samples_per_chunk, sample_description_index))
        }

        Ok(Self { entries })
    }
}

// 8.19
#[derive(Debug)]
pub struct ChunkOffset {
    pub entries: Vec<u32>,
}

impl Encode for ChunkOffset {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(0)?; // size
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"stco"))?; // type

        todo!()
    }
}

impl Decode for ChunkOffset {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let _version = input.read_u8()?;
        let _flags = input.read_u24::<BigEndian>()?;

        let entry_count = input.read_u32::<BigEndian>()?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let chunk_offset = input.read_u32::<BigEndian>()?;
            //entries.push(chunk_offset)
        }

        Ok(Self { entries })
    }
}
