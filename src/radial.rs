//! The circular spectrum visualizer.

use std::sync::Arc;

use kurbo::BezPath;
use waterui_core::{
    Computed, Environment, IntoSignalF32, SignalExt as _, reactive::signal::IntoComputed,
    view::View,
};
use waterui_graphics::{canvas::Canvas, color::Color};

use crate::analysis::SpectrumAnalyzer;
use crate::audio::SAMPLES_COUNT;
use crate::canvas::{
    derive_path, draw_background, draw_glow, draw_interior, draw_outline, subscribe,
};
use crate::geometry::{radial_path, surface_rect};
use crate::source::{SampleSource, Samples};
use crate::style::{Style, StyleOverrides, VisualizerTheme, style_methods};

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
/// as one path on a canvas. The first band sits at
/// twelve o'clock and the outline is smooth across the seam.
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

    /// The canvas this view draws.
    ///
    /// A view returns this from its body; taking it directly is what lets a
    /// caller compose the ring into a canvas of its own.
    ///
    /// The canvas records once, when it mounts. The ring path is a live
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
            inner_radius,
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
                .zip(&inner_radius);
            let ring = derive_path(
                &spectrum.zip(&layout),
                |(bands, ((area, amplitude), inner))| {
                    area.map_or_else(BezPath::new, |area| {
                        radial_path(&bands, area, f64::from(inner), amplitude)
                    })
                },
            );
            draw_background(cx, &style);
            draw_glow(cx, &style, &ring);
            draw_interior(cx, &style, &ring, INTERIOR_OPACITY);
            draw_outline(cx, &style, &ring);
        })
    }
}

impl<S: SampleSource> View for RadialSpectrum<S> {
    fn body(self, env: &Environment) -> impl View {
        self.into_canvas(env)
    }
}

/// Convenience constructor for a [`RadialSpectrum`].
pub fn radial_spectrum<S: SampleSource>(source: S) -> RadialSpectrum<S> {
    RadialSpectrum::new(source)
}
