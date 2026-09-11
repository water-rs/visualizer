//! The circular spectrum visualizer.

use kurbo::Rect;
use waterui_core::{
    Computed, Environment, IntoSignalF32, Signal as _, SignalExt as _,
    reactive::{signal::IntoComputed, watcher::BoxWatcherGuard},
    view::View,
};
use waterui_graphics::{Scene2D, SceneInvalidator, SceneView, color::Color};

use crate::analysis::SpectrumAnalyzer;
use crate::audio::SAMPLES_COUNT;
use crate::geometry::radial_path;
use crate::scene::{Drawing, VisualizerScene, draw_glow, fill_path, stroke_path};
use crate::source::SampleSource;
use crate::style::{
    ReactiveStyle, ResolvedStyle, StyleOverrides, VisualizerTheme, invalidate_on_change,
    style_methods,
};

/// How many bands the ring is divided into unless the view says otherwise.
///
/// A ring reads as a shape rather than as a bar chart, so it wants enough
/// bands for the outline to curve smoothly between them.
const DEFAULT_BANDS: usize = 64;

/// Share of the available radius the ring rests at when every band is silent.
const DEFAULT_INNER_RADIUS: f32 = 0.45;

/// Opacity of the ring's interior relative to its outline.
const INTERIOR_OPACITY: f32 = 0.35;

/// A real-time circular spectrum visualizer.
///
/// The spectrum is wrapped around a ring: each band pushes the outline out from
/// a resting radius, and the resulting closed curve is filled and stroked
/// through the engine-neutral scene contract. The first band sits at twelve
/// o'clock and the outline is smooth across the seam.
///
/// # Example
///
/// ```rust
/// use waterui_graphics::color::Color;
/// use waterui_visualizer::{AudioCapture, RadialSpectrum};
///
/// let _ring = RadialSpectrum::new(AudioCapture::new())
///     .bands(48)
///     .inner_radius(0.5)
///     .line_color(Color::cyan());
/// ```
#[derive(Clone, Debug)]
#[must_use = "a `RadialSpectrum` does nothing unless it is rendered as part of a view"]
pub struct RadialSpectrum<S> {
    source: S,
    theme: Computed<VisualizerTheme>,
    sensitivity: Computed<f64>,
    style: StyleOverrides,
    bands: usize,
    inner_radius: Computed<f32>,
}

impl<S: SampleSource> RadialSpectrum<S> {
    /// Creates a ring drawing the windows `source` publishes.
    pub fn new(source: S) -> Self {
        Self {
            source,
            theme: Computed::constant(VisualizerTheme::default()),
            sensitivity: Computed::constant(1.0),
            style: StyleOverrides::default(),
            bands: DEFAULT_BANDS,
            inner_radius: Computed::constant(DEFAULT_INNER_RADIUS),
        }
    }

    /// Sets how many bands the ring is divided into.
    ///
    /// # Panics
    ///
    /// Panics for fewer than three bands, which cannot enclose an area.
    pub fn bands(mut self, bands: usize) -> Self {
        assert!(bands >= 3, "a radial spectrum needs at least three bands");
        self.bands = bands;
        self
    }

    /// Sets the resting radius, as a share of the largest circle that fits.
    pub fn inner_radius(mut self, radius: impl IntoSignalF32 + 'static) -> Self {
        self.inner_radius = radius.into_signal_f32().computed();
        self
    }

    style_methods!(ink = "the ring");

    /// The scene content this view draws.
    ///
    /// A view returns this from its body; taking it directly is what lets a
    /// caller compose the ring into a scene of its own, or rasterize it
    /// offscreen.
    pub fn into_scene(self, env: &Environment) -> SceneView {
        let Self {
            source,
            theme,
            sensitivity,
            style,
            bands,
            inner_radius,
        } = self;
        SceneView::new(VisualizerScene::new(
            source,
            ReactiveStyle::new(&theme, style, env),
            RadialDrawing {
                analyzer: SpectrumAnalyzer::new(SAMPLES_COUNT, bands),
                sensitivity,
                inner_radius,
            },
        ))
    }
}

impl<S: SampleSource> View for RadialSpectrum<S> {
    fn body(self, env: &Environment) -> impl View {
        self.into_scene(env)
    }
}

/// Convenience constructor for a [`RadialSpectrum`].
pub fn radial_spectrum<S: SampleSource>(source: S) -> RadialSpectrum<S> {
    RadialSpectrum::new(source)
}

/// Draws the analyzed spectrum as one closed, filled and stroked ring.
#[derive(Debug)]
struct RadialDrawing {
    analyzer: SpectrumAnalyzer,
    sensitivity: Computed<f64>,
    inner_radius: Computed<f32>,
}

impl Drawing for RadialDrawing {
    fn draw(
        &mut self,
        scene: &mut dyn Scene2D,
        samples: &[f32],
        style: &ResolvedStyle,
        area: Rect,
    ) {
        let amplitude = self.sensitivity.get();
        let inner = f64::from(self.inner_radius.get());
        let ring = radial_path(self.analyzer.analyze(samples), area, inner, amplitude);
        draw_glow(scene, &ring, style);
        fill_path(scene, &ring, style.line.multiply_alpha(INTERIOR_OPACITY));
        stroke_path(scene, &ring, style, style.line);
    }

    fn install(&mut self, invalidator: &SceneInvalidator) -> Vec<BoxWatcherGuard> {
        vec![
            invalidate_on_change(&self.sensitivity, invalidator),
            invalidate_on_change(&self.inner_radius, invalidator),
        ]
    }
}
