use std::f32::consts::PI;

pub const EQ_BAND_FREQS: [f32; 5] = [60.0, 230.0, 910.0, 3600.0, 12000.0];
pub const EQ_BAND_LABELS: [&str; 5] = ["60Hz", "230Hz", "910Hz", "3.6k", "12k"];

#[derive(Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct StereoBiquad {
    c: BiquadCoeffs,
    z1_l: f32,
    z2_l: f32,
    z1_r: f32,
    z2_r: f32,
}

impl StereoBiquad {
    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let out_l = self.c.b0 * l + self.z1_l;
        self.z1_l = self.c.b1 * l - self.c.a1 * out_l + self.z2_l;
        self.z2_l = self.c.b2 * l - self.c.a2 * out_l;

        let out_r = self.c.b0 * r + self.z1_r;
        self.z1_r = self.c.b1 * r - self.c.a1 * out_r + self.z2_r;
        self.z2_r = self.c.b2 * r - self.c.a2 * out_r;

        (out_l, out_r)
    }

    fn set_peaking(&mut self, sample_rate: f32, freq: f32, q: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (freq.min(sample_rate * 0.45) / sample_rate);
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        self.c = BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
    }

    fn set_lowshelf(&mut self, sample_rate: f32, freq: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (freq.min(sample_rate * 0.45) / sample_rate);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / 0.9 - 1.0) + 2.0).max(0.01).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        self.c = BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
    }

    fn set_highshelf(&mut self, sample_rate: f32, freq: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (freq.min(sample_rate * 0.45) / sample_rate);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / 0.9 - 1.0) + 2.0).max(0.01).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        self.c = BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
    }

    fn set_lowpass(&mut self, sample_rate: f32, freq: f32, q: f32) {
        let w0 = 2.0 * PI * (freq.clamp(200.0, sample_rate * 0.46) / sample_rate);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.c = BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
    }
}

pub struct EqChain {
    sample_rate: f32,
    bands: [StereoBiquad; 5],
    tape_lp: StereoBiquad,
}

impl EqChain {
    pub fn new(sample_rate: f32) -> Self {
        let mut chain = Self {
            sample_rate,
            bands: [StereoBiquad::default(); 5],
            tape_lp: StereoBiquad::default(),
        };
        chain.set_eq([0.0; 5]);
        chain.set_tape_cutoff(18500.0);
        chain
    }

    pub fn set_eq(&mut self, gains_db: [f32; 5]) {
        self.bands[0].set_lowshelf(self.sample_rate, EQ_BAND_FREQS[0], gains_db[0]);
        self.bands[1].set_peaking(self.sample_rate, EQ_BAND_FREQS[1], 1.05, gains_db[1]);
        self.bands[2].set_peaking(self.sample_rate, EQ_BAND_FREQS[2], 1.05, gains_db[2]);
        self.bands[3].set_peaking(self.sample_rate, EQ_BAND_FREQS[3], 1.05, gains_db[3]);
        self.bands[4].set_highshelf(self.sample_rate, EQ_BAND_FREQS[4], gains_db[4]);
    }

    pub fn set_tape_cutoff(&mut self, hz: f32) {
        self.tape_lp.set_lowpass(self.sample_rate, hz, 0.72);
    }

    #[inline]
    pub fn process(&mut self, mut l: f32, mut r: f32) -> (f32, f32) {
        for band in &mut self.bands {
            (l, r) = band.process(l, r);
        }
        (l, r) = self.tape_lp.process(l, r);
        // Gentle analog tape soft-clipper
        ((l * 1.05).tanh() * 0.95, (r * 1.05).tanh() * 0.95)
    }
}
