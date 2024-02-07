use std::{
    fmt::{Debug, Formatter},
    io::{Read, Seek, SeekFrom, Write},
    str::FromStr,
};

use bstringify::bstringify;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use derivative::Derivative;
use fixed::types::{U16F16, U2F30, U8F8};
use fixed_macro::types::{U16F16, U2F30, U8F8};
use thiserror::Error;

use crate::marshal::{aac::AACSampleEntry, av1::AV1SampleEntry, avc::AVCSampleEntry};

pub mod aac;
pub mod av1;
pub mod avc;

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

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        if let Some(value) = self {
            value.encode(output)?;
        }
        Ok(())
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) fn encode_box_header(output: &mut (impl Write + Seek), r#type: [u8; 4]) -> Result<u64> {
    let begin = output.stream_position()?;
    0u32.encode(output)?; // size
    output.write_all(&r#type)?;
    Ok(begin)
}

pub(crate) fn update_box_header(output: &mut (impl Write + Seek), begin: u64) -> Result<()> {
    let end = output.stream_position()?;
    let size = end - begin;
    output.seek(SeekFrom::Start(begin))?;
    (size as u32).encode(output)?;
    output.seek(SeekFrom::Start(end))?;
    Ok(())
}

macro_rules! decode_boxes {(
    $input:ident,
    $($quantifier:ident $type:ident $name:ident),* $(,)?
) => (
     while !$input.is_empty() {
        let size = u32::decode($input)?;
        let r#type: [u8; 4] = u32::decode($input)?.to_be_bytes();

        let (mut data, remaining_data) = $input.split_at((size - 4 - 4) as usize);
        match &r#type {
            $(bstringify!($type) => decode_box!(data $quantifier $type $name),)*
            _ => {}
        }
        *$input = remaining_data;
    }

    $(unwrap_box!($quantifier $type $name);)*
)}

