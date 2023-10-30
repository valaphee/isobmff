use std::io::{Seek, Write};

use crate::marshal::{
    mp4::{encode_box_header, update_box_header, VisualSampleEntry},
    Decode, Encode, Result,
};

#[derive(Debug)]
pub struct AVCSampleEntry {
    pub base: VisualSampleEntry,
}

impl Encode for AVCSampleEntry {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"avc1")?;

        self.base.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for AVCSampleEntry {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            base: Decode::decode(input)?,
        })
    }
}
