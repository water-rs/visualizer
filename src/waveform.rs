//! The time-domain oscilloscope visualizer.

use kurbo::Rect;
use waterui_core::{
    Computed, Environment, IntoSignalF32, Signal as _, SignalExt as _,
    reactive::{signal::IntoComputed, watcher::BoxWatcherGuard},
    view::View,
};
use waterui_graphics::{Scene2D, SceneInvalidator, SceneView, color::Color};

use crate::analysis::SampleSmoothing;
use crate::geometry::waveform_path;
use crate::scene::{Drawing, VisualizerScene, draw_glow, stroke_path};
use crate::source::SampleSource;
use crate::style::{
    ReactiveStyle, ResolvedStyle, StyleOverrides, VisualizerTheme, invalidate_on_change,
    style_methods,
};

/// Share of the surface height a full-scale sample deflects the trace by.
const FULL_SCALE_DEFLECTION: f64 = 0.4;

/// How far each new window moves the drawn trace toward it.
const TRACE_SMOOTHING: f32 = 0.3;

/// A real-time waveform oscilloscope visualizer.
///
/// The trace is a Catmull-Rom curve through the sample window, stroked through
/// the engine-neutral scene contract, so the same view draws on the GPU
/// renderer, the CPU sparse-strip renderer, and any backend that owns its own
/// scene.
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

    /// The scene content this view draws.
    ///
    /// A view returns this from its body; taking it directly is what lets a
    /// caller compose the trace into a scene of its own, or rasterize it
    /// offscreen.
    pub fn into_scene(self, env: &Environment) -> SceneView {
        let Self {
            source,
            theme,
            sensitivity,
            style,
        } = self;
        SceneView::new(VisualizerScene::new(
            source,
            ReactiveStyle::new(&theme, style, env),
            WaveformDrawing {
                sensitivity,
                smoothing: SampleSmoothing::new(TRACE_SMOOTHING),
            },
        ))
    }
}

impl<S: SampleSource> View for Waveform<S> {
    fn body(self, env: &Environment) -> impl View {
        self.into_scene(env)
    }
}

/// Convenience constructor for a [`Waveform`].
pub fn waveform<S: SampleSource>(source: S) -> Waveform<S> {
    Waveform::new(source)
}

/// Draws the smoothed sample window as one stroked curve.
#[derive(Debug)]
struct WaveformDrawing {
    sensitivity: Computed<f64>,
    smoothing: SampleSmoothing,
}

impl Drawing for WaveformDrawing {
    fn draw(
        &mut self,
        scene: &mut dyn Scene2D,
        samples: &[f32],
        style: &ResolvedStyle,
        area: Rect,
    ) {
        let amplitude = FULL_SCALE_DEFLECTION * self.sensitivity.get();
        let trace = waveform_path(self.smoothing.apply(samples), area, amplitude);
        draw_glow(scene, &trace, style);
        stroke_path(scene, &trace, style, style.line);
    }

    fn install(&mut self, invalidator: &SceneInvalidator) -> Vec<BoxWatcherGuard> {
        vec![invalidate_on_change(&self.sensitivity, invalidator)]
    }
}
