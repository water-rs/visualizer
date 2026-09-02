//! Renders every visualizer from a deterministic synthetic signal.
//!
//! These are export tests: they drive each visualizer's scene content through
//! an offscreen surface and write the result out to be looked at. What the
//! geometry has to *be* is asserted in the library's own unit tests, where a
//! curve can be checked as a curve instead of as pixels.

use std::path::{Path, PathBuf};

use waterui_core::{Computed, Environment};
use waterui_graphics::color::{Color, Srgb};
use waterui_graphics::{GpuRuntime, OffscreenRenderConfig, OffscreenSize, SceneView, wgpu};
use waterui_visualizer::{
    RadialSpectrum, SAMPLES_COUNT, Samples, SpectrumBars, VisualizerTheme, Waveform,
};

/// Where the exported PNGs are written, shared with the other components'
/// scene-export tests.
const OUTPUT_DIRECTORY: &str = "/tmp/waterui_scene_engines";

/// A sine sweep rising from `start` to `end` cycles across the window.
///
/// A sweep is the honest test signal for a trace: its amplitude is constant, so
/// anything the drawing does to the envelope shows up, and its period changes
/// across the window, so a curve that resampled wrongly stops looking like a
/// sweep.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "sample indices and a sine's value are both well inside the float ranges used"
)]
fn sweep(len: usize, start: f64, end: f64) -> Samples {
    let last = (len - 1) as f64;
    (0..len)
        .map(|index| {
            let position = index as f64 / last;
            // Integrating a linear frequency ramp over the window gives the
            // phase of a chirp.
            let cycles = start.mul_add(position, (end - start) * position * position / 2.0);
            (core::f64::consts::TAU * cycles).sin() as f32
        })
        .collect::<Vec<f32>>()
        .into()
}

/// A sum of tones whose amplitudes differ, giving a spectrum whose peaks are
/// plainly of different heights.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "sample indices and a bounded sum of sines are both well inside the float ranges used"
)]
fn chord(len: usize, tones: &[(f64, f64)]) -> Samples {
    let last = (len - 1) as f64;
    (0..len)
        .map(|index| {
            let position = index as f64 / last;
            let value: f64 = tones
                .iter()
                .map(|(cycles, gain)| gain * (core::f64::consts::TAU * cycles * position).sin())
                .sum();
            value.clamp(-1.0, 1.0) as f32
        })
        .collect::<Vec<f32>>()
        .into()
}

/// The tones both spectrum visualizers are driven with.
fn spectrum_signal() -> Samples {
    chord(
        SAMPLES_COUNT,
        &[
            (3.0, 0.50),
            (11.0, 0.30),
            (37.0, 0.18),
            (96.0, 0.10),
            (240.0, 0.05),
        ],
    )
}

/// Rasterizes `scene` at `width` x `height` and writes it out as `name`.
fn export(scene: SceneView, name: &str, width: u32, height: u32) -> PathBuf {
    let directory = Path::new(OUTPUT_DIRECTORY);
    std::fs::create_dir_all(directory).expect("output directory must be creatable");
    let runtime = pollster::block_on(GpuRuntime::new())
        .expect("the visualizer scene export requires a working GPU runtime");
    let size = OffscreenSize::try_from_pixels(width, height).expect("test size must be valid");
    let config = OffscreenRenderConfig::new(size).format(wgpu::TextureFormat::Rgba8Unorm);
    let mut env = Environment::new();
    let output = pollster::block_on(
        scene
            .into_gpu_surface()
            .render_offscreen(&runtime, config, &mut env),
    )
    .expect("offscreen render should succeed");
    let path = directory.join(name);
    output.save_png(&path).expect("png should be written");
    path
}

#[test]
fn the_waveform_draws_a_sine_sweep() {
    let env = Environment::new();
    let scene = Waveform::new(Computed::constant(sweep(SAMPLES_COUNT, 2.0, 24.0)))
        .theme(VisualizerTheme::oscilloscope())
        .line_width(2.0)
        .into_scene(&env);
    export(scene, "visualizer_waveform.png", 480, 240);
}

#[test]
fn the_spectrum_draws_bars_of_differing_height() {
    let env = Environment::new();
    let scene = SpectrumBars::new(Computed::constant(spectrum_signal()))
        .bands(24)
        .into_scene(&env);
    export(scene, "visualizer_bars.png", 480, 240);
}

#[test]
fn the_radial_spectrum_draws_a_closed_ring() {
    let env = Environment::new();
    let scene = RadialSpectrum::new(Computed::constant(spectrum_signal()))
        .bands(48)
        .inner_radius(0.4)
        .line_color(Color::from(Srgb::new(1.0, 0.45, 0.15)))
        .glow_color(Color::from(Srgb::new(1.0, 0.2, 0.0)))
        .glow(0.9)
        .line_width(3.0)
        .into_scene(&env);
    export(scene, "visualizer_radial.png", 360, 360);
}
