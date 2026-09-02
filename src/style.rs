//! Colors and stroke widths shared by every visualizer.

use waterui_core::{
    Computed, Environment, Signal as _, SignalExt as _, flatten_signal, impl_constant,
    reactive::{signal::IntoComputed, watcher::BoxWatcherGuard},
};
use waterui_graphics::{
    SceneInvalidator,
    color::{Color, ResolvedColor},
};

/// Theme configuration shared by every visualizer.
///
/// A theme names the four things a visualizer paints with — a background, the
/// ink its geometry is drawn in, the glow around that ink, and how wide and how
/// strong those two are. Which shape they end up on is the visualizer's
/// business, not the theme's.
#[derive(Debug, Clone)]
pub struct VisualizerTheme {
    /// Background color.
    pub(crate) background: Color,
    /// Color of the drawn geometry.
    pub(crate) line: Color,
    /// Color of the halo around the drawn geometry.
    pub(crate) glow: Color,
    /// Stroke width in points.
    pub(crate) line_width: f32,
    /// Glow intensity, `0.0` to `1.0`.
    pub(crate) glow_intensity: f32,
}

impl_constant!(VisualizerTheme);

impl Default for VisualizerTheme {
    fn default() -> Self {
        Self::cyber()
    }
}

impl VisualizerTheme {
    /// Creates a reusable visualizer theme.
    #[must_use]
    pub fn new(
        background: impl Into<Color>,
        line: impl Into<Color>,
        glow: impl Into<Color>,
        line_width: f32,
        glow_intensity: f32,
    ) -> Self {
        Self {
            background: background.into(),
            line: line.into(),
            glow: glow.into(),
            line_width,
            glow_intensity,
        }
    }

    /// Replaces the background color.
    #[must_use]
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.background = color.into();
        self
    }

    /// Replaces the color the geometry is drawn in.
    #[must_use]
    pub fn line(mut self, color: impl Into<Color>) -> Self {
        self.line = color.into();
        self
    }

    /// Replaces the glow color.
    #[must_use]
    pub fn glow_color(mut self, color: impl Into<Color>) -> Self {
        self.glow = color.into();
        self
    }

    /// Replaces the stroke width in points.
    #[must_use]
    pub const fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Replaces the glow intensity.
    #[must_use]
    pub const fn glow(mut self, intensity: f32) -> Self {
        self.glow_intensity = intensity;
        self
    }

    /// Cyberpunk-style theme with cyan glow.
    #[must_use]
    pub fn cyber() -> Self {
        Self::new(
            Color::srgb_f32(0.05, 0.05, 0.1),
            Color::srgb_f32(0.0, 1.0, 0.8),
            Color::srgb_f32(0.0, 0.8, 1.0),
            2.0,
            0.6,
        )
    }

    /// Voice recorder style with red bars.
    #[must_use]
    pub fn recorder() -> Self {
        Self::new(
            Color::srgb_f32(0.05, 0.05, 0.05),
            Color::srgb_f32(1.0, 0.2, 0.2),
            Color::srgb_f32(1.0, 0.1, 0.1),
            3.0,
            0.3,
        )
    }

    /// Minimal green oscilloscope.
    #[must_use]
    pub fn oscilloscope() -> Self {
        Self::new(
            Color::srgb_f32(0.0, 0.02, 0.0),
            Color::srgb_f32(0.0, 1.0, 0.0),
            Color::srgb_f32(0.0, 0.8, 0.0),
            1.5,
            0.5,
        )
    }
}

/// Per-view overrides of the theme; `None` keeps whatever the theme says.
#[derive(Debug, Clone, Default)]
pub struct StyleOverrides {
    pub background: Option<Computed<Color>>,
    pub line: Option<Computed<Color>>,
    pub glow: Option<Computed<Color>>,
    pub line_width: Option<Computed<f32>>,
    pub glow_intensity: Option<Computed<f32>>,
}

/// A visualizer's style, resolved against an environment and still reactive.
///
/// Every field stays a signal for the drawing's lifetime, so a theme or color
/// change repaints the surface without the view being rebuilt.
pub struct ReactiveStyle {
    background: Computed<ResolvedColor>,
    line: Computed<ResolvedColor>,
    glow: Computed<ResolvedColor>,
    line_width: Computed<f32>,
    glow_intensity: Computed<f32>,
    guards: Vec<BoxWatcherGuard>,
}

impl core::fmt::Debug for ReactiveStyle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReactiveStyle")
            .field("installed", &!self.guards.is_empty())
            .finish_non_exhaustive()
    }
}

