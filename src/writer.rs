use std::io::{Seek, Write};

use fixed_macro::types::U16F16;

use crate::marshal::{
    av1::AV1SampleEntry, encode_box_header, update_box_header, ChunkOffsetBox, Encode, FileTypeBox,
    HandlerBox, MediaBox, MediaHeaderBox, MediaInformationBox, MediaInformationHeader, MovieBox,
    MovieHeaderBox, SampleDescriptionBox, SampleSizeBox, SampleTableBox, SampleToChunkBox,
    SampleToChunkEntry, SyncSampleBox, TimeToSampleBox, TimeToSampleEntry, TrackBox,
    TrackHeaderBox, VisualSampleEntry,
};

pub struct Writer<W> {
    output: W,
    media_data_box_begin: u64,
    tracks: Vec<Track>,
}

impl<W: Write + Seek> Writer<W> {
    pub fn new(mut output: W) -> Self {
        FileTypeBox {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                "isom".parse().unwrap(),
                "av01".parse().unwrap(),
                "iso2".parse().unwrap(),
                "mp41".parse().unwrap(),
            ],
        }
        .encode(&mut output)
        .unwrap();

        8u32.encode(&mut output).unwrap(); // size
        u32::from_be_bytes(*b"free").encode(&mut output).unwrap(); // type

        let media_data_box_begin = encode_box_header(&mut output, *b"mdat").unwrap();
        Writer {
            output,
            media_data_box_begin,
            tracks: vec![],
        }
    }

    pub fn sample(&mut self, track: u32, duration: u32, sync: bool, data: &[u8]) {
        self.tracks[track as usize].write_sample(&mut self.output, duration, sync, data);
    }

    pub fn finish(mut self) -> W {
        for track in &mut self.tracks {
            track.write_chunk(&mut self.output);
        }
        update_box_header(&mut self.output, self.media_data_box_begin).unwrap();

        MovieBox {
            header: MovieHeaderBox {
                timescale: 1000,
                next_track_id: self.tracks.len() as u32 + 1,
                ..Default::default()
            },
            tracks: self
                .tracks
                .into_iter()
                .map(|track| track.into_box())
                .collect(),
        }
        .encode(&mut self.output)
        .unwrap();

        self.output
    }
}

struct Track {
    total_duration: u32,

    sample_times: Vec<TimeToSampleEntry>,
    sample_syncs: Vec<u32>,
    sample_sizes: Vec<u32>,

    chunk_buffer: Vec<u8>,
    chunk_sample_count: u32,
    chunk_duration: u32,
    chunk_samples: Vec<SampleToChunkEntry>,
    chunk_offsets: Vec<u32>,
}

impl Track {
    fn write_sample(
        &mut self,
        output: &mut (impl Write + Seek),
        duration: u32,
        sync: bool,
        data: &[u8],
    ) {
        self.total_duration += duration;

        // Push to sample times, syncs and sizes
        self.add_sample_time(duration);
        if sync {
            let sample_id = self.sample_sizes.len() as u32 + 1;
            self.sample_syncs.push(sample_id);
        }
        self.sample_sizes.push(data.len() as u32);

        // Add to chunk buffer, and increment samples and duration of the current chunk
        self.chunk_buffer.extend_from_slice(data);
        self.chunk_sample_count += 1;
        self.chunk_duration += duration;

        // Check if chunk should be written
        if self.chunk_duration >= 1000 {
            self.write_chunk(output);
        }
    }

    fn write_chunk(&mut self, output: &mut (impl Write + Seek)) {
        // Push to chunk samples and offsets
        let chunk_offset = output.stream_position().unwrap() as u32;
        output.write_all(&self.chunk_buffer).unwrap();
        self.add_chunk_samples();
        self.chunk_offsets.push(chunk_offset);

        // Reset current chunk
        self.chunk_buffer.clear();
        self.chunk_sample_count = 0;
        self.chunk_duration = 0;
    }

    fn add_sample_time(&mut self, duration: u32) {
        if let Some(entry) = self.sample_times.last_mut() {
            if entry.sample_delta == duration {
                entry.sample_count += 1;
                return;
            }
        }

        self.sample_times.push(TimeToSampleEntry {
            sample_count: 1,
            sample_delta: duration,
        });
    }

    fn add_chunk_samples(&mut self) {
        if let Some(entry) = self.chunk_samples.last_mut() {
            if entry.samples_per_chunk == self.chunk_sample_count {
                return;
            }
        }

        let chunk_id = self.chunk_offsets.len() as u32 + 1;
        self.chunk_samples.push(SampleToChunkEntry {
            first_chunk: chunk_id,
            samples_per_chunk: self.chunk_sample_count,
            sample_description_index: 1,
        });
    }

    fn into_box(self) -> TrackBox {
        TrackBox {
            header: TrackHeaderBox {
                track_id: 1,
                duration: self.total_duration as u64,
                width: U16F16!(1920),
                height: U16F16!(1080),
                ..Default::default()
            },
            media: MediaBox {
                header: MediaHeaderBox {
                    timescale: 1000,
                    duration: self.total_duration as u64,
                    ..Default::default()
                },
                handler: HandlerBox {
                    r#type: "vide".parse().unwrap(),
                    name: "VideoHandler".to_string(),
                },
                information: MediaInformationBox {
                    header: MediaInformationHeader::Video(Default::default()),
                    data_information: Default::default(),
                    sample_table: SampleTableBox {
                        description: SampleDescriptionBox::AV1(AV1SampleEntry {
                            base: VisualSampleEntry {
                                data_reference_index: 1,
                                width: 1920,
                                height: 1080,
                                horizresolution: U16F16!(72),
                                vertresolution: U16F16!(72),
                                frame_count: 1,
                                compressorname: [0; 32],
                                depth: 24,
                            },
                        }),
                        time_to_sample: TimeToSampleBox(self.sample_times),
                        sync_sample: Some(SyncSampleBox(self.sample_syncs)),
                        sample_size: SampleSizeBox::PerSample(self.sample_sizes),
                        sample_to_chunk: SampleToChunkBox(self.chunk_samples),
                        chunk_offset: ChunkOffsetBox(self.chunk_offsets),
                        sample_to_group: None,
                    },
                },
            },
            edit: None,
        }
    }
}
