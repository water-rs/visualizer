//! Real-time audio visualization components for `WaterUI`.
//!
//! Every visualizer is a map from a sample signal to vector geometry, drawn on a
//! `waterui-graphics` canvas. The canvas records once, when it mounts: the
//! geometry is a live operand derived from the sample signal, once per audio
//! window, and the style signals are live operands too, so nothing records
//! again as sound arrives. Nothing here owns a GPU device, a pipeline or a
//! shader, so the same views are realized by whichever renderer draws the
//! screen around them.
//!
//! # Views
//!
//! - [`Waveform`] — a time-domain oscilloscope trace, as a smooth curve
//! - [`SpectrumBars`] — the frequency spectrum, as rounded bars
//! - [`RadialSpectrum`] — the frequency spectrum, wrapped around a ring
//!
//! # Sources
//!
//! A visualizer draws whatever [`SampleSource`] it is given. [`AudioCapture`]
//! is the microphone; a [`Computed<Samples>`](Samples) is anything else — a
//! decoded file, a test fixture, a synthesized signal.
//!
//! ```rust
//! use waterui_visualizer::{AudioCapture, SpectrumBars, Waveform};
//!
//! let capture = AudioCapture::new();
//! let _trace = Waveform::new(capture.clone()).sensitivity(1.5).glow(0.8);
//! let _spectrum = SpectrumBars::new(capture).bands(24);
//! ```

pub mod analysis;
pub mod geometry;

mod audio;
mod bars;
mod canvas;
mod radial;
mod source;
mod style;
mod waveform;

pub use analysis::{SampleSmoothing, SpectrumAnalyzer};
pub use audio::{AudioCapture, SAMPLES_COUNT};
pub use bars::{SpectrumBars, spectrum_bars};
pub use radial::{RadialSpectrum, radial_spectrum};
pub use source::{SampleSource, Samples, silence};
pub use style::VisualizerTheme;
pub use waveform::{Waveform, waveform};
