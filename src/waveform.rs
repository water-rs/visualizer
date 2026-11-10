//! The time-domain oscilloscope visualizer.

use std::sync::Arc;

use kurbo::BezPath;
use waterui_core::{
    Computed, Environment, IntoSignalF32, SignalExt as _, reactive::signal::IntoComputed,
    view::View,
};
use waterui_graphics::{canvas::Canvas, color::Color};

use crate::analysis::SampleSmoothing;
use crate::canvas::{derive_path, draw_background, draw_glow, draw_outline, subscribe};
use crate::geometry::{surface_rect, waveform_path};
use crate::source::{SampleSource, Samples};
use crate::style::{Style, StyleOverrides, VisualizerTheme, style_methods};

/// Share of the surface height a full-scale sample deflects the trace by.
const FULL_SCALE_DEFLECTION: f64 = 0.4;

/// How far each new window moves the drawn trace toward it.
const TRACE_SMOOTHING: f32 = 0.3;

/// A real-time waveform oscilloscope visualizer.
///
/// The trace is a Catmull-Rom curve through the sample window, drawn on a
/// canvas, so the same view is realized by whichever renderer draws the screen
/// around it.
///
/// # Example
///
/// ```rust
/// use waterui_core::Binding;
/// use waterui_graphics::color::Color;
/// use waterui_visualizer::{AudioCapture, Waveform};
///
/// let capture = AudioCapture::new();
/// let _waveform = Waveform::new(capture)
///     .line_color(Color::cyan())
///     .bg_color(Color::srgb(0, 0, 0))
///     .glow(0.8)
///     .sensitivity(Binding::f64(1.5));
/// ```
#[derive(Clone, Debug)]
#[must_use = "a `Waveform` does nothing unless it is rendered as part of a view"]
pub struct Waveform<S> {
    source: S,
    theme: Computed<VisualizerTheme>,
    sensitivity: Computed<f64>,
    style: StyleOverrides,
}

impl<S: SampleSource> Waveform<S> {
    /// Creates a waveform drawing the windows `source` publishes.
    pub fn new(source: S) -> Self {
        Self {
            source,
            theme: Computed::constant(VisualizerTheme::default()),
            sensitivity: Computed::constant(1.0),
            style: StyleOverrides::default(),
        }
    }

    style_methods!(ink = "the trace");

    /// The canvas this view draws.
    ///
    /// A view returns this from its body; taking it directly is what lets a
    /// caller compose the trace into a canvas of its own.
    ///
    /// The canvas records once, when it mounts. The trace is a live operand
    /// derived from the sample signal, once per window, and the style signals
    /// are live operands of the commands that paint with them, so nothing
    /// records again as audio arrives.
    pub fn into_canvas(self, env: &Environment) -> Canvas {
        let Self {
            source,
            theme,
            sensitivity,
            style,
        } = self;
        let style = Style::new(&theme, style, env);
        Canvas::new(move |cx| {
            let samples = subscribe(cx, source);
            let smoothed = samples.scan(
                SampleSmoothing::new(TRACE_SMOOTHING),
                |smoothing, samples: Samples| -> Arc<[f32]> { smoothing.apply(&samples).into() },
            );
            let layout = cx
                .size()
                .map(|size| surface_rect(size.width, size.height))
                .zip(&sensitivity);
            let trace = derive_path(&smoothed.zip(&layout), |(samples, (area, sensitivity))| {
                area.map_or_else(BezPath::new, |area| {
                    waveform_path(&samples, area, FULL_SCALE_DEFLECTION * sensitivity)
                })
            });
            // The path signal only listens to the smoothed window while the
            // smoothing is alive.
            cx.keep(smoothed);
            draw_background(cx, &style);
            draw_glow(cx, &style, &trace);
            draw_outline(cx, &style, &trace);
        })
    }
}

impl<S: SampleSource> View for Waveform<S> {
    fn body(self, env: &Environment) -> impl View {
        self.into_canvas(env)
    }
}

/// Convenience constructor for a [`Waveform`].
pub fn waveform<S: SampleSource>(source: S) -> Waveform<S> {
    Waveform::new(source)
}