macro_rules! decode_box {
    ($input:ident optional $type:ident $name:ident) => {{
        if $name.is_some() {
            return Err(Error::InvalidBoxQuantity {
                r#type: stringify!($type),
                quantity: 2,
                expected: 1,
            });
        }
        $name = Some(Decode::decode(&mut $input)?);
    }};

    ($input:ident required $type:ident $name:ident) => {{
        if $name.is_some() {
            return Err(Error::InvalidBoxQuantity {
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
        let $name = $name.ok_or(Error::InvalidBoxQuantity {
            r#type: stringify!($type),
            quantity: 0,
            expected: 1,
        })?;
    };

    (multiple $type:ident $name:ident) => {};
}

#[derive(Debug)]
pub struct File {
    pub file_type: FileTypeBox,
    pub movie: Option<MovieBox>,
    pub media_data: Vec<MediaDataBox>,
    pub meta: Option<MetaBox>,
}

impl Encode for File {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        self.file_type.encode(output)?;
        self.movie.encode(output)?;
        for media_data in &self.media_data {
            media_data.encode(output)?;
        }
        self.meta.encode(output)
    }
}

impl Decode for File {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut file_type = None;
        let mut movie = None;
        let mut media_data = Vec::new();
        let mut meta = None;

        decode_boxes! {
            input,
            required ftyp file_type,
            optional moov movie,
            multiple mdat media_data,
            optional meta meta,
        }

        Ok(Self {
            file_type,
            media_data,
            movie,
            meta,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 4.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct FileTypeBox {
    pub major_brand: FourCC,
    pub minor_version: u32,
    pub compatible_brands: Vec<FourCC>,
}

impl Encode for FileTypeBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"ftyp")?;

        self.major_brand.0.encode(output)?;
        self.minor_version.encode(output)?;
        for compatible_brand in &self.compatible_brands {
            compatible_brand.0.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for FileTypeBox {
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
// ISO/IEC 14496-12:2008 8.1.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MediaDataBox(#[derivative(Debug = "ignore")] pub Vec<u8>);

impl Encode for MediaDataBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"mdat")?;

        output.write_all(&self.0)?;

        update_box_header(output, begin)
    }
}

impl Decode for MediaDataBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let data = input.to_owned();
        *input = &input[input.len()..];
        Ok(Self(data))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.2.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MovieBox {
    pub header: MovieHeaderBox,
    pub tracks: Vec<TrackBox>,
}

impl Encode for MovieBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"moov")?;

        self.header.encode(output)?;
        for track in &self.tracks {
            track.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for MovieBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut tracks = Vec::new();

        decode_boxes! {
            input,
            required mvhd header,
            multiple trak tracks,
        }

        Ok(Self { header, tracks })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.2.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MovieHeaderBox {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub rate: U16F16,
    pub volume: U8F8,
    pub matrix: Matrix,
    pub next_track_id: u32,
}

impl Default for MovieHeaderBox {
    fn default() -> Self {
        Self {
            creation_time: 0,
            modification_time: 0,
            timescale: 0,
            duration: 0,
            rate: U16F16!(1),
            volume: U8F8!(1),
            matrix: Matrix::identity(),
            next_track_id: 0,
        }
    }
}

impl Encode for MovieHeaderBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"mvhd")?;
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
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        0u32.encode(output)?; // pre_defined
        self.next_track_id.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for MovieHeaderBox {
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
        let next_track_id = Decode::decode(input)?;
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
// ISO/IEC 14496-12:2008 8.3.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct TrackBox {
    pub header: TrackHeaderBox,
    pub media: MediaBox,
    pub edit: Option<EditBox>,
}

impl Encode for TrackBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"trak")?;

        self.header.encode(output)?;
        self.media.encode(output)?;
        self.edit.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for TrackBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut header = None;
        let mut edit = None;
        let mut media = None;

        decode_boxes! {
            input,
            required tkhd header,
            required mdia media,
            optional edts edit,
        }

        Ok(Self {
            header,
            edit,
            media,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.3.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct TrackHeaderBox {
    pub enabled: bool,
    pub in_movie: bool,
    pub in_preview: bool,
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

impl Default for TrackHeaderBox {
    fn default() -> Self {
        Self {
            enabled: true,
            in_movie: true,
            in_preview: true,
            creation_time: 0,
            modification_time: 0,
            track_id: 1,
            duration: 0,
            layer: 0,
            alternate_group: 0,
            volume: U8F8!(1),
            matrix: Matrix::identity(),
            width: U16F16!(0),
            height: U16F16!(0),
        }
    }
}

impl Encode for TrackHeaderBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"tkhd")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(
            if self.enabled { 1 << 0 } else { 0 }
                | if self.in_movie { 1 << 1 } else { 0 }
                | if self.in_preview { 1 << 2 } else { 0 },
        )?;

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
        self.height.encode(output)?;

        update_box_header(output, begin)
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
            enabled: flags & 1 << 0 != 0,
            in_movie: flags & 1 << 1 != 0,
            in_preview: flags & 1 << 2 != 0,
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
// ISO/IEC 14496-12:2008 8.4.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MediaBox {
    pub header: MediaHeaderBox,
    pub handler: HandlerBox,
    pub information: MediaInformationBox,
}

impl Encode for MediaBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"mdia")?;

        self.header.encode(output)?;
        self.handler.encode(output)?;
        self.information.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for MediaBox {
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
// ISO/IEC 14496-12:2008 8.4.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct MediaHeaderBox {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub language: u16,
}

impl Encode for MediaHeaderBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"mdhd")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.creation_time as u32).encode(output)?;
        (self.modification_time as u32).encode(output)?;
        self.timescale.encode(output)?;
        (self.duration as u32).encode(output)?;
        self.language.encode(output)?;
        0u16.encode(output)?; // pre_defined

        update_box_header(output, begin)
    }
}

impl Decode for MediaHeaderBox {
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
        let language = Decode::decode(input)?;
        assert_eq!(u16::decode(input)?, 0); // pre_defined
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
// ISO/IEC 14496-12:2008 8.4.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct HandlerBox {
    pub r#type: FourCC,
    pub name: String,
}

