//! The frequency-bar spectrum visualizer.

use std::sync::Arc;

use kurbo::{BezPath, Shape as _};
use waterui_core::{
    Computed, Environment, IntoSignalF32, SignalExt as _, reactive::signal::IntoComputed,
    view::View,
};
use waterui_graphics::{canvas::Canvas, color::Color};

use crate::analysis::SpectrumAnalyzer;
use crate::audio::SAMPLES_COUNT;
use crate::canvas::{derive_path, draw_background, draw_glow, subscribe};
use crate::geometry::{bar_rects, surface_rect};
use crate::source::{SampleSource, Samples};
use crate::style::{Style, StyleOverrides, VisualizerTheme, style_methods};

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
/// as one path on a canvas.
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

    /// The canvas this view draws.
    ///
    /// A view returns this from its body; taking it directly is what lets a
    /// caller compose the bars into a canvas of its own.
    ///
    /// The canvas records once, when it mounts. The bar path is a live
    /// operand derived from the sample signal, once per window, and the style
    /// signals are live operands of the commands that paint with them, so
    /// nothing records again as audio arrives.
    pub fn into_canvas(self, env: &Environment) -> Canvas {
        let Self {
            source,
            theme,
            sensitivity,
            style,
            bands,
            gap,
            corner,
        } = self;
        let style = Style::new(&theme, style, env);
        Canvas::new(move |cx| {
            let samples = subscribe(cx, source);
            let spectrum = samples.scan(
                SpectrumAnalyzer::new(SAMPLES_COUNT, bands),
                |analyzer, samples: Samples| -> Arc<[f32]> { analyzer.analyze(&samples).into() },
            );
            let layout = cx
                .size()
                .map(|size| surface_rect(size.width, size.height))
                .zip(&sensitivity)
                .zip(&gap)
                .zip(&corner);
            let bars = derive_path(
                &spectrum.zip(&layout),
                |(bands, (((area, amplitude), gap), corner))| {
                    let mut path = BezPath::new();
                    if let Some(area) = area {
                        for bar in
                            bar_rects(&bands, area, amplitude, f64::from(gap), f64::from(corner))
                        {
                            path.extend(bar.path_elements(BAR_TOLERANCE));
                        }
                    }
                    path
                },
            );
            // The path signal only listens to the spectrum while the
            // spectrum is alive.
            cx.keep(spectrum);
            draw_background(cx, &style);
            draw_glow(cx, &style, &bars);
            cx.fill(bars, style.line().clone());
        })
    }
}

impl<S: SampleSource> View for SpectrumBars<S> {
    fn body(self, env: &Environment) -> impl View {
        self.into_canvas(env)
    }
}

/// Convenience constructor for a [`SpectrumBars`].
pub fn spectrum_bars<S: SampleSource>(source: S) -> SpectrumBars<S> {
    SpectrumBars::new(source)
}
