use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Bounded ~512 KB stereo f32 ring buffer so even huge FLAC/WAV files use < 1 MB RAM.
const MAX_BUFFER_SAMPLES: usize = 48_000 * 2 * 2;

pub struct LocalAudioStream {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    stop: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
    seek_req_ms: Arc<AtomicU64>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
}

impl LocalAudioStream {
    pub fn open(path: PathBuf, target_sample_rate: u32) -> Result<Self, String> {
        let file = File::open(&path)
            .map_err(|e| format!("Failed opening audio file {}: {e}", path.display()))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("Unsupported audio format: {e}"))?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| "Audio file contains no default audio track".to_string())?;
        let track_id = track.id;
        let codec_params = track.codec_params.clone();
        let src_rate = codec_params.sample_rate.unwrap_or(44_100).max(8_000);
        let duration_ms_val = codec_params
            .n_frames
            .map(|n| (n as f64 / src_rate as f64 * 1000.0) as u64)
            .unwrap_or(0);

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed creating audio decoder: {e}"))?;

        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BUFFER_SAMPLES / 2)));
        let stop = Arc::new(AtomicBool::new(false));
        let ended = Arc::new(AtomicBool::new(false));
        let seek_req_ms = Arc::new(AtomicU64::new(u64::MAX));
        let position_ms = Arc::new(AtomicU64::new(0));
        let duration_ms = Arc::new(AtomicU64::new(duration_ms_val));

        let buf_c = Arc::clone(&buffer);
        let stop_c = Arc::clone(&stop);
        let ended_c = Arc::clone(&ended);
        let seek_c = Arc::clone(&seek_req_ms);
        let pos_c = Arc::clone(&position_ms);

        let handle = thread::Builder::new()
            .name("kissa-audio-decode".into())
            .spawn(move || {
                let ratio = src_rate as f64 / target_sample_rate.max(8_000) as f64;
                let mut sample_buf: Option<SampleBuffer<f32>> = None;
                let mut frac_pos = 0.0_f64;
                let mut total_src_frames: u64 = 0;

                while !stop_c.load(Ordering::Relaxed) {
                    // Handle seek requests
                    let req = seek_c.swap(u64::MAX, Ordering::AcqRel);
                    if req != u64::MAX {
                        let secs = req / 1000;
                        let frac = (req % 1000) as f64 / 1000.0;
                        if format
                            .seek(
                                SeekMode::Coarse,
                                SeekTo::Time {
                                    time: Time::new(secs, frac),
                                    track_id: Some(track_id),
                                },
                            )
                            .is_ok()
                        {
                            decoder.reset();
                            if let Ok(mut guard) = buf_c.lock() {
                                guard.clear();
                            }
                            total_src_frames = (req as f64 / 1000.0 * src_rate as f64) as u64;
                            pos_c.store(req, Ordering::Relaxed);
                            ended_c.store(false, Ordering::Relaxed);
                        }
                    }

                    // Keep ring buffer bounded for ultra-low RAM
                    let current_len = buf_c.lock().map(|g| g.len()).unwrap_or(MAX_BUFFER_SAMPLES);
                    if current_len >= MAX_BUFFER_SAMPLES {
                        thread::sleep(Duration::from_millis(18));
                        continue;
                    }

                    let packet = match format.next_packet() {
                        Ok(p) => p,
                        Err(SymphError::IoError(_)) => {
                            ended_c.store(true, Ordering::Relaxed);
                            thread::sleep(Duration::from_millis(40));
                            continue;
                        }
                        Err(_) => {
                            ended_c.store(true, Ordering::Relaxed);
                            break;
                        }
                    };

                    if packet.track_id() != track_id {
                        continue;
                    }

                    let decoded = match decoder.decode(&packet) {
                        Ok(d) => d,
                        Err(SymphError::DecodeError(_)) => continue,
                        Err(_) => break,
                    };

                    let spec = *decoded.spec();
                    let channels = spec.channels.count().max(1);
                    let frames = decoded.frames();
                    if frames == 0 {
                        continue;
                    }

                    let sbuf = sample_buf.get_or_insert_with(|| {
                        SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)
                    });
                    sbuf.copy_interleaved_ref(decoded);
                    let samples = sbuf.samples();

                    let mut out = Vec::with_capacity((frames as f64 / ratio) as usize * 2 + 8);
                    while (frac_pos as usize) < frames {
                        let i0 = frac_pos as usize;
                        let i1 = (i0 + 1).min(frames - 1);
                        let t = (frac_pos - i0 as f64) as f32;

                        let l0 = samples[i0 * channels];
                        let r0 = if channels > 1 {
                            samples[i0 * channels + 1]
                        } else {
                            l0
                        };
                        let l1 = samples[i1 * channels];
                        let r1 = if channels > 1 {
                            samples[i1 * channels + 1]
                        } else {
                            l1
                        };
                        out.push(l0 + (l1 - l0) * t);
                        out.push(r0 + (r1 - r0) * t);
                        frac_pos += ratio;
                    }
                    frac_pos -= frames as f64;
                    total_src_frames += frames as u64;
                    pos_c.store(
                        (total_src_frames as f64 / src_rate as f64 * 1000.0) as u64,
                        Ordering::Relaxed,
                    );

                    if let Ok(mut guard) = buf_c.lock() {
                        guard.extend(out);
                    }
                }
            })
            .map_err(|e| format!("Failed spawning audio decoder thread: {e}"))?;

        Ok(Self {
            buffer,
            stop,
            ended,
            seek_req_ms,
            position_ms,
            duration_ms,
            handle: Some(handle),
        })
    }

    pub fn buffer(&self) -> Arc<Mutex<VecDeque<f32>>> {
        Arc::clone(&self.buffer)
    }

    pub fn seek_seconds(&self, secs: f32) {
        self.seek_req_ms
            .store((secs.max(0.0) * 1000.0) as u64, Ordering::Release);
    }

    pub fn position_secs(&self) -> f32 {
        self.position_ms.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn duration_secs(&self) -> f32 {
        self.duration_ms.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn is_finished(&self) -> bool {
        self.ended.load(Ordering::Relaxed)
            && self.buffer.lock().map(|b| b.is_empty()).unwrap_or(true)
    }
}

impl Drop for LocalAudioStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