impl Encode for HandlerBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"hdlr")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        0u32.encode(output)?; // pre_defined
        self.r#type.0.encode(output)?;
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        0u32.encode(output)?; // reserved
        self.name.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for HandlerBox {
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
// ISO/IEC 14496-12:2008 8.4.4
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MediaInformationBox {
    pub header: MediaInformationHeader,
    pub data_information: DataInformationBox,
    pub sample_table: SampleTableBox,
}

impl Encode for MediaInformationBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"minf")?;

        match &self.header {
            MediaInformationHeader::Video(header) => header.encode(output),
            MediaInformationHeader::Sound(header) => header.encode(output),
        }?;
        self.data_information.encode(output)?;
        self.sample_table.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for MediaInformationBox {
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
// ISO/IEC 14496-12:2008 8.4.5
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum MediaInformationHeader {
    Video(VideoMediaHeaderBox),
    Sound(SoundMediaHeaderBox),
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.4.5.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct VideoMediaHeaderBox {
    pub graphicsmode: u16,
    pub opcolor: [u16; 3],
}

impl Encode for VideoMediaHeaderBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"vmhd")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(1)?; // flags

        self.graphicsmode.encode(output)?;
        for value in self.opcolor {
            value.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for VideoMediaHeaderBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let graphicsmode = Decode::decode(input)?;
        let opcolor = [
            Decode::decode(input)?,
            Decode::decode(input)?,
            Decode::decode(input)?,
        ];
        Ok(Self {
            graphicsmode,
            opcolor,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.4.5.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SoundMediaHeaderBox {
    pub balance: U8F8,
}

impl Encode for SoundMediaHeaderBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"smhd")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.balance.encode(output)?;
        0u16.encode(output)?; // reserved

        update_box_header(output, begin)
    }
}

impl Decode for SoundMediaHeaderBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let balance = U8F8::from_bits(input.read_u16::<BigEndian>()?);
        assert_eq!(input.read_u16::<BigEndian>()?, 0); // reserved
        Ok(Self { balance })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.5.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SampleTableBox {
    pub description: SampleDescriptionBox,
    pub time_to_sample: TimeToSampleBox,
    pub sync_sample: Option<SyncSampleBox>,
    pub sample_size: SampleSizeBox,
    pub sample_to_chunk: SampleToChunkBox,
    pub chunk_offset: ChunkOffsetBox,
    pub sample_to_group: Option<SampleToGroupBox>,
}

impl Encode for SampleTableBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stbl")?;

        self.description.encode(output)?;
        self.time_to_sample.encode(output)?;
        self.sync_sample.encode(output)?;
        self.sample_size.encode(output)?;
        self.sample_to_chunk.encode(output)?;
        self.chunk_offset.encode(output)?;
        self.sample_to_group.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for SampleTableBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut description = None;
        let mut time_to_sample = None;
        let mut sync_sample = None;
        let mut sample_size = None;
        let mut sample_to_chunk = None;
        let mut chunk_offset = None;
        let mut sample_to_group = None;

        decode_boxes! {
            input,
            required stsd description,
            required stts time_to_sample,
            optional stss sync_sample,
            required stsz sample_size,
            required stsc sample_to_chunk,
            required stco chunk_offset,
            optional sbgp sample_to_group,
        }

        Ok(Self {
            description,
            time_to_sample,
            sync_sample,
            sample_size,
            sample_to_chunk,
            chunk_offset,
            sample_to_group,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.5.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum SampleDescriptionBox {
    AV1(AV1SampleEntry),
    AVC(AVCSampleEntry),
    AAC(AACSampleEntry),
}

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
}

impl Encode for VisualSampleEntry {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
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
        u16::MAX.encode(output) // pre_defined
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
        Ok(Self {
            data_reference_index,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            compressorname,
            depth,
        })
    }
}

#[derive(Debug)]
pub struct AudioSampleEntry {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: U16F16,
}

impl Encode for AudioSampleEntry {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
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
        self.samplerate.encode(output)
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
        Ok(Self {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
        })
    }
}

