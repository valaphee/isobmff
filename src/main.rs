#![feature(portable_simd)]

use std::io::{Seek, SeekFrom, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};
use byteorder::{BigEndian, WriteBytesExt};
use fixed_macro::types::{U16F16, U2F30, U8F8};
use rav1e::prelude::*;
use windows::core::ComInterface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D11::{D3D11_CPU_ACCESS_READ, D3D11_SDK_VERSION, D3D11_USAGE_STAGING, D3D11CreateDevice, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, DXGI_MAP_READ, IDXGIFactory1, IDXGIOutput1, IDXGISurface1};
use mp4::marshall::{ChunkOffsetBox, DataEntry, DataEntryUrlBox, DataInformationBox, DataReferenceBox, Decode, Encode, File, FileTypeBox, HandlerBox, Language, Matrix, MediaBox, MediaDataBox, MediaHeaderBox, MediaInformationBox, MediaInformationHeader, MovieBox, MovieHeaderBox, SampleDescriptionBox, SampleSizeBox, SampleTableBox, SampleToChunkBox, SampleToChunkEntry, SyncSampleBox, TimeToSampleBox, TimeToSampleEntry, TrackBox, TrackHeaderBox, VideoMediaHeaderBox, VisualSampleEntry};

struct Mp4Writer<W> {
    writer: W,

    media_data_start: u64,

    chunk_buffer: Vec<u8>,
    chunk_samples: u32,
    chunk_duration: u32,
    sample_id: u32,

