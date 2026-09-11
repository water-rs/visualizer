//! The frequency-bar spectrum visualizer.

use kurbo::{BezPath, Rect, Shape as _};
use waterui_core::{
    Computed, Environment, IntoSignalF32, Signal as _, SignalExt as _,
    reactive::{signal::IntoComputed, watcher::BoxWatcherGuard},
    view::View,
};
use waterui_graphics::{Scene2D, SceneInvalidator, SceneView, color::Color};

use crate::analysis::SpectrumAnalyzer;
use crate::audio::SAMPLES_COUNT;
use crate::geometry::bar_rects;
use crate::scene::{Drawing, VisualizerScene, draw_glow, fill_path};
use crate::source::SampleSource;
use crate::style::{
    ReactiveStyle, ResolvedStyle, StyleOverrides, VisualizerTheme, invalidate_on_change,
    style_methods,
};

/// How many bands a spectrum is divided into unless the view says otherwise.
const DEFAULT_BANDS: usize = 32;

/// Share of each band's slot left empty between neighbouring bars.
const DEFAULT_GAP: f32 = 0.3;

/// Share of half a bar's width used as its corner radius.
const DEFAULT_CORNER: f32 = 1.0;

/// Accuracy, in points, that rounded bars are flattened to.
const BAR_TOLERANCE: f64 = 0.05;

/// A real-time frequency-bar spectrum visualizer.
///
/// Each band becomes one rounded rectangle growing from the baseline, filled
/// through the engine-neutral scene contract.
///
/// # Example
///
/// ```rust
/// use waterui_graphics::color::Color;
/// use waterui_visualizer::{AudioCapture, SpectrumBars};
///
/// let _bars = SpectrumBars::new(AudioCapture::new())
///     .bands(24)
///     .line_color(Color::cyan())
///     .gap(0.25);
/// ```
#[derive(Clone, Debug)]
#[must_use = "a `SpectrumBars` does nothing unless it is rendered as part of a view"]
pub struct SpectrumBars<S> {
    source: S,
    theme: Computed<VisualizerTheme>,
    sensitivity: Computed<f64>,
    style: StyleOverrides,
    bands: usize,
    gap: Computed<f32>,
    corner: Computed<f32>,
}

impl<S: SampleSource> SpectrumBars<S> {
    /// Creates a spectrum drawing the windows `source` publishes.
    pub fn new(source: S) -> Self {
        Self {
            source,
            theme: Computed::constant(VisualizerTheme::default()),
            sensitivity: Computed::constant(1.0),
            style: StyleOverrides::default(),
            bands: DEFAULT_BANDS,
            gap: Computed::constant(DEFAULT_GAP),
            corner: Computed::constant(DEFAULT_CORNER),
        }
    }

    /// Sets how many bands the spectrum is divided into.
    ///
    /// # Panics
    ///
    /// Panics when `bands` is zero: a spectrum with no bands has nothing to
    /// analyze and nothing to draw.
    pub fn bands(mut self, bands: usize) -> Self {
        assert!(bands > 0, "a spectrum needs at least one band");
        self.bands = bands;
        self
    }

    /// Sets the share of each band's slot left empty between bars.
    pub fn gap(mut self, gap: impl IntoSignalF32 + 'static) -> Self {
        self.gap = gap.into_signal_f32().computed();
        self
    }

    /// Sets the corner radius of a bar, as a share of half its width.
    pub fn corner_radius(mut self, radius: impl IntoSignalF32 + 'static) -> Self {
        self.corner = radius.into_signal_f32().computed();
        self
    }

    style_methods!(ink = "the bars");

    /// The scene content this view draws.
    ///
    /// A view returns this from its body; taking it directly is what lets a
    /// caller compose the bars into a scene of its own, or rasterize them
    /// offscreen.
    pub fn into_scene(self, env: &Environment) -> SceneView {
        let Self {
            source,
            theme,
            sensitivity,
            style,
            bands,
            gap,
            corner,
        } = self;
        SceneView::new(VisualizerScene::new(
            source,
            ReactiveStyle::new(&theme, style, env),
            BarsDrawing {
                analyzer: SpectrumAnalyzer::new(SAMPLES_COUNT, bands),
                sensitivity,
                gap,
                corner,
            },
        ))
    }
}

impl<S: SampleSource> View for SpectrumBars<S> {
    fn body(self, env: &Environment) -> impl View {
        self.into_scene(env)
    }
}

/// Convenience constructor for a [`SpectrumBars`].
pub fn spectrum_bars<S: SampleSource>(source: S) -> SpectrumBars<S> {
    SpectrumBars::new(source)
}

/// Draws the analyzed spectrum as one filled path of rounded bars.
#[derive(Debug)]
struct BarsDrawing {
    analyzer: SpectrumAnalyzer,
    sensitivity: Computed<f64>,
    gap: Computed<f32>,
    corner: Computed<f32>,
}

impl Drawing for BarsDrawing {
    fn draw(
        &mut self,
        scene: &mut dyn Scene2D,
        samples: &[f32],
        style: &ResolvedStyle,
        area: Rect,
    ) {
        let amplitude = self.sensitivity.get();
        let gap = f64::from(self.gap.get());
        let corner = f64::from(self.corner.get());
        let mut bars = BezPath::new();
        for bar in bar_rects(self.analyzer.analyze(samples), area, amplitude, gap, corner) {
            bars.extend(bar.path_elements(BAR_TOLERANCE));
        }
        draw_glow(scene, &bars, style);
        fill_path(scene, &bars, style.line);
    }

    fn install(&mut self, invalidator: &SceneInvalidator) -> Vec<BoxWatcherGuard> {
        vec![
            invalidate_on_change(&self.sensitivity, invalidator),
            invalidate_on_change(&self.gap, invalidator),
            invalidate_on_change(&self.corner, invalidator),
        ]
    }
}