impl Encode for SampleDescriptionBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stsd")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        1u32.encode(output)?; // entry_count

        update_box_header(output, begin)
    }
}

impl Decode for SampleDescriptionBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let mut entry = None;

        assert_eq!(u32::decode(input)?, 1); // entry_count
        let size = u32::decode(input)?;
        let r#type: [u8; 4] = u32::decode(input)?.to_be_bytes();

        let (mut data, remaining_data) = input.split_at((size - 4 - 4) as usize);
        match &r#type {
            b"av01" => entry = Some(SampleDescriptionBox::AV1(Decode::decode(&mut data)?)),
            b"avc1" => entry = Some(SampleDescriptionBox::AVC(Decode::decode(&mut data)?)),
            b"mp4a" => entry = Some(SampleDescriptionBox::AAC(Decode::decode(&mut data)?)),
            _ => {}
        }
        *input = remaining_data;

        Ok(entry.unwrap())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.6.1.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct TimeToSampleBox(pub Vec<TimeToSampleEntry>);

#[derive(Debug)]
pub struct TimeToSampleEntry {
    pub sample_count: u32,
    pub sample_delta: u32,
}

impl Encode for TimeToSampleBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stts")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.0.len() as u32).encode(output)?;
        for entry in &self.0 {
            entry.sample_count.encode(output)?;
            entry.sample_delta.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for TimeToSampleBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let sample_count = Decode::decode(input)?;
            let sample_delta = Decode::decode(input)?;
            entries.push(TimeToSampleEntry {
                sample_count,
                sample_delta,
            });
        }
        Ok(Self(entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.6.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SyncSampleBox(#[derivative(Debug = "ignore")] pub Vec<u32>);

impl Encode for SyncSampleBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stss")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.0.len() as u32).encode(output)?;
        for entry in &self.0 {
            entry.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for SyncSampleBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::new();
        for _ in 0..entry_count {
            let sample_number = Decode::decode(input)?;
            entries.push(sample_number);
        }
        Ok(Self(entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.6.5
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct EditBox {
    pub edit_list: Option<EditListBox>,
}

impl Encode for EditBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"edts")?;

        self.edit_list.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for EditBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut edit_list = None;

        decode_boxes! {
            input,
            optional elst edit_list,
        }

        Ok(Self { edit_list })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.6.6
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct EditListBox(pub Vec<EditListEntry>);

#[derive(Debug)]
pub struct EditListEntry {
    pub segment_duration: u64,
    pub media_time: u64,
    pub media_rate: U16F16,
}

impl Encode for EditListBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"elst")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.0.len() as u32).encode(output)?;
        for entry in &self.0 {
            (entry.segment_duration as u32).encode(output)?;
            (entry.media_time as u32).encode(output)?;
            entry.media_rate.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for EditListBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let version = input.read_u8()?;
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::new();
        for _ in 0..entry_count {
            let segment_duration;
            let media_time;
            match version {
                0 => {
                    segment_duration = u32::decode(input)? as u64;
                    media_time = u32::decode(input)? as u64;
                }
                1 => {
                    segment_duration = Decode::decode(input)?;
                    media_time = Decode::decode(input)?;
                }
                _ => panic!(),
            }
            let media_rate = Decode::decode(input)?;
            entries.push(EditListEntry {
                segment_duration,
                media_time,
                media_rate,
            });
        }
        Ok(Self(entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.7.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct DataInformationBox {
    pub reference: DataReferenceBox,
}

impl Default for DataInformationBox {
    fn default() -> Self {
        Self {
            reference: DataReferenceBox(vec![DataEntry::Url(DataEntryUrlBox { location: None })]),
        }
    }
}

impl Encode for DataInformationBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"dinf")?;

        self.reference.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for DataInformationBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut reference = None;

        decode_boxes! {
            input,
            required dref reference,
        }

        Ok(Self { reference })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.7.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct DataReferenceBox(pub Vec<DataEntry>);

impl Default for DataReferenceBox {
    fn default() -> Self {
        Self(vec![DataEntry::Url(Default::default())])
    }
}

#[derive(Debug)]
pub enum DataEntry {
    Url(DataEntryUrlBox),
    Urn(DataEntryUrnBox),
}

#[derive(Debug, Default)]
pub struct DataEntryUrlBox {
    pub location: Option<String>,
}

impl Encode for DataEntryUrlBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"url ")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(if self.location.is_none() { 1 << 0 } else { 0 })?; // flags

        self.location.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for DataEntryUrlBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        let flags = input.read_u24::<BigEndian>()?; // flags

        let location = if flags & 1 << 0 == 0 {
            Some(Decode::decode(input)?)
        } else {
            None
        };
        Ok(Self { location })
    }
}

#[derive(Debug)]
pub struct DataEntryUrnBox {
    pub name: String,
    pub location: String,
}

impl Encode for DataEntryUrnBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"urn ")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.name.encode(output)?;
        self.location.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for DataEntryUrnBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let name = Decode::decode(input)?;
        let location = Decode::decode(input)?;
        Ok(Self { name, location })
    }
}

