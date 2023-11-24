use std::io::{Seek, Write};

use crate::marshal::{
    encode_box_header, update_box_header, AudioSampleEntry, Decode, Encode, Result,
};

#[derive(Debug)]
pub struct AACSampleEntry {
    pub base: AudioSampleEntry,
}

impl Encode for AACSampleEntry {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"mp4a")?;

        self.base.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for AACSampleEntry {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            base: Decode::decode(input)?,
        })
    }
}