    sample_sizes: Vec<u32>,
    time_to_sample: Vec<TimeToSampleEntry>,
    sample_to_chunk: Vec<SampleToChunkEntry>,
    chunk_offsets: Vec<u32>,
    sync_samples: Vec<u32>,
    duration: u32,
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
        8u32.encode(&mut writer).unwrap(); // size
        u32::from_be_bytes(*b"free").encode(&mut writer).unwrap(); // type
        let media_data_start = writer.stream_position().unwrap();
        MediaDataBox {
            data: vec![],
        }.encode(&mut writer).unwrap();
        Self {
            writer,
            media_data_start,
            chunk_buffer: vec![],
            chunk_samples: 0,
            chunk_duration: 0,
            sample_id: 1,
            sample_sizes: vec![],
            time_to_sample: vec![],
            sample_to_chunk: vec![],
            chunk_offsets: vec![],
            sync_samples: vec![],
            duration: 0,
        }
    }

    fn write_sample(&mut self, data: &[u8], duration: u32, sync: bool) {
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
           self.write_chunk();
        }
        if sync {
            self.sync_samples.push(self.sample_id);
        }
        self.sample_id += 1;
        self.duration += duration
    }

    fn write_chunk(&mut self) {
        let chunk_offset = self.writer.stream_position().unwrap() as u32;
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
        self.chunk_offsets.push(chunk_offset);

        self.chunk_buffer.clear();
        self.chunk_samples = 0;
        self.chunk_duration = 0;
    }

    fn write_footer(mut self) -> W {
        self.write_chunk();

        println!("{:?}", self.sync_samples);
        let media_data_end = self.writer.stream_position().unwrap();
        let media_data_size = media_data_end - self.media_data_start;
        self.writer.seek(SeekFrom::Start(self.media_data_start)).unwrap();
        self.writer.write_u32::<BigEndian>(media_data_size as u32).unwrap();
        self.writer.seek(SeekFrom::Start(media_data_end)).unwrap();
        MovieBox {
            header: MovieHeaderBox {
                creation_time: 0,
                modification_time: 0,
                timescale: 1000,
                duration: self.duration as u64,
                rate: U16F16!(1),
                volume: U8F8!(1),
                matrix: Matrix {
                    a: U16F16!(1),
                    b: U16F16!(0),
                    u: U2F30!(0),
                    c: U16F16!(0),
                    d: U16F16!(1),
                    v: U2F30!(0),
                    x: U16F16!(0),
                    y: U16F16!(0),
                    w: U2F30!(1),
                },
                next_track_id: 2,
            },
            tracks: vec![TrackBox {
                header: TrackHeaderBox {
                    creation_time: 0,
                    modification_time: 0,
                    track_id: 1,
                    duration: self.duration as u64,
                    layer: 0,
                    alternate_group: 0,
                    volume: U8F8!(0),
                    matrix: Matrix {
                        a: U16F16!(1),
                        b: U16F16!(0),
                        u: U2F30!(0),
                        c: U16F16!(0),
                        d: U16F16!(1),
                        v: U2F30!(0),
                        x: U16F16!(0),
                        y: U16F16!(0),
                        w: U2F30!(1),
                    },
                    width: U16F16!(1920),
                    height: U16F16!(1080),
                },
                edit: None,
                media: MediaBox {
                    header: MediaHeaderBox {
                        creation_time: 0,
                        modification_time: 0,
                        timescale: 1000,
                        duration: self.duration as u64,
                        language: Language(0),
                    },
                    handler: HandlerBox { r#type: "vide".parse().unwrap(), name: "VideoHandler".to_string() },
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
                                width: 1920,
                                height: 1080,
                                horizresolution: U16F16!(72),
                                vertresolution: U16F16!(72),
                                frame_count: 1,
                                compressorname: [0; 32],
                                depth: 24,
                                config: vec![
                                    0,
                                    0,
                                    0,
                                    29,
                                    // av1C
                                    97,
                                    118,
                                    49,
                                    67,

                                    129,
                                    12,
                                    12,
                                    0,

                                    // OBU_SEQUENCE_HEADER
                                    10,
                                    15,
                                    0,
                                    0,
                                    0,
                                    98,
                                    234,
                                    127,
                                    236,
                                    251,
                                    181,
                                    242,
                                    32,
                                    33,
                                    160,
                                    48,
                                    128,
                                ],
                            }), audio: None },
                            time_to_sample: TimeToSampleBox { entries: self.time_to_sample },
                            sample_to_chunk: SampleToChunkBox { entries: self.sample_to_chunk },
                            sample_size: SampleSizeBox::PerSample(self.sample_sizes),
                            chunk_offset: ChunkOffsetBox { entries: self.chunk_offsets },
                            sync_sample: Some(SyncSampleBox { entries: self.sync_samples }),
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
    let encoder_config = EncoderConfig {
        width: 1920,
        height: 1080,
        tiles: 16,
        speed_settings: SpeedSettings::from_preset(10),
        ..Default::default()
    };
    let config = Config::new().with_encoder_config(encoder_config.clone());
    let (mut frame_sender, packet_receiver) = config.new_by_gop_channel(8).unwrap();

    std::thread::spawn(move || {
        unsafe {
            let dxgi_factory: IDXGIFactory1 = CreateDXGIFactory1().unwrap();
            let adapter = dxgi_factory.EnumAdapters1(0).unwrap();

            let mut device = Default::default();
            let mut device_context = Default::default();
            D3D11CreateDevice(
                &adapter,
                Default::default(),
                HMODULE::default(),
                Default::default(),
                Default::default(),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut device_context)
            ).unwrap();
            let device = device.unwrap();
            let device_context = device_context.unwrap();

            let output = adapter.EnumOutputs(0).unwrap();
            let output1 = output.cast::<IDXGIOutput1>().unwrap();
            let output_duplication = output1.DuplicateOutput(&device).unwrap();

            let resource = loop {
                let mut frame_info = Default::default();
                let mut resource = Default::default();
                output_duplication.AcquireNextFrame(1000, &mut frame_info, &mut resource).unwrap();
                let resource = resource.unwrap();
                if frame_info.LastPresentTime != 0 {
                    break resource;
                }
                output_duplication.ReleaseFrame().unwrap();
            };
            let texture = resource.cast::<ID3D11Texture2D>().unwrap();

            let mut copy_texture_desc = Default::default();
            texture.GetDesc(&mut copy_texture_desc);
            let mut copy_texture = Default::default();
            copy_texture_desc.Usage = D3D11_USAGE_STAGING;
            copy_texture_desc.BindFlags = 0;
            copy_texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            copy_texture_desc.MiscFlags = 0;
            device.CreateTexture2D(&copy_texture_desc, None, Some(&mut copy_texture)).unwrap();
            let copy_texture = copy_texture.unwrap();
            let copy_surface = copy_texture.cast::<IDXGISurface1>().unwrap();
            output_duplication.ReleaseFrame().unwrap();

            let mut frame = frame_sender.new_frame();
            let mut mapped_rect = Default::default();
            for _ in 0..60 * 8 {
                let begin = Instant::now();
                let resource = loop {
                    let mut frame_info = Default::default();
                    let mut resource = Default::default();
                    output_duplication.AcquireNextFrame(1000, &mut frame_info, &mut resource).unwrap();
                    let resource = resource.unwrap();
                    if frame_info.LastPresentTime != 0 {
                        break resource;
                    }
                    output_duplication.ReleaseFrame().unwrap();
                };
                let texture = resource.cast::<ID3D11Texture2D>().unwrap();
                device_context.CopyResource(&copy_texture, &texture);
                output_duplication.ReleaseFrame().unwrap();

                copy_surface.Map(&mut mapped_rect, DXGI_MAP_READ).unwrap();
                let bgra = std::slice::from_raw_parts_mut(mapped_rect.pBits, (copy_texture_desc.Width * copy_texture_desc.Height * 4) as usize);
                let bgra_stride = mapped_rect.Pitch as usize;
                for (plane_idx, plane) in frame.planes.iter_mut().enumerate() {
                    let height = plane.cfg.height;
                    let width = plane.cfg.width;
                    let stride = plane.cfg.stride;
                    let x_decimator = plane.cfg.xdec + 1;
                    let y_decimator = plane.cfg.ydec + 1;

                    let data = plane.data_origin_mut();
                    for y in 0..height {
                        let bgra_row = &mut bgra[y * y_decimator * bgra_stride..];
                        let data_row = &mut data[y * stride..];
                        for x in 0..width {
                            data_row[x] = bgra_row[x * x_decimator * 4 + plane_idx];
                        }
                    }
                }
                copy_surface.Unmap().unwrap();

                frame_sender.send(frame.clone()).unwrap();

                let elapsed = begin.elapsed();
                if elapsed >= Duration::from_millis(1000 / 60) {
                    println!("Frame took too long! ({:?})", elapsed);
                } else {
                    sleep(Duration::from_millis(1000 / 60) - elapsed);
                }
            }
        }
    });

    let mut writer = Mp4Writer::write_header(std::fs::File::create("test.mp4").unwrap());
    loop {
        match packet_receiver.recv() {
            Ok(packet) => writer.write_sample(&packet.data, 32, packet.frame_type == FrameType::KEY),
            Err(_) => break,
        }
    }
    writer.write_footer();
}

fn test(hello: [u8; 4]) -> u32 {
    u32::from_be_bytes(hello)
}

fn world() {
    test(*b"HELL");
}