impl Encode for DataReferenceBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"dref")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.0.len() as u32).encode(output)?;
        for entry in &self.0 {
            match entry {
                DataEntry::Url(entry) => entry.encode(output),
                DataEntry::Urn(entry) => entry.encode(output),
            }?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for DataReferenceBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let size = u32::decode(input)?;
            let r#type: [u8; 4] = u32::decode(input)?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 4 - 4) as usize);
            match &r#type {
                b"url " => {
                    entries.push(DataEntry::Url(Decode::decode(&mut data)?));
                }
                b"urn " => {
                    entries.push(DataEntry::Urn(Decode::decode(&mut data)?));
                }
                _ => {}
            }
            *input = remaining_data;
        }
        Ok(Self(entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.7.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub enum SampleSizeBox {
    Value { sample_size: u32, sample_count: u32 },
    PerSample(#[derivative(Debug = "ignore")] Vec<u32>),
}

impl Encode for SampleSizeBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stsz")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        match self {
            SampleSizeBox::Value {
                sample_size,
                sample_count,
            } => {
                sample_size.encode(output)?;
                sample_count.encode(output)?;
            }
            SampleSizeBox::PerSample(samples) => {
                0u32.encode(output)?; // sample_size
                (samples.len() as u32).encode(output)?;
                for sample in samples {
                    sample.encode(output)?;
                }
            }
        }

        update_box_header(output, begin)
    }
}

