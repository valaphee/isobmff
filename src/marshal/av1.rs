use std::io::{Seek, Write};

use crate::marshal::{
    encode_box_header, update_box_header, Decode, Encode, Result, VisualSampleEntry,
};

#[derive(Debug)]
pub struct AV1SampleEntry {
    pub base: VisualSampleEntry,
}

impl Encode for AV1SampleEntry {
    fn encode(&self, output: &mut (impl Write + Seek)) -> Result<()> {
        let begin = encode_box_header(output, *b"av01")?;

        self.base.encode(output)?;

        update_box_header(output, begin)
    }
}

impl Decode for AV1SampleEntry {
    fn decode(input: &mut &[u8]) -> Result<Self> {
        Ok(Self {
            base: Decode::decode(input)?,
        })
    }
}
