pub mod ambience;
pub mod decoder;
pub mod dsp;
pub mod synth;

use ambience::AmbientMixer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use decoder::LocalAudioStream;
use dsp::EqChain;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use synth::LofiSynth;

enum AudioCmd {
    SetPlaying(bool),
    SelectStation(String),
    AttachLocalBuffer(Option<Arc<Mutex<VecDeque<f32>>>>),
    SetVolume(f32),
    SetMuted(bool),
    SetSleepFade(f32),
    SetEq([f32; 5]),
    SetTapeCutoff(f32),
    SetAmbience(HashMap<String, f32>),
    PlayChime,
}

pub struct AudioEngine {
    tx: Sender<AudioCmd>,
    _stream: Option<cpal::Stream>,
    sample_rate: u32,
    playing: bool,
    local_stream: Option<LocalAudioStream>,
    #[allow(dead_code)]
    pub init_error: Option<String>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<AudioCmd>();

        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            return Self {
                tx,
                _stream: None,
                sample_rate: 44_100,
                playing: false,
                local_stream: None,
                init_error: Some("No default audio output device found".to_string()),
            };
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                return Self {
                    tx,
                    _stream: None,
                    sample_rate: 44_100,
                    playing: false,
                    local_stream: None,
                    init_error: Some(format!("Audio config error: {e}")),
                }
            }
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let mut synth = LofiSynth::new(sample_rate as f32);
        let mut ambience = AmbientMixer::new(sample_rate as f32);
        let mut eq = EqChain::new(sample_rate as f32);

        let mut is_playing = false;
        let mut volume = 0.72_f32;
        let mut muted = false;
        let mut sleep_fade = 1.0_f32;
        let mut local_buf: Option<Arc<Mutex<VecDeque<f32>>>> = None;
        let mut smoothed_gain = 0.72_f32;

        let err_fn = |err| eprintln!("Audio stream error: {err}");

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        AudioCmd::SetPlaying(p) => is_playing = p,
                        AudioCmd::SelectStation(id) => {
                            local_buf = None;
                            synth.select_station(&id);
                        }
                        AudioCmd::AttachLocalBuffer(b) => local_buf = b,
                        AudioCmd::SetVolume(v) => volume = v.clamp(0.0, 1.0),
                        AudioCmd::SetMuted(m) => muted = m,
                        AudioCmd::SetSleepFade(f) => sleep_fade = f.clamp(0.0, 1.0),
                        AudioCmd::SetEq(b) => eq.set_eq(b),
                        AudioCmd::SetTapeCutoff(hz) => eq.set_tape_cutoff(hz),
                        AudioCmd::SetAmbience(map) => ambience.set_map(&map),
                        AudioCmd::PlayChime => synth.trigger_chime(),
                    }
                }

                let target_gain = if muted { 0.0 } else { volume * sleep_fade };
                let use_local = local_buf.is_some();

                let mut local_guard = if is_playing && use_local {
                    local_buf.as_ref().and_then(|b| b.try_lock().ok())
                } else {
                    None
                };

                for frame in data.chunks_mut(channels.max(1)) {
                    smoothed_gain += (target_gain - smoothed_gain) * 0.004;

                    let (mut ml, mut mr) = if is_playing && use_local {
                        if let Some(ref mut q) = local_guard {
                            let l = q.pop_front().unwrap_or(0.0);
                            let r = q.pop_front().unwrap_or(l);
                            (l, r)
                        } else {
                            (0.0, 0.0)
                        }
                    } else {
                        synth.next_sample(is_playing && !use_local)
                    };

                    // Always mix Focus completion chimes from synth when local mode is active
                    if use_local {
                        let (cl, cr) = synth.next_sample(false);
                        ml += cl;
                        mr += cr;
                    }

                    let (eq_l, eq_r) = eq.process(ml, mr);
                    let (amb_l, amb_r) = if is_playing {
                        ambience.next_sample()
                    } else {
                        (0.0, 0.0)
                    };

                    let out_l = ((eq_l + amb_l) * smoothed_gain).clamp(-1.0, 1.0);
                    let out_r = ((eq_r + amb_r) * smoothed_gain).clamp(-1.0, 1.0);

                    if frame.len() >= 2 {
                        frame[0] = out_l;
                        frame[1] = out_r;
                        for extra in &mut frame[2..] {
                            *extra = (out_l + out_r) * 0.5;
                        }
                    } else if let Some(s) = frame.first_mut() {
                        *s = (out_l + out_r) * 0.5;
                    }
                }
            },
            err_fn,
            None,
        );

        match stream {
            Ok(s) => {
                let _ = s.play();
                Self {
                    tx,
                    _stream: Some(s),
                    sample_rate,
                    playing: false,
                    local_stream: None,
                    init_error: None,
                }
            }
            Err(e) => Self {
                tx,
                _stream: None,
                sample_rate,
                playing: false,
                local_stream: None,
                init_error: Some(format!("Failed building audio output stream: {e}")),
            },
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
        let _ = self.tx.send(AudioCmd::SetPlaying(playing));
    }

    pub fn toggle_play(&mut self) -> bool {
        self.set_playing(!self.playing);
        self.playing
    }

    pub fn select_station(&mut self, station_id: &str, auto_play: bool) {
        self.local_stream = None;
        let _ = self.tx.send(AudioCmd::SelectStation(station_id.to_string()));
        if auto_play {
            self.set_playing(true);
        }
    }

    pub fn play_local_file(&mut self, path: PathBuf, auto_play: bool) -> Result<(), String> {
        let stream = LocalAudioStream::open(path, self.sample_rate)?;
        let buf = stream.buffer();
        self.local_stream = Some(stream);
        let _ = self.tx.send(AudioCmd::AttachLocalBuffer(Some(buf)));
        if auto_play {
            self.set_playing(true);
        }
        Ok(())
    }

    pub fn seek_local(&self, secs: f32) {
        if let Some(s) = &self.local_stream {
            s.seek_seconds(secs);
        }
    }

    pub fn local_progress(&self) -> (f32, f32) {
        if let Some(s) = &self.local_stream {
            (s.position_secs(), s.duration_secs())
        } else {
            (0.0, 0.0)
        }
    }

    pub fn local_finished(&self) -> bool {
        self.playing
            && self
                .local_stream
                .as_ref()
                .map(|s| s.is_finished())
                .unwrap_or(false)
    }

    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(AudioCmd::SetVolume(v));
    }

    pub fn set_muted(&self, muted: bool) {
        let _ = self.tx.send(AudioCmd::SetMuted(muted));
    }

    pub fn set_sleep_fade(&self, mult: f32) {
        let _ = self.tx.send(AudioCmd::SetSleepFade(mult));
    }

    pub fn set_eq(&self, bands: [f32; 5]) {
        let _ = self.tx.send(AudioCmd::SetEq(bands));
    }

    pub fn set_tape_cutoff(&self, hz: f32) {
        let _ = self.tx.send(AudioCmd::SetTapeCutoff(hz));
    }

    pub fn set_ambience(&self, map: &HashMap<String, f32>) {
        let _ = self.tx.send(AudioCmd::SetAmbience(map.clone()));
    }

    pub fn play_chime(&self) {
        let _ = self.tx.send(AudioCmd::PlayChime);
    }
}
