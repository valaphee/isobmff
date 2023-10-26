use std::{
    hash::Hasher,
    io::{Read, Write},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use derivative::Derivative;
use fixed::types::U16F16;

use crate::r#box::{Decode, Encode, FourCC, Result};

// 8.14
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
                b"stsz" => {
                    assert!(sample_size.is_none());
                    sample_size = Some(Decode::decode(&mut data)?)
                }
                b"stz2" => {}
                b"stco" => {
                    assert!(chunk_offset.is_none());
                    chunk_offset = Some(Decode::decode(&mut data)?)
                }
                b"co64" => {}
                b"stss" => {
                    assert!(sync_sample.is_none());
                    sync_sample = Some(Decode::decode(&mut data)?)
                }
                b"stsh" => {}
                b"padb" => {}
                b"stdp" => {}
                b"sdtp" => {}
                b"sbgp" => {
                    assert!(sample_to_group.is_none());
                    sample_to_group = Some(Decode::decode(&mut data)?)
                }
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
            sample_size: sample_size.unwrap(),
            chunk_offset: chunk_offset.unwrap(),
            sync_sample,
            sample_to_group,
        })
    }
}

// 8.15.2
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

// 8.16
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
    fn size(&self) -> u64 {
        4 + 4 + 6 * 1 + 2 + 2 + 2 + 3 * 4 + 2 + 2 + 4 + 4 + 4 + 2 + 32 + 2 + 2
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
        u16::MAX.encode(output)
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
    fn size(&self) -> u64 {
        4 + 4 + 6 * 1 + 2 + 2 * 4 + 2 + 2 + 2 + 2 + 4
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

#[derive(Debug)]
pub struct SampleDescription {
    pub avc1: Option<VisualSampleEntry>,
    pub mp4a: Option<AudioSampleEntry>,
}

impl Encode for SampleDescription {
    fn size(&self) -> u64 {
        4 + 4 + 1 + 3 + 4 + self.avc1.as_ref().map_or(0, Encode::size)
    }

    fn encode(&self, output: &mut impl Write) -> Result<()> {
        (self.size() as u32).encode(output)?; // size
        u32::from_be_bytes(*b"stsd").encode(output)?; // type
        output.write_u8(0)?; // version
        output.write_u24::<BigEndian>(0)?; // flags

        if let Some(avc1) = &self.avc1 {
            avc1.encode(output)?;
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

// 8.17
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

// 8.18
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

// 8.19
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

// 8.20
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

// 8.40.3.2
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
