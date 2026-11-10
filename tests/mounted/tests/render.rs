//! Renders every visualizer from a deterministic synthetic signal through the
//! offscreen Hydrolysis renderer and writes the frames out to be looked at.
//!
//! What the geometry has to *be* is asserted in the library's own unit tests,
//! where a curve can be checked as a curve instead of as pixels. What these
//! tests own is the canvas: a visualizer records once, when it mounts, and the
//! audio windows and style changes that follow only move live operands.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use hydrolysis_m3::Material3;
use waterui::graphics::color::{Color, Srgb};
use waterui_core::{Binding, Computed, SignalExt as _};
use waterui_testing::{OffscreenApp, Snapshot, ui};
use waterui_visualizer::{
    RadialSpectrum, SAMPLES_COUNT, SampleSource, Samples, SpectrumBars, VisualizerTheme, Waveform,
    silence,
};

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

/// Successive audio windows: each one's tones differ from the last, so every
/// window draws a plainly different spectrum.
fn windows() -> [Samples; 4] {
    [
        chord(SAMPLES_COUNT, &[(4.0, 0.60), (20.0, 0.20)]),
        chord(SAMPLES_COUNT, &[(8.0, 0.30), (48.0, 0.50), (120.0, 0.20)]),
        chord(SAMPLES_COUNT, &[(2.0, 0.80), (72.0, 0.10)]),
        chord(SAMPLES_COUNT, &[(16.0, 0.45), (32.0, 0.45), (180.0, 0.15)]),
    ]
}

/// Where the exported PNGs are written.
fn output(file: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("visualizer");
    std::fs::create_dir_all(&directory).expect("output directory must be creatable");
    directory.join(file)
}

fn mount<V: waterui::View + 'static>(
    width: u16,
    height: u16,
    view: impl Fn() -> V + 'static,
) -> OffscreenApp {
    ui().theme(Material3::defaults())
        .viewport(u32::from(width), u32::from(height))
        .mount_offscreen(view)
}

fn save(snapshot: &Snapshot, file: &str) {
    snapshot
        .save_png(output(file))
        .expect("the png must be writable");
}

#[test]
fn the_waveform_draws_a_sine_sweep() {
    let mut app = mount(480, 240, || {
        Waveform::new(Computed::constant(sweep(SAMPLES_COUNT, 2.0, 24.0)))
            .theme(VisualizerTheme::oscilloscope())
            .line_width(2.0)
    });
    save(&app.snapshot(), "visualizer_waveform.png");
}

#[test]
fn the_spectrum_draws_bars_of_differing_height() {
    let mut app = mount(480, 240, || {
        SpectrumBars::new(Computed::constant(spectrum_signal())).bands(24)
    });
    save(&app.snapshot(), "visualizer_bars.png");
}

#[test]
fn the_radial_spectrum_draws_a_closed_ring() {
    let mut app = mount(360, 360, || {
        RadialSpectrum::new(Computed::constant(spectrum_signal()))
            .bands(48)
            .inner_radius(0.4)
            .line_color(Color::from(Srgb::new(1.0, 0.45, 0.15)))
            .glow_color(Color::from(Srgb::new(1.0, 0.2, 0.0)))
            .glow(0.9)
            .line_width(3.0)
    });
    save(&app.snapshot(), "visualizer_radial.png");
}

/// A source over a window binding that counts how often a canvas subscribes.
///
/// A canvas subscribes when its closure runs, and the closure is the only code
/// that records, so the count is how many times the canvas recorded.
#[derive(Clone)]
struct CountingSource {
    window: Binding<Samples>,
    subscriptions: Rc<Cell<u32>>,
}

impl SampleSource for CountingSource {
    fn subscribe(&self) -> Computed<Samples> {
        self.subscriptions.set(self.subscriptions.get() + 1);
        self.window.computed()
    }
}

