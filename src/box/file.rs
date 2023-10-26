use std::io::Write;

use derivative::Derivative;

use crate::r#box::{
    movie::{MediaData, Movie},
    Decode, Encode, FourCC, Result,
};

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
            + self.movie.size()
            + self.media_data.iter().map(Encode::size).sum::<u64>()
            + 4
            + 4
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

        while !input.is_empty() {
            let size = u32::decode(input)?;
            let r#type: [u8; 4] = u32::decode(input)?.to_be_bytes();

            let (mut data, remaining_data) = input.split_at((size - 4 - 4) as usize);
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
                b"mdat" => media_data.push(Decode::decode(&mut data)?),
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
            media_data,
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
