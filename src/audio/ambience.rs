use std::collections::HashMap;
use std::f32::consts::TAU;

pub struct AmbientMixer {
    sample_rate: f32,
    levels: [f32; 8], // rain, vinyl, fireplace, thunder, wind, cafe, forest, ocean
    rng: u32,
    pink: [f32; 3],
    brown: f32,
    rain_lp: f32,
    rain_hp: f32,
    wind_lp: f32,
    cafe_lp: f32,
    ocean_lp: f32,
    lfo_phase: f32,
    thunder_env: f32,
    thunder_timer: f32,
    cricket_phase: f32,
    cricket_timer: f32,
    crackle_env: f32,
}

impl AmbientMixer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            levels: [0.0; 8],
            rng: 0x9E37_79B9,
            pink: [0.0; 3],
            brown: 0.0,
            rain_lp: 0.0,
            rain_hp: 0.0,
            wind_lp: 0.0,
            cafe_lp: 0.0,
            ocean_lp: 0.0,
            lfo_phase: 0.0,
            thunder_env: 0.0,
            thunder_timer: 2.5,
            cricket_phase: 0.0,
            cricket_timer: 0.0,
            crackle_env: 0.0,
        }
    }

    pub fn set_map(&mut self, map: &HashMap<String, f32>) {
        let keys = [
            "rain",
            "vinyl",
            "fireplace",
            "thunder",
            "wind",
            "cafe",
            "forest",
            "ocean",
        ];
        for (i, k) in keys.iter().enumerate() {
            self.levels[i] = map.get(*k).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        }
    }

    pub fn any_active(&self) -> bool {
        self.levels.iter().any(|&v| v > 0.005)
    }

    #[inline]
    fn white(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) / (u32::MAX as f32) * 2.0 - 1.0
    }

    #[inline]
    pub fn next_sample(&mut self) -> (f32, f32) {
        if !self.any_active() {
            return (0.0, 0.0);
        }

        let dt = 1.0 / self.sample_rate;
        let w = self.white();
        let w2 = self.white();

        // Fast 3-pole pink noise approximation + brown noise
        self.pink[0] = 0.997 * self.pink[0] + w * 0.029;
        self.pink[1] = 0.985 * self.pink[1] + w * 0.032;
        self.pink[2] = 0.950 * self.pink[2] + w * 0.048;
        let pink = (self.pink[0] + self.pink[1] + self.pink[2] + w * 0.018) * 1.4;
        self.brown = (self.brown + 0.022 * w) / 1.022;
        let brown = self.brown * 2.6;

        self.lfo_phase = (self.lfo_phase + dt * 0.12) % 1.0;
        let lfo = 0.5 + 0.5 * (self.lfo_phase * TAU).sin();

        let mut l = 0.0_f32;
        let mut r = 0.0_f32;

        // 0: Rain (warm filtered pink + light stereo sheen)
        if self.levels[0] > 0.005 {
            self.rain_lp += 0.18 * (pink - self.rain_lp);
            self.rain_hp += 0.02 * (self.rain_lp - self.rain_hp);
            let rain_body = self.rain_lp - self.rain_hp;
            let amp = self.levels[0] * 0.58;
            l += (rain_body + w * 0.012) * amp;
            r += (rain_body + w2 * 0.012) * amp;
        }

        // 1: Vinyl crackle
        if self.levels[1] > 0.005 {
            let pop = if w > 0.9992 { w2 * 0.55 } else { 0.0 };
            let hiss = w * 0.012;
            let s = (pop + hiss) * self.levels[1] * 0.7;
            l += s;
            r += s * 0.95;
        }

        // 2: Fireplace (deep warm hearth rumble + ember snaps)
        if self.levels[2] > 0.005 {
            if w > 0.9994 {
                self.crackle_env = 0.55;
            }
            self.crackle_env *= 0.985;
            let s = (brown * 0.25 + w2 * self.crackle_env) * self.levels[2] * 0.65;
            l += s;
            r += s;
        }

        // 3: Distant Thunder
        if self.levels[3] > 0.005 {
            self.thunder_timer -= dt;
            if self.thunder_timer <= 0.0 {
                self.thunder_env = 0.9;
                self.thunder_timer = 6.5 + (w.abs() * 7.0);
            }
            self.thunder_env *= 0.99993;
            let s = brown * self.thunder_env * self.levels[3] * 0.85;
            l += s;
            r += s;
        }

        // 4: Night Wind
        if self.levels[4] > 0.005 {
            let cutoff = 0.04 + lfo * 0.09;
            self.wind_lp += cutoff * (pink - self.wind_lp);
            let s = self.wind_lp * (0.45 + 0.55 * lfo) * self.levels[4] * 0.55;
            l += s;
            r += s * 0.9;
        }

        // 5: Cafe Murmur
        if self.levels[5] > 0.005 {
            self.cafe_lp += 0.08 * (brown - self.cafe_lp);
            let s = self.cafe_lp * self.levels[5] * 0.5;
            l += s;
            r += s;
        }

        // 6: Forest Crickets
        if self.levels[6] > 0.005 {
            self.cricket_timer = (self.cricket_timer + dt) % 1.4;
            let chirp =
                if self.cricket_timer < 0.22 && ((self.cricket_timer * 28.0) as i32) % 2 == 0 {
                    self.cricket_phase = (self.cricket_phase + 4320.0 * dt) % 1.0;
                    (self.cricket_phase * TAU).sin() * 0.045
                } else {
                    0.0
                };
            let s = (pink * 0.08 + chirp) * self.levels[6] * 0.7;
            l += s;
            r += s;
        }

        // 7: Ocean Waves
        if self.levels[7] > 0.005 {
            self.ocean_lp += 0.07 * (brown - self.ocean_lp);
            let swell = 0.25 + 0.75 * lfo;
            let s = self.ocean_lp * swell * self.levels[7] * 0.6;
            l += s;
            r += s;
        }

        (l * 0.75, r * 0.75)
    }
}