/// Mounts `view` over a counting source, feeds it every window of
/// [`windows`], and returns the frames and the subscription count after each.
fn feed_windows<V: waterui::View + 'static>(
    name: &str,
    width: u16,
    height: u16,
    view: impl Fn(CountingSource) -> V + 'static,
) {
    let window = Binding::container(silence(SAMPLES_COUNT));
    let subscriptions = Rc::new(Cell::new(0));
    let mut app = {
        let (window, subscriptions) = (window.clone(), Rc::clone(&subscriptions));
        mount(width, height, move || {
            view(CountingSource {
                window: window.clone(),
                subscriptions: Rc::clone(&subscriptions),
            })
        })
    };
    let mut previous = app.snapshot();
    save(&previous, &format!("{name}_window0.png"));
    assert_eq!(subscriptions.get(), 1, "the canvas records at mount");

    for (index, samples) in windows().into_iter().enumerate() {
        window.set(samples);
        let frame = app.snapshot();
        assert_ne!(
            frame.rgba8(),
            previous.rgba8(),
            "window {} must change what is drawn",
            index + 1
        );
        assert_eq!(
            subscriptions.get(),
            1,
            "window {} must update the live operand, not record the canvas again",
            index + 1
        );
        save(&frame, &format!("{name}_window{}.png", index + 1));
        previous = frame;
    }

    // The recording keeps the source for as long as it is mounted, and
    // unmounting drops it: nothing is left holding the capture open.
    assert!(Rc::strong_count(&subscriptions) > 1);
    drop(app);
    assert_eq!(Rc::strong_count(&subscriptions), 1);
}

#[test]
fn the_bars_follow_successive_windows_without_recording_again() {
    feed_windows("windows_bars", 480, 240, |source| {
        SpectrumBars::new(source).bands(24)
    });
}

#[test]
fn the_ring_follows_successive_windows_without_recording_again() {
    feed_windows("windows_ring", 360, 360, |source| {
        RadialSpectrum::new(source).bands(48).glow(0.9)
    });
}

#[test]
fn the_waveform_follows_successive_windows_without_recording_again() {
    feed_windows("windows_waveform", 480, 240, |source| {
        Waveform::new(source).theme(VisualizerTheme::oscilloscope())
    });
}

/// The style signals are live operands: a new color or width repaints the
/// canvas in place, and neither records it again.
#[test]
fn style_changes_repaint_without_recording_again() {
    let line = Binding::container(Color::from(Srgb::new(0.0, 1.0, 0.8)));
    let width = Binding::container(2.0_f32);
    let glow = Binding::container(0.0_f32);
    let subscriptions = Rc::new(Cell::new(0));
    let mut app = {
        let (line, width, glow, subscriptions) = (
            line.clone(),
            width.clone(),
            glow.clone(),
            Rc::clone(&subscriptions),
        );
        mount(480, 240, move || {
            Waveform::new(CountingSource {
                window: Binding::container(sweep(SAMPLES_COUNT, 2.0, 24.0)),
                subscriptions: Rc::clone(&subscriptions),
            })
            .line_color(line.clone())
            .line_width(width.clone())
            .glow(glow.clone())
        })
    };
    let mut previous = app.snapshot();
    save(&previous, "style_initial.png");

    let mut step = |name: &str, change: &dyn Fn()| {
        change();
        let frame = app.snapshot();
        assert_ne!(
            frame.rgba8(),
            previous.rgba8(),
            "{name} must change what is drawn"
        );
        assert_eq!(
            subscriptions.get(),
            1,
            "{name} must update a live operand, not record the canvas again"
        );
        save(&frame, &format!("style_{name}.png"));
        previous = frame;
    };
    step("color", &|| {
        line.set(Color::from(Srgb::new(1.0, 0.3, 0.2)));
    });
    step("width", &|| width.set(7.0));
    // Crossing zero is the one change that records, and only the halo's region.
    step("glow", &|| glow.set(0.9));
}
