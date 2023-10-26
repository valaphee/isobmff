use std::io::Write;
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
    fn size(&self) -> u64;

    fn encode(&self, output: &mut impl Write) -> Result<()>;
}

pub trait Decode: Sized {
    fn decode(input: &mut &[u8]) -> Result<Self>;
}

impl Encode for String {
    fn size(&self) -> u64 {
        self.as_bytes().len() as u64 + 1
    }

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

#[derive(Debug)]
pub struct File {
    pub file_type: FileType,
    pub movie: Movie,
}

impl Encode for File {
    fn size(&self) -> u64 {
        self.file_type.size() + self.movie.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        self.file_type.encode(output)?;
        self.movie.encode(output)?;
        Ok(())
    }
}

impl Decode for File {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut file_type = Default::default();
        let mut movie = Default::default();
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"ftyp" => file_type = Some(Decode::decode(&mut data)?),
                b"moov" => movie = Some(Decode::decode(&mut data)?),
                _ => {
                    println!("File {}", std::str::from_utf8(&r#type).unwrap());
                }
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
    pub major_brand: u32,
    pub minor_version: u32,
    pub compatible_brands: Vec<u32>,
}

impl Encode for FileType {
    fn size(&self) -> u64 {
        (4 + 4 + 4 + 4 + self.compatible_brands.len() as u32 * 4) as u64
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(self.size() as u32)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"ftyp"))?;

        output.write_u32::<BigEndian>(self.major_brand)?;
        output.write_u32::<BigEndian>(self.minor_version)?;
        for compatible_brand in &self.compatible_brands {
            output.write_u32::<BigEndian>(*compatible_brand)?;
        }

        Ok(())
    }
}

impl Decode for FileType {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            major_brand: input.read_u32::<BigEndian>()?,
            minor_version: input.read_u32::<BigEndian>()?,
            compatible_brands: input.chunks(4).map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap())).collect(),
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
    fn size(&self) -> u64 {
        self.header.size() + self.tracks.iter().map(Track::size).sum()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(self.size() as u32)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"moov"))?;

        self.header.encode(output)?;
        for track in self.tracks {
            track.encode(output)?;
        }

        Ok(())
    }
}

impl Decode for Movie {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = Default::default();
        let mut tracks = Vec::default();
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"mvhd" => header = Some(Decode::decode(&mut data)?),
                b"trak" => tracks.push(Decode::decode(&mut data)?),
                _ => {
                    println!("File > Movie {}", std::str::from_utf8(&r#type).unwrap());
                }
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
        4 + 4 + 1 + 3 + 8 + 8 + 4 + 8 + 4 + 2 + 2 + 2 * 4 + 9 * 4 + 6 * 4 + 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(self.size() as u32)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mvhd"))?;
        output.write_u8(1)?;
        output.write_u24::<BigEndian>(0)?;

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

// 8.4
#[derive(Debug)]
pub struct Track {
    pub header: TrackHeader,
    pub media: Media,
}

impl Encode for Track {
    fn size(&self) -> u64 {
        todo!()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"trak"))?;

        Ok(())
    }
}

impl Decode for Track {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = Default::default();
        let mut media = Default::default();
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"tkhd" => header = Some(Decode::decode(&mut data)?),
                b"mdia" => media = Some(Decode::decode(&mut data)?),
                _ => {
                    println!("File > Movie > Track {}", std::str::from_utf8(&r#type).unwrap());
                }
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
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"tkhd"))?;

        Ok(())
    }
}

impl Decode for TrackHeader {
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

// 8.7
#[derive(Debug)]
pub struct Media {
    header: MediaHeader,
    handler: Handler,
    information: MediaInformation,
}

impl Encode for Media {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mdia"))?;

        Ok(())
    }
}

impl Decode for Media {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = Default::default();
        let mut handler = Default::default();
        let mut information = Default::default();

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"mdhd" => header = Some(Decode::decode(&mut data)?),
                b"hdlr" => handler = Some(Decode::decode(&mut data)?),
                b"minf" => information = Some(Decode::decode(&mut data)?),
                _ => {
                    println!("File > Movie > Track > Media {}", std::str::from_utf8(&r#type).unwrap());
                }
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
    pub language: u16,
}

impl Encode for MediaHeader {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"mdhd"))?;

        Ok(())
    }
}

