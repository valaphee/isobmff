use std::{
    fmt::{Debug, Formatter},
    hash::Hasher,
    io::{Read, Write},
};

use bstringify::bstringify;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use derivative::Derivative;
use fixed::types::{U16F16, U2F30, U8F8};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Invalid {r#type} box quantity, current {quantity}, expected {expected}")]
    BoxQuantity {
        r#type: &'static str,
        quantity: usize,
        expected: usize,
    },
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

impl Encode for U2F30 {
    fn size(&self) -> u64 {
        4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
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

impl Encode for Matrix {
    fn size(&self) -> u64 {
        9 * 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
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

macro_rules! decode_boxes {(
    $input:ident,
    $(
        $quantifier:ident $type:ident $name:ident
    ),* $(,)?
) => (
     while !$input.is_empty() {
        let size = u32::decode($input)?;
        let r#type: [u8; 4] = u32::decode($input)?.to_be_bytes();

        let (mut data, remaining_data) = $input.split_at((size - 4 - 4) as usize);
        match &r#type {
            $(
                bstringify!($type) => decode_box!(data $quantifier $type $name),
            )*
                _ => {}
        }
        *$input = remaining_data;
    }

    $(unwrap_box!($quantifier $type $name);)*
)}

macro_rules! decode_box {
    ($input:ident optional $type:ident $name:ident) => {{
        if $name.is_some() {
            return Err(Error::BoxQuantity {
                r#type: stringify!($type),
                quantity: 2,
                expected: 1,
            });
        }
        $name = Some(Decode::decode(&mut $input)?);
    }};

    ($input:ident required $type:ident $name:ident) => {{
        if $name.is_some() {
            return Err(Error::BoxQuantity {
                r#type: stringify!($type),
                quantity: 2,
                expected: 1,
            });
        }
        $name = Some(Decode::decode(&mut $input)?);
    }};

    ($input:ident multiple $type:ident $name:ident) => {
        $name.push(Decode::decode(&mut $input)?)
    };
}

macro_rules! unwrap_box {
    (optional $type:ident $name:ident) => {};

    (required $type:ident $name:ident) => {
        let $name = $name.ok_or(Error::BoxQuantity {
            r#type: stringify!($type),
            quantity: 0,
            expected: 1,
        })?;
    };

    (multiple $type:ident $name:ident) => {};
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub struct File {
    pub file_type: FileType,
    pub movie: Movie,
    #[derivative(Debug = "ignore")]
    pub media_data: Vec<MediaData>,
}

impl Encode for File {
    fn size(&self) -> u64 {
        self.file_type.size()
            + 4
            + 4
            + self.media_data.iter().map(Encode::size).sum::<u64>()
            + self.movie.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        self.file_type.encode(output)?;
        8u32.encode(output)?; // size
        u32::from_be_bytes(*b"free").encode(output)?; // type
        for media_data in &self.media_data {
            media_data.encode(output)?;
        }
        self.movie.encode(output)?;
        Ok(())
    }
}

impl Decode for File {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut file_type = None;
        let mut movie = None;
        let mut media_data = vec![];

        decode_boxes! {
            input,
            required ftyp file_type,
            required moov movie,
            multiple mdat media_data,
        }

        Ok(Self {
            file_type,
            movie,
            media_data,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 4.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct FileType {
    pub major_brand: FourCC,
    pub minor_version: u32,
    pub compatible_brands: Vec<FourCC>,
}

impl Encode for FileType {
    fn size(&self) -> u64 {
        4 + 4 + 4 + 4 + self.compatible_brands.len() as u64 * 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"ftyp").encode(output)?; // type

        self.major_brand.0.encode(output)?;
        self.minor_version.encode(output)?;
        for compatible_brand in &self.compatible_brands {
            compatible_brand.0.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for FileType {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let major_brand = FourCC(Decode::decode(input)?);
        let minor_version = Decode::decode(input)?;
        let compatible_brands = input
            .chunks(4)
            .map(|chunk| FourCC(u32::from_be_bytes(chunk.try_into().unwrap())))
            .collect();
        *input = &input[input.len()..];

        Ok(Self {
            major_brand,
            minor_version,
            compatible_brands,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.1
////////////////////////////////////////////////////////////////////////////////////////////////////

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

        decode_boxes! {
            input,
            required mvhd header,
            multiple trak tracks,
        }

        assert!(!tracks.is_empty());
        Ok(Self { header, tracks })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.2
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MovieHeader {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub rate: U16F16,
    pub volume: U8F8,
    pub matrix: Matrix,
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
        self.matrix.encode(output)?;
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
        let matrix = Decode::decode(input)?;
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.4
////////////////////////////////////////////////////////////////////////////////////////////////////

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

        decode_boxes! {
            input,
            required tkhd header,
            optional edts edit,
            required mdia media,
        }

        Ok(Self {
            header,
            edit,
            media,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.5
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct TrackHeader {
    pub creation_time: u64,
    pub modification_time: u64,
    pub track_id: u32,
    pub duration: u64,
    pub layer: u16,
    pub alternate_group: u16,
    pub volume: U8F8,
    pub matrix: Matrix,
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
        self.matrix.encode(output)?;
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
        let matrix = Decode::decode(input)?;
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.7
////////////////////////////////////////////////////////////////////////////////////////////////////

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

        decode_boxes! {
            input,
            required mdhd header,
            required hdlr handler,
            required minf information,
        }

        Ok(Self {
            header,
            handler,
            information,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.8
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.9
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.10
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MediaInformation {
    pub header: MediaInformationHeader,
    pub data_information: DataInformation,
    pub sample_table: SampleTable,
}

impl Encode for MediaInformation {
    fn size(&self) -> u64 {
        4 + 4
            + match &self.header {
                MediaInformationHeader::Video(header) => header.size(),
                MediaInformationHeader::Sound(header) => header.size(),
            }
            + self.data_information.size()
            + self.sample_table.size()
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"minf").encode(output)?; // type

        match &self.header {
            MediaInformationHeader::Video(header) => header.encode(output),
            MediaInformationHeader::Sound(header) => header.encode(output),
        }?;
        self.data_information.encode(output)?;
        self.sample_table.encode(output)
    }
}

impl Decode for MediaInformation {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut video_header = None;
        let mut sound_header = None;
        let mut data_information = None;
        let mut sample_table = None;

        decode_boxes! {
            input,
            optional vmhd video_header,
            optional smhd sound_header,
            required dinf data_information,
            required stbl sample_table,
        }

        Ok(Self {
            header: if let Some(video_header) = video_header {
                MediaInformationHeader::Video(video_header)
            } else if let Some(sound_header) = sound_header {
                MediaInformationHeader::Sound(sound_header)
            } else {
                todo!()
            },
            data_information,
            sample_table,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.11
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum MediaInformationHeader {
    Video(VideoMediaHeader),
    Sound(SoundMediaHeader),
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.11.2
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.11.3
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.12
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.13
////////////////////////////////////////////////////////////////////////////////////////////////////

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
        4 + 4
            + 1
            + 3
            + 4
            + self
                .entries
                .iter()
                .map(|entry| match entry {
                    DataEntry::Url(entry) => entry.size(),
                    DataEntry::Urn(entry) => entry.size(),
                })
                .sum::<u64>()
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.14
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SampleTable {
    pub description: SampleDescription,
    pub time_to_sample: TimeToSample,
    #[derivative(Debug = "ignore")]
    pub sample_to_chunk: SampleToChunk,
    #[derivative(Debug = "ignore")]
    pub sample_size: SampleSize,
    #[derivative(Debug = "ignore")]
    pub chunk_offset: ChunkOffset,
    pub sync_sample: Option<SyncSample>,
    pub sample_to_group: Option<SampleToGroup>,
}

impl Encode for SampleTable {
    fn size(&self) -> u64 {
        4 + 4
            + self.description.size()
            + self.time_to_sample.size()
            + self.sample_to_chunk.size()
            + self.sample_size.size()
            + self.chunk_offset.size()
            + self.sync_sample.as_ref().map_or(0, Encode::size)
            + self.sample_to_group.as_ref().map_or(0, Encode::size)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stbl").encode(output)?; // type

        self.description.encode(output)?;
        self.time_to_sample.encode(output)?;
        self.sample_to_chunk.encode(output)?;
        self.sample_size.encode(output)?;
        self.chunk_offset.encode(output)?;
        if let Some(sync_sample) = &self.sync_sample {
            sync_sample.encode(output)?;
        }
        if let Some(sample_to_group) = &self.sample_to_group {
            sample_to_group.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for SampleTable {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut description = None;
        let mut time_to_sample = None;
        let mut sample_to_chunk = None;
        let mut sample_size = None;
        let mut chunk_offset = None;
        let mut sync_sample = None;
        let mut sample_to_group = None;

        decode_boxes! {
            input,
            required stsd description,
            required stts time_to_sample,
            required stsc sample_to_chunk,
            required stsz sample_size,
            required stco chunk_offset,
            optional stss sync_sample,
            optional sbgp sample_to_group,
        }

        Ok(Self {
            description,
            time_to_sample,
            sample_to_chunk,
            sample_size,
            chunk_offset,
            sync_sample,
            sample_to_group,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.15.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct TimeToSample {
    pub entries: Vec<(u32, u32)>,
}

impl Encode for TimeToSample {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.entries.len() as u64 * (4 + 4)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stts").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.entries.len() as u32).encode(output)?;
        for entry in &self.entries {
            entry.0.encode(output)?;
            entry.1.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for TimeToSample {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.16
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct VisualSampleEntry {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: U16F16,
    pub vertresolution: U16F16,
    pub frame_count: u16,
    pub compressorname: [u8; 32],
    pub depth: u16,
    pub extra: Vec<u8>,
}

impl Encode for VisualSampleEntry {
    fn size(&self) -> u64 {
        4 + 4
            + 6 * 1
            + 2
            + 2
            + 2
            + 3 * 4
            + 2
            + 2
            + 4
            + 4
            + 4
            + 2
            + 32
            + 2
            + 2
            + self.extra.len() as u64
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"avc1").encode(output)?; // type

        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        self.data_reference_index.encode(output)?;

        0u16.encode(output)?; // pre_defined
        0u16.encode(output)?; // reserved
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        self.width.encode(output)?;
        self.height.encode(output)?;
        self.horizresolution.encode(output)?;
        self.vertresolution.encode(output)?;
        0u32.encode(output)?;
        self.frame_count.encode(output)?;
        output.write_all(&self.compressorname)?;
        self.depth.encode(output)?;
        u16::MAX.encode(output)?;
        output.write_all(&self.extra)?;
        Ok(())
    }
}

impl Decode for VisualSampleEntry {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        let data_reference_index = Decode::decode(input)?;

        assert_eq!(u16::decode(input)?, 0); // pre_defined
        assert_eq!(u16::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // pre_defined
        assert_eq!(u32::decode(input)?, 0); // pre_defined
        assert_eq!(u32::decode(input)?, 0); // pre_defined
        let width = Decode::decode(input)?;
        let height = Decode::decode(input)?;
        let horizresolution = Decode::decode(input)?;
        let vertresolution = Decode::decode(input)?;
        assert_eq!(u32::decode(input)?, 0); // reserved
        let frame_count = Decode::decode(input)?;
        let mut compressorname = [0u8; 32];
        input.read_exact(&mut compressorname)?;
        let depth = Decode::decode(input)?;
        assert_eq!(u16::decode(input)?, u16::MAX); // pre_defined
        let extra = input.to_owned();

        Ok(Self {
            data_reference_index,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            compressorname,
            depth,
            extra,
        })
    }
}

#[derive(Debug)]
pub struct AudioSampleEntry {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: U16F16,
    pub extra: Vec<u8>,
}

impl Encode for AudioSampleEntry {
    fn size(&self) -> u64 {
        4 + 4 + 6 * 1 + 2 + 2 * 4 + 2 + 2 + 2 + 2 + 4 + self.extra.len() as u64
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"mp4a").encode(output)?; // type

        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        output.write_u8(0)?; // reserved
        self.data_reference_index.encode(output)?;

        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        self.channelcount.encode(output)?;
        self.samplesize.encode(output)?;
        0u16.encode(output)?; // pre_defined
        0u16.encode(output)?; // reserved
        self.samplerate.encode(output)?;
        output.write_all(&self.extra)?;
        Ok(())
    }
}

impl Decode for AudioSampleEntry {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        assert_eq!(input.read_u8()?, 0); // reserved
        let data_reference_index = Decode::decode(input)?;

        assert_eq!(u32::decode(input)?, 0); // reserved
        assert_eq!(u32::decode(input)?, 0); // reserved
        let channelcount = Decode::decode(input)?;
        let samplesize = Decode::decode(input)?;
        assert_eq!(u16::decode(input)?, 0); // pre_defined
        assert_eq!(u16::decode(input)?, 0); // reserved
        let samplerate = Decode::decode(input)?;
        let extra = input.to_owned();

        Ok(Self {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
            extra,
        })
    }
}

#[derive(Debug)]
pub struct SampleDescription {
    pub avc1: Option<VisualSampleEntry>,
    pub mp4a: Option<AudioSampleEntry>,
}

impl Encode for SampleDescription {
    fn size(&self) -> u64 {
        4 + 4
            + 1
            + 3
            + 4
            + self.avc1.as_ref().map_or(0, Encode::size)
            + self.mp4a.as_ref().map_or(0, Encode::size)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stsd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        1u32.encode(output)?; // entry_count
        if let Some(avc1) = &self.avc1 {
            avc1.encode(output)?;
        }
        if let Some(mp4a) = &self.mp4a {
            mp4a.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for SampleDescription {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let mut avc1 = None;
        let mut mp4a = None;

        let entry_count = input.read_u32::<BigEndian>()?;
        for _ in 0..entry_count {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"avc1" => avc1 = Some(Decode::decode(&mut data)?),
                b"mp4a" => mp4a = Some(Decode::decode(&mut data)?),
                _ => {}
            }
            *input = remaining_data;
        }

        Ok(Self { avc1, mp4a })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.17
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum SampleSize {
    Global(u32),
    Unique(Vec<u32>),
}

impl Encode for SampleSize {
    fn size(&self) -> u64 {
        4 + 4
            + 1
            + 3
            + 4
            + 4
            + match self {
                SampleSize::Global(_) => 0,
                SampleSize::Unique(samples) => samples.len() as u64 * 4,
            }
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stsz").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        match self {
            SampleSize::Global(sample_size) => {
                sample_size.encode(output)?;
                0u32.encode(output)?; // sample_count
            }
            SampleSize::Unique(samples) => {
                0u32.encode(output)?; // sample_size
                (samples.len() as u32).encode(output)?;
                for sample in samples {
                    sample.encode(output)?;
                }
            }
        }
        Ok(())
    }
}

impl Decode for SampleSize {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let sample_size = input.read_u32::<BigEndian>()?;
        let sample_count = input.read_u32::<BigEndian>()?;
        if sample_size != 0 {
            return Ok(SampleSize::Global(sample_size));
        }

        let mut samples = Vec::default();
        for _ in 0..sample_count {
            let entry_size = input.read_u32::<BigEndian>()?;
            samples.push(entry_size)
        }

        Ok(SampleSize::Unique(samples))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.18
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SampleToChunk {
    pub entries: Vec<(u32, u32, u32)>,
}

impl Encode for SampleToChunk {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.entries.len() as u64 * (4 + 4 + 4)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stsc").encode(output)?; // type
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

impl Decode for SampleToChunk {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let first_chunk = u32::decode(input)?;
            let samples_per_chunk = u32::decode(input)?;
            let sample_description_index = u32::decode(input)?;
            entries.push((first_chunk, samples_per_chunk, sample_description_index))
        }

        Ok(Self { entries })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.19
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct ChunkOffset {
    pub entries: Vec<u32>,
}

impl Encode for ChunkOffset {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.entries.len() as u64 * 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stco").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.entries.len() as u32).encode(output)?;
        for entry in &self.entries {
            entry.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for ChunkOffset {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let chunk_offset = u32::decode(input)?;
            entries.push(chunk_offset)
        }

        Ok(Self { entries })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.20
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SyncSample {
    pub entries: Vec<u32>,
}

impl Encode for SyncSample {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.entries.len() as u64 * 4
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stss").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.entries.len() as u32).encode(output)?;
        for entry in &self.entries {
            entry.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for SyncSample {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let sample_number = Decode::decode(input)?;
            entries.push(sample_number)
        }

        Ok(Self { entries })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.25
////////////////////////////////////////////////////////////////////////////////////////////////////

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
                b"elst" => edit_list = Some(Decode::decode(&mut data)?),
                _ => {}
            }
            *input = remaining_data;
        }
        Ok(Self { edit_list })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.26
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2005 8.40.3.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SampleToGroup {
    pub grouping_type: FourCC,
    pub entries: Vec<(u32, u32)>,
}

impl Encode for SampleToGroup {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + 4 + self.entries.len() as u64 * (4 + 4)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"sbgp").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.grouping_type.0.encode(output)?;
        (self.entries.len() as u32).encode(output)?;
        for entry in &self.entries {
            entry.0.encode(output)?;
            entry.1.encode(output)?;
        }
        Ok(())
    }
}

impl Decode for SampleToGroup {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let grouping_type = FourCC(Decode::decode(input)?);
        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let sample_count = Decode::decode(input)?;
            let group_description_index = Decode::decode(input)?;
            entries.push((sample_count, group_description_index))
        }

        Ok(Self {
            grouping_type,
            entries,
        })
    }
}
