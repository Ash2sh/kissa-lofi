use crate::models::{LofiStation, BUILTIN_STATIONS};
use std::f32::consts::TAU;

fn midi_to_hz(midi: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

#[derive(Clone, Copy, Default)]
struct RhodesVoice {
    active: bool,
    phase: f32,
    overtone_phase: f32,
    freq: f32,
    age: f32,
    duration: f32,
    amp: f32,
    pan: f32,
}

#[derive(Clone, Copy, Default)]
struct BellVoice {
    active: bool,
    phase: f32,
    freq: f32,
    age: f32,
    duration: f32,
    amp: f32,
}

pub struct LofiSynth {
    sample_rate: f32,
    station_idx: usize,
    step_index: usize,
    samples_until_step: f32,
    voices: [RhodesVoice; 16],
    bells: [BellVoice; 6],
    // Bass
    bass_phase: f32,
    bass_freq: f32,
    bass_age: f32,
    bass_dur: f32,
    bass_active: bool,
    // Kick
    kick_phase: f32,
    kick_age: f32,
    kick_active: bool,
    // Snare & Hat envelopes
    snare_age: f32,
    snare_active: bool,
    snare_lp: f32,
    hat_age: f32,
    hat_active: bool,
    hat_hp_prev: f32,
    // PRNG
    rng: u32,
}

impl LofiSynth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            station_idx: 0,
            step_index: 0,
            samples_until_step: 0.0,
            voices: [RhodesVoice::default(); 16],
            bells: [BellVoice::default(); 6],
            bass_phase: 0.0,
            bass_freq: 65.0,
            bass_age: 0.0,
            bass_dur: 1.0,
            bass_active: false,
            kick_phase: 0.0,
            kick_age: 0.0,
            kick_active: false,
            snare_age: 0.0,
            snare_active: false,
            snare_lp: 0.0,
            hat_age: 0.0,
            hat_active: false,
            hat_hp_prev: 0.0,
            rng: 0x1234_5678,
        }
    }

    pub fn select_station(&mut self, station_id: &str) {
        if let Some(idx) = BUILTIN_STATIONS.iter().position(|s| s.id == station_id) {
            self.station_idx = idx;
            self.step_index = 0;
            self.samples_until_step = 0.0;
        }
    }

    pub fn trigger_chime(&mut self) {
        for (i, &freq) in [523.25_f32, 659.25, 783.99].iter().enumerate() {
            if let Some(slot) = self.bells.get_mut(i) {
                *slot = BellVoice {
                    active: true,
                    phase: 0.0,
                    freq,
                    age: -(i as f32) * 0.13,
                    duration: 1.25,
                    amp: 0.14,
                };
            }
        }
    }

    #[inline]
    fn rand_f32(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) / (u32::MAX as f32)
    }

    fn station(&self) -> &'static LofiStation {
        &BUILTIN_STATIONS[self.station_idx % BUILTIN_STATIONS.len()]
    }

    fn trigger_step(&mut self) {
        let st = self.station();
        let step_in_bar = self.step_index % 8;
        let bar = (self.step_index / 8) % st.chords.len();
        let chord = st.chords[bar];
        let beat_sec = 60.0 / st.bpm;

        // 1. Warm Rhodes chords on beat 1 (and gentle echo on 3.5)
        if step_in_bar == 0 || (step_in_bar == 5 && self.rand_f32() < 0.35) {
            let dur = if step_in_bar == 0 {
                beat_sec * 3.7
            } else {
                beat_sec * 1.4
            };
            let amp = if step_in_bar == 0 { 0.085 } else { 0.042 };
            for (idx, &midi) in chord.iter().enumerate() {
                let detune = 1.0 + (self.rand_f32() - 0.5) * 0.0045;
                let pan = 0.35 + (idx as f32 / 4.0) * 0.30;
                if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
                    *v = RhodesVoice {
                        active: true,
                        phase: 0.0,
                        overtone_phase: 0.0,
                        freq: midi_to_hz(midi) * detune,
                        age: -(idx as f32) * 0.016,
                        duration: dur,
                        amp,
                        pan,
                    };
                }
            }
        }

        // 2. Warm sub-bass
        if step_in_bar == 0 || (step_in_bar == 6 && self.rand_f32() < 0.5) {
            self.bass_active = true;
            self.bass_phase = 0.0;
            self.bass_age = 0.0;
            self.bass_dur = if step_in_bar == 0 { 1.45 } else { 0.65 };
            self.bass_freq = midi_to_hz(chord[0].saturating_sub(12));
        }

        // 3. Soft boom-bap kick
        if step_in_bar == 0 || (step_in_bar == 5 && self.rand_f32() < 0.72) {
            self.kick_active = true;
            self.kick_phase = 0.0;
            self.kick_age = 0.0;
        }

        // 4. Dusty brush snare on 2 & 4
        if step_in_bar == 2 || step_in_bar == 6 {
            self.snare_active = true;
            self.snare_age = 0.0;
        }

        // 5. Whisper hi-hat
        if step_in_bar % 2 == 0 || self.rand_f32() < 0.72 {
            self.hat_active = true;
            self.hat_age = 0.0;
        }

        // 6. Occasional pentatonic bell note
        if (step_in_bar == 3 || step_in_bar == 7) && self.rand_f32() < 0.42 {
            let pick = (self.rand_f32() * chord.len() as f32) as usize % chord.len();
            let note = chord[pick] + 12;
            if let Some(b) = self.bells.iter_mut().find(|b| !b.active) {
                *b = BellVoice {
                    active: true,
                    phase: 0.0,
                    freq: midi_to_hz(note),
                    age: 0.0,
                    duration: 1.15,
                    amp: 0.05,
                };
            }
        }

        let swing = if step_in_bar % 2 == 1 { 0.022 } else { -0.011 };
        let eighth_sec = (beat_sec * 0.5 + swing).max(0.15);
        self.samples_until_step = eighth_sec * self.sample_rate;
        self.step_index = self.step_index.wrapping_add(1);
    }

    #[inline]
    pub fn next_sample(&mut self, playing_station: bool) -> (f32, f32) {
        let dt = 1.0 / self.sample_rate;

        if playing_station {
            self.samples_until_step -= 1.0;
            if self.samples_until_step <= 0.0 {
                self.trigger_step();
            }
        }

        let mut l = 0.0_f32;
        let mut r = 0.0_f32;

        // Rhodes electric piano voices
        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            v.age += dt;
            if v.age < 0.0 {
                continue;
            }
            if v.age >= v.duration {
                v.active = false;
                continue;
            }
            let env = if v.age < 0.04 {
                v.age / 0.04
            } else {
                (-2.2 * (v.age - 0.04) / v.duration).exp() * (1.0 - v.age / v.duration)
            };
            v.phase = (v.phase + v.freq * dt) % 1.0;
            v.overtone_phase = (v.overtone_phase + v.freq * 2.0 * dt) % 1.0;

            // Soft triangle + mellow decaying 2nd harmonic
            let tri = 4.0 * (v.phase - 0.5).abs() - 1.0;
            let overtone = (v.overtone_phase * TAU).sin() * (-6.0 * v.age).exp() * 0.22;
            let s = (tri * 0.78 + overtone) * env * v.amp;
            l += s * (1.0 - v.pan * 0.35);
            r += s * (0.65 + v.pan * 0.35);
        }

        // Bells & focus chimes (always render even when paused so focus chime plays!)
        for b in &mut self.bells {
            if !b.active {
                continue;
            }
            b.age += dt;
            if b.age < 0.0 {
                continue;
            }
            if b.age >= b.duration {
                b.active = false;
                continue;
            }
            let env = if b.age < 0.02 {
                b.age / 0.02
            } else {
                (-3.8 * b.age).exp()
            };
            b.phase = (b.phase + b.freq * dt) % 1.0;
            let s = (b.phase * TAU).sin() * env * b.amp;
            l += s;
            r += s;
        }

        if playing_station {
            // Sub-bass
            if self.bass_active {
                self.bass_age += dt;
                if self.bass_age >= self.bass_dur {
                    self.bass_active = false;
                } else {
                    let env = if self.bass_age < 0.035 {
                        self.bass_age / 0.035
                    } else {
                        (-2.4 * self.bass_age / self.bass_dur).exp()
                    };
                    self.bass_phase = (self.bass_phase + self.bass_freq * dt) % 1.0;
                    let s = (self.bass_phase * TAU).sin() * env * 0.21;
                    l += s;
                    r += s;
                }
            }

            // Soft kick
            if self.kick_active {
                self.kick_age += dt;
                if self.kick_age >= 0.25 {
                    self.kick_active = false;
                } else {
                    let freq = 42.0 + 75.0 * (-22.0 * self.kick_age).exp();
                    self.kick_phase = (self.kick_phase + freq * dt) % 1.0;
                    let env = (-12.5 * self.kick_age).exp();
                    let s = (self.kick_phase * TAU).sin() * env * 0.26;
                    l += s;
                    r += s;
                }
            }

            let white = self.rand_f32() * 2.0 - 1.0;

            // Dusty brush snare
            if self.snare_active {
                self.snare_age += dt;
                if self.snare_age >= 0.14 {
                    self.snare_active = false;
                } else {
                    self.snare_lp += 0.22 * (white - self.snare_lp);
                    let env = (-24.0 * self.snare_age).exp();
                    let s = self.snare_lp * env * 0.12;
                    l += s;
                    r += s;
                }
            }

            // Whisper hi-hat
            if self.hat_active {
                self.hat_age += dt;
                if self.hat_age >= 0.045 {
                    self.hat_active = false;
                } else {
                    let hp = white - self.hat_hp_prev;
                    self.hat_hp_prev = white;
                    let env = (-65.0 * self.hat_age).exp();
                    let s = hp * env * 0.018;
                    l += s;
                    r += s;
                }
            }
        }

        (l * 0.82, r * 0.82)
    }
}
