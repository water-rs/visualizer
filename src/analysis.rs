//! Time- and frequency-domain analysis shared by the visualizers.
//!
//! Nothing here knows how anything is drawn: an analyzer turns one window of
//! [`Samples`](crate::Samples) into the numbers a visualizer's geometry is a
//! function of, and the geometry module turns those into curves.

use std::sync::Arc;

use realfft::{RealFftPlanner, RealToComplex, num_complex::Complex32};

/// Level, in decibels, that a band has to reach to leave the floor.
///
/// Everything quieter than this maps to zero, which is what keeps the noise
/// floor of a live microphone from lifting every bar off the baseline.
const FLOOR_DB: f32 = 60.0;

/// Exponential smoothing applied across successive sample windows.
///
/// Raw capture jitters between windows; smoothing the samples rather than the
/// drawn curve keeps the visualizer's geometry a pure function of what the
/// analyzer produced.
#[derive(Debug)]
pub struct SampleSmoothing {
    smoothed: Vec<f32>,
    factor: f32,
}

impl SampleSmoothing {
    /// Creates a smoother that moves `factor` of the way to each new window.
    ///
    /// `factor` is clamped to `0.0..=1.0`; `1.0` passes windows through
    /// untouched.
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self {
            smoothed: Vec::new(),
            factor: factor.clamp(0.0, 1.0),
        }
    }

    /// Smooths `samples` against the windows seen before it.
    ///
    /// The first window of a given length is taken as it is, so a one-shot
    /// render shows the signal it was given rather than a fraction of it.
    pub fn apply(&mut self, samples: &[f32]) -> &[f32] {
        if self.smoothed.len() == samples.len() {
            for (smoothed, sample) in self.smoothed.iter_mut().zip(samples) {
                *smoothed = (*sample - *smoothed).mul_add(self.factor, *smoothed);
            }
        } else {
            self.smoothed.clear();
            self.smoothed.extend_from_slice(samples);
        }
        &self.smoothed
    }
}

/// Turns a window of samples into logarithmically spaced magnitude bands.
///
/// Bands are normalized to `0.0..=1.0`, where `1.0` is a full-scale sine in
/// that band and `0.0` is [`FLOOR_DB`] below it. Successive windows decay into
/// each other rather than being averaged, so a transient reaches its true
/// height on the frame it happens and falls back over the following ones.
pub struct SpectrumAnalyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    window_gain: f32,
    input: Vec<f32>,
    spectrum: Vec<Complex32>,
    bands: Vec<f32>,
    decay: f32,
}

impl core::fmt::Debug for SpectrumAnalyzer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SpectrumAnalyzer")
            .field("window_len", &self.window.len())
            .field("bands", &self.bands.len())
            .finish_non_exhaustive()
    }
}

impl SpectrumAnalyzer {
    /// Creates an analyzer producing `bands` bands from `window_len` samples.
    ///
    /// # Panics
    ///
    /// Panics when `bands` is zero or `window_len` is smaller than four, which
    /// leaves no usable spectrum to divide up.
    #[must_use]
    pub fn new(window_len: usize, bands: usize) -> Self {
        assert!(bands > 0, "a spectrum needs at least one band");
        assert!(
            window_len >= 4,
            "a spectrum needs at least four samples per window"
        );
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(window_len);
        let window = hann(window_len);
        let window_gain = window.iter().sum();
        Self {
            input: vec![0.0; window_len],
            spectrum: fft.make_output_vec(),
            fft,
            window,
            window_gain,
            bands: vec![0.0; bands],
            decay: 0.82,
        }
    }

    /// The number of bands this analyzer produces.
    #[must_use]
    pub const fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// Analyzes `samples`, returning the current band levels.
    ///
    /// A window whose length differs from the planned one replans the
    /// transform, so a source that changes its window size stays supported.
    ///
    /// # Panics
    ///
    /// Panics for a window shorter than four samples, which leaves no usable
    /// spectrum to divide up.
    pub fn analyze(&mut self, samples: &[f32]) -> &[f32] {
        if samples.len() != self.window.len() {
            self.replan(samples.len());
        }
        for ((input, sample), window) in self.input.iter_mut().zip(samples).zip(&self.window) {
            *input = *sample * *window;
        }
        self.fft
            .process(&mut self.input, &mut self.spectrum)
            .expect("the analyzer's buffers are sized by its own plan");

        // Bin 0 is DC and carries no pitch, so the bands start at bin 1.
        let usable = self.spectrum.len() - 1;
        let band_count = self.bands.len();
        for index in 0..band_count {
            let start = band_edge(usable, band_count, index);
            let end = band_edge(usable, band_count, index + 1).max(start + 1);
            let peak = self.spectrum[start..end.min(self.spectrum.len())]
                .iter()
                .map(|bin| bin.norm())
                .fold(0.0_f32, f32::max);
            let level = to_level(2.0 * peak / self.window_gain);
            let band = &mut self.bands[index];
            *band = level.max(*band * self.decay);
        }
        &self.bands
    }