impl ReactiveStyle {
    /// Resolves `theme` against `env`, with `overrides` taking precedence.
    pub fn new(
        theme: &Computed<VisualizerTheme>,
        overrides: StyleOverrides,
        env: &Environment,
    ) -> Self {
        let background = overrides
            .background
            .unwrap_or_else(|| theme.map(|theme| theme.background).computed());
        let line = overrides
            .line
            .unwrap_or_else(|| theme.map(|theme| theme.line).computed());
        let glow = overrides
            .glow
            .unwrap_or_else(|| theme.map(|theme| theme.glow).computed());
        Self {
            background: resolve_color(background, env),
            line: resolve_color(line, env),
            glow: resolve_color(glow, env),
            line_width: overrides
                .line_width
                .unwrap_or_else(|| theme.map(|theme| theme.line_width).computed()),
            glow_intensity: overrides
                .glow_intensity
                .unwrap_or_else(|| theme.map(|theme| theme.glow_intensity).computed()),
            guards: Vec::new(),
        }
    }

    /// Repaints the surface whenever any style signal changes.
    pub fn install(&mut self, invalidator: &SceneInvalidator) {
        self.guards = vec![
            invalidate_on_change(&self.background, invalidator),
            invalidate_on_change(&self.line, invalidator),
            invalidate_on_change(&self.glow, invalidator),
            invalidate_on_change(&self.line_width, invalidator),
            invalidate_on_change(&self.glow_intensity, invalidator),
        ];
    }

    /// Stops watching every style signal.
    pub fn uninstall(&mut self) {
        self.guards.clear();
    }

    /// Reads every style signal for one frame.
    pub fn resolve(&self) -> ResolvedStyle {
        ResolvedStyle {
            background: to_peniko(&self.background.get()),
            line: to_peniko(&self.line.get()),
            glow: to_peniko(&self.glow.get()),
            line_width: f64::from(self.line_width.get().max(0.0)),
            glow_intensity: self.glow_intensity.get().clamp(0.0, 1.0),
        }
    }
}

/// One frame's worth of style values.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedStyle {
    pub background: peniko::Color,
    pub line: peniko::Color,
    pub glow: peniko::Color,
    pub line_width: f64,
    pub glow_intensity: f32,
}

/// Requests a repaint whenever `signal` changes.
pub fn invalidate_on_change<T: Clone + 'static>(
    signal: &Computed<T>,
    invalidator: &SceneInvalidator,
) -> BoxWatcherGuard {
    let invalidator = SceneInvalidator::clone(invalidator);
    signal.watch(move |_| invalidator())
}

/// Resolves `color` against `env`, keeping the result reactive.
fn resolve_color(color: impl IntoComputed<Color>, env: &Environment) -> Computed<ResolvedColor> {
    let env = env.clone();
    flatten_signal(color.into_computed().map(move |color| color.resolve(&env)))
}

/// Converts a resolved linear-RGB color into the sRGB-encoded color peniko takes.
fn to_peniko(color: &ResolvedColor) -> peniko::Color {
    let srgb = color.to_srgb_with_headroom();
    peniko::Color::new([
        srgb.red,
        srgb.green,
        srgb.blue,
        color.opacity.clamp(0.0, 1.0),
    ])
}

/// Emits the style builder methods every visualizer view shares.
///
/// `ink` names what the visualizer draws, so each view's documentation talks
/// about its own geometry while the methods behind them stay one definition.
macro_rules! style_methods {
    (ink = $ink:literal) => {
        /// Sets the visual theme.
        pub fn theme(mut self, theme: impl IntoComputed<VisualizerTheme>) -> Self {
            self.theme = theme.into_computed();
            self
        }

        /// Sets the sensitivity, the multiplier applied to the analyzed level.
        pub fn sensitivity(mut self, sensitivity: impl IntoComputed<f64>) -> Self {
            self.sensitivity = sensitivity.into_computed();
            self
        }

        /// Sets the background color.
        pub fn bg_color(mut self, color: impl IntoComputed<Color>) -> Self {
            self.style.background = Some(color.into_computed());
            self
        }

        #[doc = concat!("Sets the color of ", $ink, ".")]
        pub fn line_color(mut self, color: impl IntoComputed<Color>) -> Self {
            self.style.line = Some(color.into_computed());
            self
        }

        #[doc = concat!("Sets the color of the glow around ", $ink, ".")]
        pub fn glow_color(mut self, color: impl IntoComputed<Color>) -> Self {
            self.style.glow = Some(color.into_computed());
            self
        }

        #[doc = concat!("Sets the stroke width of ", $ink, ", in points.")]
        pub fn line_width(mut self, width: impl IntoSignalF32 + 'static) -> Self {
            self.style.line_width = Some(width.into_signal_f32().computed());
            self
        }

        /// Sets the glow intensity, from `0.0` for none to `1.0` for full.
        pub fn glow(mut self, intensity: impl IntoSignalF32 + 'static) -> Self {
            self.style.glow_intensity = Some(intensity.into_signal_f32().computed());
            self
        }
    };
}

pub(crate) use style_methods;
