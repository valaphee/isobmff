use std::io::{Seek, Write};
use mp4::marshall::{ChunkOffsetBox, DataEntry, DataEntryUrlBox, DataInformationBox, DataReferenceBox, Encode, FileTypeBox, HandlerBox, Language, Matrix, MediaBox, MediaHeaderBox, MediaInformationBox, MediaInformationHeader, MovieBox, MovieHeaderBox, SampleDescriptionBox, SampleSizeBox, SampleTableBox, SampleToChunkBox, SampleToChunkEntry, TimeToSampleBox, TimeToSampleEntry, TrackBox, TrackHeaderBox, VideoMediaHeaderBox, VisualSampleEntry};

struct Mp4Writer<W> {
    writer: W,

    chunk_buffer: Vec<u8>,
    chunk_samples: u32,
    chunk_duration: u32,
    sample_id: u32,

    sample_sizes: Vec<u32>,
    time_to_sample: Vec<TimeToSampleEntry>,
    sample_to_chunk: Vec<SampleToChunkEntry>,
    chunk_offsets: Vec<u32>,
}

impl<W: Write + Seek> Mp4Writer<W> {
    fn write_header(mut writer: W) -> Self {
        FileTypeBox {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                "isom".parse().unwrap(),
                "av01".parse().unwrap(),
                "iso2".parse().unwrap(),
                "mp41".parse().unwrap(),
            ],
        }.encode(&mut writer).unwrap();
        8u32.encode(&mut writer)?; // size
        u32::from_be_bytes(*b"free").encode(&mut writer)?; // type
        Self {
            writer,
            chunk_buffer: vec![],
            chunk_samples: 0,
            chunk_duration: 0,
            sample_id: 1,
            sample_sizes: vec![],
            time_to_sample: vec![],
            sample_to_chunk: vec![],
            chunk_offsets: vec![],
        }
    }

    fn write_sample(&mut self, data: &[u8], duration: u32) {
        self.chunk_buffer.extend_from_slice(&data);
        self.chunk_samples += 1;
        self.chunk_duration += duration;
        self.sample_sizes.push(data.len() as u32);
        loop {
            if let Some(entry) = self.time_to_sample.last_mut() {
                if entry.sample_delta == duration {
                    entry.sample_count += 1;
                    break;
                }
            }
            self.time_to_sample.push(TimeToSampleEntry {
                sample_count: 1,
                sample_delta: duration,
            });
            break;
        }
        if self.chunk_duration >= 1000 {
            self.writer.write_all(&self.chunk_buffer).unwrap();
            loop {
                if let Some(entry) = self.sample_to_chunk.last_mut() {
                    if entry.samples_per_chunk == self.chunk_samples {
                        break;
                    }
                }
                let chunk_id = self.chunk_offsets.len() as u32 + 1;
                self.sample_to_chunk.push(SampleToChunkEntry {
                    first_chunk: chunk_id,
                    samples_per_chunk: self.chunk_samples,
                    sample_description_index: 1,
                });
                break;
            }
            self.chunk_offsets.push(self.writer.stream_position().unwrap() as u32);

            self.chunk_buffer.clear();
            self.chunk_samples = 0;
            self.chunk_duration = 0;
        }
    }

    fn write_footer(mut self) -> W {
        MovieBox {
            header: MovieHeaderBox {
                creation_time: 0,
                modification_time: 0,
                timescale: 1000,
                duration: 0,
                rate: 1.into(),
                volume: 1.into(),
                matrix: Matrix {
                    a: 1.into(),
                    b: 0.into(),
                    u: 0.into(),
                    c: 0.into(),
                    d: 1.into(),
                    v: 0.into(),
                    x: 0.into(),
                    y: 0.into(),
                    w: 1.into(),
                },
                next_track_id: 2,
            },
            tracks: vec![TrackBox {
                header: TrackHeaderBox {
                    creation_time: 0,
                    modification_time: 0,
                    track_id: 1,
                    duration: 0,
                    layer: 0,
                    alternate_group: 0,
                    volume: 1.into(),
                    matrix: Matrix {
                        a: 1.into(),
                        b: 0.into(),
                        u: 0.into(),
                        c: 0.into(),
                        d: 1.into(),
                        v: 0.into(),
                        x: 0.into(),
                        y: 0.into(),
                        w: 1.into(),
                    },
                    width: 2560.into(),
                    height: 1440.into(),
                },
                edit: None,
                media: MediaBox {
                    header: MediaHeaderBox {
                        creation_time: 0,
                        modification_time: 0,
                        timescale: 1000,
                        duration: 0,
                        language: Language(0),
                    },
                    handler: HandlerBox { r#type: "vide".into(), name: "VideoHandler".to_string() },
                    information: MediaInformationBox {
                        header: MediaInformationHeader::Video(VideoMediaHeaderBox {
                            graphicsmode: 0,
                            opcolor: [0, 0, 0],
                        }),
                        data_information: DataInformationBox {
                            reference: DataReferenceBox {
                                entries: vec![
                                    DataEntry::Url(DataEntryUrlBox {
                                        location: "".to_string(),
                                    })
                                ]
                            }
                        },
                        sample_table: SampleTableBox {
                            description: SampleDescriptionBox { visual: Some(VisualSampleEntry {
                                data_reference_index: 1,
                                width: 2560,
                                height: 1440,
                                horizresolution: 72.into(),
                                vertresolution: 72.into(),
                                frame_count: 1,
                                compressorname: [0; 32],
                                depth: 24,
                                config: vec![
                                    0,
                                    0,
                                    0,
                                    28,
                                    97,
                                    118,
                                    49,
                                    67,
                                    129,
                                    4,
                                    12,
                                    0,
                                    10,
                                    14,
                                    0,
                                    0,
                                    0,
                                    36,
                                    197,
                                    23,
                                    223,
                                    118,
                                    190,
                                    68,
                                    4,
                                    52,
                                    6,
                                    16,
                                    0,
                                    0,
                                    0,
                                    19,
                                    99,
                                    111,
                                    108,
                                    114,
                                    110,
                                    99,
                                    108,
                                    120,
                                    0,
                                    1,
                                    0,
                                    13,
                                    0,
                                    1,
                                    128,
                                    0,
                                    0,
                                    0,
                                    20,
                                    98,
                                    116,
                                    114,
                                    116,
                                    0,
                                    0,
                                    0,
                                    0,
                                    0,
                                    38,
                                    37,
                                    160,
                                    0,
                                    11,
                                    151,
                                    249,
                                ],
                            }), audio: None },
                            time_to_sample: TimeToSampleBox { entries: self.time_to_sample },
                            sample_to_chunk: SampleToChunkBox { entries: self.sample_to_chunk },
                            sample_size: SampleSizeBox::PerSample(self.sample_sizes),
                            chunk_offset: ChunkOffsetBox { entries: self.chunk_offsets },
                            sync_sample: None,
                            sample_to_group: None,
                        },
                    },
                },
            }],
        }.encode(&mut self.writer).unwrap();
        self.writer
    }
}

fn main() {
    let mut writer = Mp4Writer::write_header(std::fs::File::create("test.mp4").unwrap());
    writer.write_footer();
}
