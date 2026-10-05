//! The scene content every visualizer shares.
//!
//! One [`SceneContent`] serves all three visualizers: it owns the sample
//! signal, the resolved style and the invalidation wiring, and delegates the
//! only part that differs — the geometry — to a [`Drawing`].

use kurbo::{BezPath, Cap, Join, Rect, Stroke};
use waterui_core::{Computed, Signal as _, reactive::watcher::BoxWatcherGuard};
use waterui_graphics::draw::{Draw as _, Fixed, Recorder, WorkingColor};
use waterui_graphics::{RecordingResources, SceneContent, SceneInvalidator, invalidate_on_change};

use crate::geometry::surface_rect;
use crate::source::{SampleSource, Samples};
use crate::style::{ReactiveStyle, ResolvedStyle};

/// How many halo passes make up a glow.
///
/// A glow is a stack of progressively wider, fainter strokes of the same path.
/// Three is where another pass stops being visible against the one under it.
const GLOW_LAYERS: u16 = 3;

/// How much wider each halo pass is than the stroke it surrounds.
const GLOW_SPREAD: f64 = 3.0;

/// Opacity of the innermost halo pass at full glow intensity.
const GLOW_OPACITY: f32 = 0.35;

/// How a visualizer turns one analyzed window into scene commands.
pub trait Drawing: 'static {
    /// Records one window of `samples` inside `area`.
    fn draw(&mut self, recorder: &mut Recorder, samples: &[f32], style: &ResolvedStyle, area: Rect);

    /// Repaints the surface whenever one of this drawing's own inputs changes.
    fn install(&mut self, invalidator: &SceneInvalidator) -> Vec<BoxWatcherGuard>;
}

/// Scene content drawing `D` from the sample windows `S` publishes.
///
/// The samples are a signal, so a new window repaints the surface precisely
/// rather than through a rebuild of the view, and a silent visualizer costs no
/// frames at all.
pub struct VisualizerScene<S, D> {
    source: S,
    samples: Option<Computed<Samples>>,
    style: ReactiveStyle,
    drawing: D,
    guards: Vec<BoxWatcherGuard>,
}

impl<S, D: core::fmt::Debug> core::fmt::Debug for VisualizerScene<S, D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VisualizerScene")
            .field("style", &self.style)
            .field("drawing", &self.drawing)
            .finish_non_exhaustive()
    }
}

impl<S: SampleSource, D: Drawing> VisualizerScene<S, D> {
    /// Creates content drawing `drawing` from `source`, painted with `style`.
    pub const fn new(source: S, style: ReactiveStyle, drawing: D) -> Self {
        Self {
            source,
            samples: None,
            style,
            drawing,
            guards: Vec::new(),
        }
    }

    /// The sample signal, subscribing to the source the first time it is asked
    /// for. Subscribing is what opens a capture session, so it happens when
    /// drawing starts rather than when the view is built.
    fn samples(&mut self) -> &Computed<Samples> {
        if self.samples.is_none() {
            self.samples = Some(self.source.subscribe());
        }
        self.samples
            .as_ref()
            .expect("the sample signal was just subscribed to")
    }
}

impl<S: SampleSource, D: Drawing> SceneContent for VisualizerScene<S, D> {
    fn build_scene(
        &mut self,
        recorder: &mut Recorder,
        _resources: &mut RecordingResources<'_>,
        width: f32,
        height: f32,
    ) -> bool {
        let Some(area) = surface_rect(width, height) else {
            return false;
        };
        let style = self.style.resolve();
        fill_rect(recorder, area, style.background);
        let samples = self.samples().snapshot();
        self.drawing.draw(recorder, &samples, &style, area);
        false
    }

    /// A visualizer registers no engine resources — the recording names only
    /// colors and paths — so a replacement engine has nothing to rebuild.
    fn rebuild_for_engine(&mut self) {}

    fn set_invalidator(&mut self, invalidator: Option<SceneInvalidator>) {
        self.guards.clear();
        self.style.uninstall();
        let Some(invalidator) = invalidator else {
            return;
        };
        self.style.install(&invalidator);
        let samples = self.samples().clone();
        self.guards = self.drawing.install(&invalidator);
        self.guards
            .push(invalidate_on_change(&invalidator, &samples));
    }
}

/// Fills `rect` with `color`.
pub fn fill_rect(recorder: &mut Recorder, rect: Rect, color: WorkingColor) {
    recorder.fill(rect, Fixed(color));
}

/// The stroke a visualizer's ink is drawn with.
const fn ink_stroke(width: f64) -> Stroke {
    Stroke::new(width)
        .with_caps(Cap::Round)
        .with_join(Join::Round)
}

/// Draws the halo around `path`, widest and faintest pass first.
///
/// This is what the old fragment shader's exponential falloff becomes in vector
/// terms: the same path, stroked a few times at growing widths and shrinking
/// opacity, which any scene engine can draw without a shader of its own.
pub fn draw_glow(recorder: &mut Recorder, path: &BezPath, style: &ResolvedStyle) {
    if style.glow_intensity <= 0.0 || path.is_empty() {
        return;
    }
    for layer in (1..=GLOW_LAYERS).rev() {
        let spread = GLOW_SPREAD * f64::from(layer);
        let width = style.line_width.mul_add(spread, style.line_width);
        let alpha = style.glow_intensity * GLOW_OPACITY / f32::from(layer);
        recorder.stroke(
            path.clone(),
            ink_stroke(width),
            Fixed(style.glow.with_alpha(style.glow.components[3] * alpha)),
        );
    }
}

/// Strokes `path` in `color` at the style's stroke width.
pub fn stroke_path(
    recorder: &mut Recorder,
    path: &BezPath,
    style: &ResolvedStyle,
    color: WorkingColor,
) {
    if path.is_empty() {
        return;
    }
    recorder.stroke(path.clone(), ink_stroke(style.line_width), Fixed(color));
}

/// Fills `path` with `color`.
pub fn fill_path(recorder: &mut Recorder, path: &BezPath, color: WorkingColor) {
    if path.is_empty() {
        return;
    }
    recorder.fill(path.clone(), Fixed(color));
}