impl Decode for SampleSizeBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let sample_size = Decode::decode(input)?;
        let sample_count = Decode::decode(input)?;
        if sample_size != 0 {
            return Ok(SampleSizeBox::Value {
                sample_size,
                sample_count,
            });
        }
        let mut samples = Vec::default();
        for _ in 0..sample_count {
            let entry_size = Decode::decode(input)?;
            samples.push(entry_size);
        }
        Ok(SampleSizeBox::PerSample(samples))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.7.4
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SampleToChunkBox(#[derivative(Debug = "ignore")] pub Vec<SampleToChunkEntry>);

#[derive(Debug)]
pub struct SampleToChunkEntry {
    pub first_chunk: u32,
    pub samples_per_chunk: u32,
    pub sample_description_index: u32,
}

impl Encode for SampleToChunkBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stsc")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.0.len() as u32).encode(output)?;
        for entry in &self.0 {
            entry.first_chunk.encode(output)?;
            entry.samples_per_chunk.encode(output)?;
            entry.sample_description_index.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for SampleToChunkBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let first_chunk = Decode::decode(input)?;
            let samples_per_chunk = Decode::decode(input)?;
            let sample_description_index = Decode::decode(input)?;
            entries.push(SampleToChunkEntry {
                first_chunk,
                samples_per_chunk,
                sample_description_index,
            });
        }
        Ok(Self(entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.7.5
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ChunkOffsetBox(#[derivative(Debug = "ignore")] pub Vec<u32>);

impl Encode for ChunkOffsetBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"stco")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        (self.0.len() as u32).encode(output)?;
        for entry in &self.0 {
            entry.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for ChunkOffsetBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let entry_count = u32::decode(input)?;
        let mut entries = Vec::default();
        for _ in 0..entry_count {
            let chunk_offset = Decode::decode(input)?;
            entries.push(chunk_offset);
        }
        Ok(Self(entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.9.2
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SampleToGroupBox(pub FourCC, pub Vec<SampleToGroupEntry>);

#[derive(Debug)]
pub struct SampleToGroupEntry {
    pub sample_count: u32,
    pub group_description_index: u32,
}

impl Encode for SampleToGroupBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"sbgp")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.0 .0.encode(output)?;
        (self.1.len() as u32).encode(output)?;
        for entry in &self.1 {
            entry.sample_count.encode(output)?;
            entry.group_description_index.encode(output)?;
        }

        update_box_header(output, begin)
    }
}

impl Decode for SampleToGroupBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let grouping_type = FourCC(Decode::decode(input)?);
        let entry_count = u32::decode(input)?;
        let mut entries = Vec::new();
        for _ in 0..entry_count {
            let sample_count = Decode::decode(input)?;
            let group_description_index = Decode::decode(input)?;
            entries.push(SampleToGroupEntry {
                sample_count,
                group_description_index,
            });
        }
        Ok(Self(grouping_type, entries))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.11.1
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MetaBox {
    pub handler: HandlerBox,
    pub item_location: Option<ItemLocationBox>,
}

impl Encode for MetaBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"meta")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        self.handler.encode(output)?;
        self.item_location.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for MetaBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let mut handler = None;
        let mut item_location = None;

        decode_boxes! {
            input,
            required hdlr handler,
            optional iloc item_location,
        }

        Ok(Self {
            handler,
            item_location,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ISO/IEC 14496-12:2008 8.11.3
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct ItemLocationBox(Vec<ItemLocationEntry>);

#[derive(Debug)]
pub struct ItemLocationEntry {
    pub item_id: u16,
    pub data_reference_index: u16,
    pub base_offset: u64,
    pub extents: Vec<ItemLocationEntryExtent>,
}

#[derive(Debug)]
pub struct ItemLocationEntryExtent {
    pub extent_offset: u64,
    pub extent_length: u64,
}

impl Encode for ItemLocationBox {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"iloc")?;
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        update_box_header(output, begin)
    }
}

impl Decode for ItemLocationBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        assert_eq!(input.read_u8()?, 0); // version
        input.read_u24::<BigEndian>()?; // flags

        let offset_and_length_size = input.read_u8()?;
        let base_offset_size = input.read_u8()?;
        let item_count = u16::decode(input)?;
        let mut items = Vec::new();
        for _ in 0..item_count {
            let item_id = Decode::decode(input)?;
            let data_reference_index = Decode::decode(input)?;
            let base_offset = match base_offset_size & 0xF {
                0 => 0,
                4 => input.read_u32::<BigEndian>()? as u64,
                8 => input.read_u64::<BigEndian>()?,
                _ => todo!(),
            };
            let extent_count = u16::decode(input)?;
            let mut extents = Vec::new();
            for _ in 0..extent_count {
                let extent_offset = match offset_and_length_size & 0xF {
                    0 => 0,
                    4 => input.read_u32::<BigEndian>()? as u64,
                    8 => input.read_u64::<BigEndian>()?,
                    _ => todo!(),
                };
                let extent_length = match offset_and_length_size >> 4 & 0xF {
                    0 => 0,
                    4 => input.read_u32::<BigEndian>()? as u64,
                    8 => input.read_u64::<BigEndian>()?,
                    _ => todo!(),
                };
                extents.push(ItemLocationEntryExtent {
                    extent_offset,
                    extent_length,
                });
            }
            items.push(ItemLocationEntry {
                item_id,
                data_reference_index,
                base_offset,
                extents,
            })
        }
        Ok(Self(items))
    }
}