    fn replan(&mut self, window_len: usize) {
        assert!(
            window_len >= 4,
            "a spectrum needs at least four samples per window"
        );
        self.fft = RealFftPlanner::<f32>::new().plan_fft_forward(window_len);
        self.window = hann(window_len);
        self.window_gain = self.window.iter().sum();
        self.input = vec![0.0; window_len];
        self.spectrum = self.fft.make_output_vec();
    }
}

/// The Hann window of `len` coefficients.
fn hann(len: usize) -> Vec<f32> {
    let last = index_to_f32(len.saturating_sub(1)).max(1.0);
    (0..len)
        .map(|index| {
            let phase = core::f32::consts::TAU * index_to_f32(index) / last;
            0.5 * (1.0 - phase.cos())
        })
        .collect()
}

/// The first bin of band `index`, spaced logarithmically over `usable` bins.
///
/// Pitch is logarithmic, so equal-width bands would spend most of their width
/// on the top octave and crush everything a listener hears as "the bass" into
/// the first bar. Edge zero is bin 1 — bin 0 is DC — and edge `band_count` is
/// one past the last usable bin, so the edges bound every band as a half-open
/// range.
///
/// Logarithmic spacing alone would put the first several edges on the same bin,
/// because a transform's bins are spaced linearly and the bottom of the
/// spectrum is where the log curve is flattest. Each edge is therefore forced
/// at least one bin past the one before it, which makes the lowest bands one
/// bin wide — a bar per bin down there and log-spaced groups higher up, rather
/// than a row of identical bars all reading bin one.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the exponential is bounded by `usable + 1`, so the truncation is in range and never negative"
)]
fn band_edge(usable: usize, band_count: usize, index: usize) -> usize {
    let fraction = index_to_f32(index) / index_to_f32(band_count);
    let logarithmic = index_to_f32(usable + 1).powf(fraction) as usize;
    logarithmic.max(index + 1).clamp(1, usable + 1)
}

/// Maps a normalized magnitude onto `0.0..=1.0` across [`FLOOR_DB`].
fn to_level(magnitude: f32) -> f32 {
    if magnitude <= 0.0 {
        return 0.0;
    }
    let decibels = 20.0 * magnitude.log10();
    ((decibels + FLOOR_DB) / FLOOR_DB).clamp(0.0, 1.0)
}

/// Converts a sample, bin or band index into the float it is used as.
#[expect(
    clippy::cast_precision_loss,
    reason = "window, bin and band indices are far below f32's exact-integer limit"
)]
const fn index_to_f32(index: usize) -> f32 {
    index as f32
}

#[cfg(test)]
mod tests {
    use super::{SampleSmoothing, SpectrumAnalyzer, index_to_f32};

    /// A sine at `cycles` periods over the window, at full scale.
    fn sine(len: usize, cycles: f32) -> Vec<f32> {
        (0..len)
            .map(|index| {
                (core::f32::consts::TAU * cycles * index_to_f32(index) / index_to_f32(len)).sin()
            })
            .collect()
    }

    #[test]
    fn the_first_window_passes_smoothing_untouched() {
        let mut smoothing = SampleSmoothing::new(0.3);
        assert_eq!(smoothing.apply(&[1.0, -1.0]), [1.0, -1.0]);
    }

    #[test]
    fn smoothing_moves_toward_later_windows() {
        let mut smoothing = SampleSmoothing::new(0.5);
        smoothing.apply(&[0.0, 0.0]);
        assert_eq!(smoothing.apply(&[1.0, -1.0]), [0.5, -0.5]);
    }

    #[test]
    fn a_tone_lights_one_band_and_leaves_the_rest_down() {
        let mut analyzer = SpectrumAnalyzer::new(1024, 16);
        let bands = analyzer.analyze(&sine(1024, 64.0)).to_vec();
        let loudest = bands
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .expect("the analyzer produces bands");
        assert!(
            *loudest.1 > 0.5,
            "a full-scale tone should drive its band well off the floor, got {}",
            loudest.1
        );
        let others = bands
            .iter()
            .enumerate()
            .filter(|(index, _)| index.abs_diff(loudest.0) > 1)
            .fold(0.0_f32, |peak, (_, band)| peak.max(*band));
        assert!(
            others < *loudest.1 * 0.5,
            "a pure tone should not spread across the spectrum, got {others}"
        );
    }

    #[test]
    fn silence_stays_on_the_floor() {
        let mut analyzer = SpectrumAnalyzer::new(256, 8);
        assert!(
            analyzer
                .analyze(&[0.0; 256])
                .iter()
                .all(|band| *band == 0.0)
        );
    }

    #[test]
    fn a_transient_decays_rather_than_vanishing() {
        let mut analyzer = SpectrumAnalyzer::new(512, 8);
        let peak = analyzer
            .analyze(&sine(512, 32.0))
            .iter()
            .fold(0.0_f32, |peak, band| peak.max(*band));
        let after = analyzer
            .analyze(&[0.0; 512])
            .iter()
            .fold(0.0_f32, |peak, band| peak.max(*band));
        assert!(after > 0.0 && after < peak, "expected decay, got {after}");
    }
}