impl Decode for MediaHeader {
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

// 8.9
#[derive(Debug)]
pub struct Handler {
    pub r#type: u32,
}

impl Encode for Handler {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"hdlr"))?;

        Ok(())
    }
}

impl Decode for Handler {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        input.read_u32::<BigEndian>()?;
        let r#type = input.read_u32::<BigEndian>()?;
        input.read_u32::<BigEndian>()?;
        input.read_u32::<BigEndian>()?;
        input.read_u32::<BigEndian>()?;

        Ok(Self {
            r#type,
        })
    }
}

// 8.10
#[derive(Debug)]
pub struct MediaInformation {
    video_header: Option<VideoMediaHeader>,
    sound_header: Option<SoundMediaHeader>,
    data_information: DataInformation,
    sample_table: SampleTable,
}

impl Encode for MediaInformation {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"minf"))?;

        Ok(())
    }
}

impl Decode for MediaInformation {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut video_header = Default::default();
        let mut sound_header = Default::default();
        let mut data_information = Default::default();
        let mut sample_table = Default::default();

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"vmhd" => video_header = Some(Decode::decode(&mut data)?),
                b"smhd" => sound_header = Some(Decode::decode(&mut data)?),
                b"dinf" => data_information = Some(Decode::decode(&mut data)?),
                b"stbl" => sample_table = Some(Decode::decode(&mut data)?),
                _ => {
                    println!("File > Movie > Track > Media > MediaInformation {}", std::str::from_utf8(&r#type).unwrap());
                }
            }
            *input = remaining_data;
        }

        Ok(Self {
            video_header,
            sound_header,
            data_information: data_information.unwrap(),
            sample_table: sample_table.unwrap(),
        })
    }
}

// 8.11.2
#[derive(Debug)]
pub struct VideoMediaHeader {
    pub graphicsmode: u16,
    pub opcolor: [u16; 3],
}

impl Encode for VideoMediaHeader {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"vmhd"))?;

        Ok(())
    }
}

impl Decode for VideoMediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

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
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"smhd"))?;

        Ok(())
    }
}

impl Decode for SoundMediaHeader {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        let balance = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        input.read_u16::<BigEndian>()?; // reserved

        Ok(Self {
            balance,
        })
    }
}

// 8.12
#[derive(Debug)]
pub struct DataInformation {
    reference: DataReference
}

impl Encode for DataInformation {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"dinf"))?;

        Ok(())
    }
}

impl Decode for DataInformation {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut reference = Default::default();

        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"dref" => reference = Some(Decode::decode(input)?),
                _ => {
                    println!("File > Movie > Track > Media > MediaInformation > DataInformation {}", std::str::from_utf8(&r#type).unwrap());
                }
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
    Urn(DataEntryUrn)
}

#[derive(Debug)]
pub struct DataEntryUrl {
    pub location: String,
}

impl Encode for DataEntryUrl {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"url "))?;

        Ok(())
    }
}

impl Decode for DataEntryUrl {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        let location = Decode::decode(input)?;

        Ok(Self {
            location,
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
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"urn "))?;

        Ok(())
    }
}

impl Decode for DataEntryUrn {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        let name = Decode::decode(input)?;
        let location = Decode::decode(input)?;

        Ok(Self {
            name,
            location,
        })
    }
}

#[derive(Debug)]
pub struct DataReference {
    pub entries: Vec<DataEntry>
}

impl Encode for DataReference {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"dref"))?;

        Ok(())
    }
}

impl Decode for DataReference {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        let flags = input.read_u24::<BigEndian>()?;

        let entry_count = input.read_u32::<BigEndian>()?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"url " => entries.push(DataEntry::Url(Decode::decode(&mut data)?)),
                b"urn " => entries.push(DataEntry::Urn(Decode::decode(&mut data)?)),
                _ => todo!()
            }
            *input = remaining_data;
        }

        Ok(Self {
            entries,
        })
    }
}

// 8.14
#[derive(Debug)]
pub struct SampleTable {
}

impl Encode for SampleTable {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"stbl"))?;

        Ok(())
    }
}

impl Decode for SampleTable {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                _ => {
                    println!("File > Movie > Track > Media > MediaInformation > SampleTable {}", std::str::from_utf8(&r#type).unwrap());
                }
            }
            *input = remaining_data;
        }

        Ok(Self {
        })
    }
}