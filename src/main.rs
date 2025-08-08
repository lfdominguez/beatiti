use std::collections::VecDeque;
use aubio::{OnsetMode, Smpl, Tempo};
use pipewire::spa::param::format::{MediaSubtype, MediaType};

struct UserData {
    format: pipewire::spa::param::audio::AudioInfoRaw,
    tempo: Option<Tempo>,
    buffer: VecDeque<Smpl>,  // Ring buffer for audio samples
    bpm_history: Vec<Smpl>,  // For smoothing BPM values
    avg_bpm: Option<Smpl>,    // Current averaged BPM
    sample_count: usize,
}

const I16_TO_SMPL: Smpl = 1.0 / (1 << 16) as Smpl;
const BUF_SIZE: usize = 1024;
const HOP_SIZE: usize = BUF_SIZE / 2;
const BPM_HISTORY_SIZE: usize = 5;  // Number of BPM values to average

fn main() -> Result<(), pipewire::Error> {
    pipewire::init();

    let mainloop = pipewire::main_loop::MainLoop::new(None)?;
    let context = pipewire::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let data = UserData {
        format: Default::default(),
        tempo: None,
        buffer: VecDeque::with_capacity(BUF_SIZE * 2),  // Buffer for accumulating samples
        bpm_history: Vec::with_capacity(BPM_HISTORY_SIZE),
        avg_bpm: None,
        sample_count: 0
    };

    let props = pipewire::properties::properties! {
        *pipewire::keys::MEDIA_TYPE => "Audio",
        *pipewire::keys::MEDIA_CATEGORY => "Capture",
        *pipewire::keys::MEDIA_ROLE => "Music",
        *pipewire::keys::STREAM_CAPTURE_SINK => "true",
    };

    let stream = pipewire::stream::Stream::new(&core, "audio-capture", props)?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(move |_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };

            if id != pipewire::spa::param::ParamType::Format.as_raw() {
                return;
            }

            let (media_type, media_subtype) = match pipewire::spa::param::format_utils::parse_format(param) {
                Ok(v) => v,
                Err(_) => return
            };

            if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                return;
            }

            user_data.format
                .parse(param)
                .expect("Failed to parse param changed to AudioInfoRaw");

            user_data.tempo = Some(
                Tempo::new(OnsetMode::Hfc, BUF_SIZE, HOP_SIZE, user_data.format.rate())
                    .unwrap()
                    .with_silence(-40.0)
                    .with_threshold(0.1)
            );

            // Clear any existing buffers when format changes
            user_data.buffer.clear();
            user_data.bpm_history.clear();
            user_data.avg_bpm = None;

            println!(
                "capturing rate:{} channels:{}",
                user_data.format.rate(),
                user_data.format.channels()
            );
        })
        .process(|stream, user_data| match stream.dequeue_buffer() {
            None => println!("No buffer available"),

            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }

                let data = &mut datas[0];

                let Some(tempo) = user_data.tempo.as_mut() else { return };

                if let Some(samples) = data.data() {
                    let channels = user_data.format.channels() as usize;
                    let i16_samples: Vec<i16> = samples
                        .chunks_exact(2)
                        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();

                    // Convert to mono and add to buffer
                    for chunk in i16_samples.chunks(channels) {
                        if chunk.len() != channels { break; }

                        // Convert to mono by averaging channels
                        let mut sample_sum = 0.0;
                        for &s in chunk {
                            sample_sum += s as Smpl * I16_TO_SMPL;
                        }
                        let mono_sample = sample_sum / channels as Smpl;
                        user_data.buffer.push_back(mono_sample);
                        user_data.sample_count += 1;
                    }


                    // Process every HOP_SIZE samples (non-overlapping windows for simplicity)
                    while user_data.buffer.len() >= HOP_SIZE {
                        let mut input = Vec::with_capacity(HOP_SIZE);

                        // Extract exactly HOP_SIZE samples
                        for _ in 0..HOP_SIZE {
                            input.push(user_data.buffer.pop_front().unwrap());
                        }

                        // Process with aubio
                        if let Ok(_is_beat) = tempo.do_result(&input) {
                            let bpm = tempo.get_bpm();
                            let confidence = tempo.get_confidence();

                            // Only process meaningful BPM values
                            if bpm > 0.0 && bpm >= 30.0 && bpm <= 300.0 {
                                // Add to history
                                if user_data.bpm_history.len() >= BPM_HISTORY_SIZE {
                                    user_data.bpm_history.remove(0);
                                }
                                user_data.bpm_history.push(bpm);

                                // Calculate running average
                                let avg_bpm: Smpl = user_data.bpm_history.iter().sum::<Smpl>()
                                    / user_data.bpm_history.len() as Smpl;

                                user_data.avg_bpm = Some(avg_bpm);

                                // Simple octave error correction based on common patterns
                                let corrected_bpm = if user_data.bpm_history.len() >= 3 {
                                    let recent_avg = user_data.bpm_history.iter().rev().take(3).sum::<Smpl>() / 3.0;

                                    // Check for consistent octave errors
                                    if recent_avg > 150.0 && confidence < 0.7 {
                                        recent_avg / 2.0  // Likely doubled
                                    } else if recent_avg < 50.0 && confidence < 0.7 {
                                        recent_avg * 2.0  // Likely halved
                                    } else {
                                        avg_bpm
                                    }
                                } else {
                                    avg_bpm
                                };

                                print!("BPM: {:.1} (raw: {:.1}, conf: {:.2}, samples: {})                                      \r",
                                         corrected_bpm, bpm, confidence, user_data.bpm_history.len());

                                // Only print updates periodically to avoid spam
                                // if user_data.sample_count % (user_data.format.rate() as usize / 4) == 0 {
                                //     println!("BPM: {:.1} (raw: {:.1}, conf: {:.2}, samples: {})",
                                //              corrected_bpm, bpm, confidence, user_data.bpm_history.len());
                                // }
                            }
                        }
                    }
                }
            }
        })
        .register()?;

    let mut audio_info = pipewire::spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(pipewire::spa::param::audio::AudioFormat::S16LE);
    audio_info.set_channels(2);  // Request stereo input for better detection

    let obj = pipewire::spa::pod::Object {
        type_: pipewire::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pipewire::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into()
    };

    let values: Vec<u8> =
        pipewire::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pipewire::spa::pod::Value::Object(obj)
        )
        .unwrap()
        .0
        .into_inner();

    let mut params = [pipewire::spa::pod::Pod::from_bytes(&values).unwrap()];

    stream.connect(
        pipewire::spa::utils::Direction::Input,
        None,
        pipewire::stream::StreamFlags::AUTOCONNECT
            | pipewire::stream::StreamFlags::MAP_BUFFERS
            | pipewire::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    mainloop.run();

    Ok(())
}
