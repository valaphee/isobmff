use std::io::Write;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::{Decode, Encode, Result};
use crate::movie::MovieBox;

#[derive(Debug)]
pub struct FileBox {
    pub file_type: FileTypeBox,
    pub movie: MovieBox,
}

impl Encode for FileBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        self.file_type.encode(output)?;
        self.movie.encode(output)?;
        Ok(())
    }
}

impl Decode for FileBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        let mut file_type = Default::default();
        let mut movie = Default::default();
        while !input.is_empty() {
            let size = input.read_u32::<BigEndian>()?;
            let r#type: [u8; 4] = input.read_u32::<BigEndian>()?.to_be_bytes();
            println!("File {} {}", size, std::str::from_utf8(&r#type).unwrap());

            let (mut data, remaining_data) = input.split_at((size - 8) as usize);
            match &r#type {
                b"ftyp" => file_type = Some(Decode::decode(&mut data)?),
                b"moov" => movie = Some(Decode::decode(&mut data)?),
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
pub struct FileTypeBox {
    pub major_brand: u32,
    pub minor_version: u32,
    pub compatible_brands: Vec<u32>,
}

impl Encode for FileTypeBox {
    fn encode(&self, output: &mut impl Write) -> Result<()> {
        output.write_u32::<BigEndian>(8 + 4 + 4 + self.compatible_brands.len() as u32 * 4)?;
        output.write_u32::<BigEndian>(u32::from_be_bytes(*b"ftyp"))?;

        output.write_u32::<BigEndian>(self.major_brand)?;
        output.write_u32::<BigEndian>(self.minor_version)?;
        for compatible_brand in &self.compatible_brands {
            output.write_u32::<BigEndian>(*compatible_brand)?;
        }

        Ok(())
    }
}

impl Decode for FileTypeBox {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            major_brand: input.read_u32::<BigEndian>()?,
            minor_version: input.read_u32::<BigEndian>()?,
            compatible_brands: input.chunks(4).map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap())).collect(),
        })
    }
}
