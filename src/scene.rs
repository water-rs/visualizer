//! The scene content every visualizer shares.
//!
//! One [`SceneContent`] serves all three visualizers: it owns the sample
//! signal, the resolved style and the invalidation wiring, and delegates the
//! only part that differs — the geometry — to a [`Drawing`].

use kurbo::{Affine, BezPath, Cap, Join, Rect, Stroke};
use peniko::{Brush, Fill};
use waterui_core::{Computed, Signal as _, reactive::watcher::BoxWatcherGuard};
use waterui_graphics::{Scene2D, SceneContent, SceneInvalidator};

use crate::geometry::surface_rect;
use crate::source::{SampleSource, Samples};
use crate::style::{ReactiveStyle, ResolvedStyle, invalidate_on_change};

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
    /// Draws one window of `samples` inside `area`.
    fn draw(&mut self, scene: &mut dyn Scene2D, samples: &[f32], style: &ResolvedStyle, area: Rect);

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
    fn build_scene(&mut self, scene: &mut dyn Scene2D, width: f32, height: f32) -> bool {
        let Some(area) = surface_rect(width, height) else {
            return false;
        };
        let style = self.style.resolve();
        fill_rect(scene, area, &Brush::Solid(style.background));
        let samples = self.samples().get();
        self.drawing.draw(scene, &samples, &style, area);
        false
    }

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
            .push(invalidate_on_change(&samples, &invalidator));
    }
}

/// Fills `rect` with `brush`.
pub fn fill_rect(scene: &mut dyn Scene2D, rect: Rect, brush: &Brush) {
    let mut path = BezPath::new();
    path.move_to((rect.x0, rect.y0));
    path.line_to((rect.x1, rect.y0));
    path.line_to((rect.x1, rect.y1));
    path.line_to((rect.x0, rect.y1));
    path.close_path();
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, &path);
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
pub fn draw_glow(scene: &mut dyn Scene2D, path: &BezPath, style: &ResolvedStyle) {
    if style.glow_intensity <= 0.0 || path.is_empty() {
        return;
    }
    for layer in (1..=GLOW_LAYERS).rev() {
        let spread = GLOW_SPREAD * f64::from(layer);
        let width = style.line_width.mul_add(spread, style.line_width);
        let alpha = style.glow_intensity * GLOW_OPACITY / f32::from(layer);
        scene.stroke(
            &ink_stroke(width),
            Affine::IDENTITY,
            &Brush::Solid(style.glow.multiply_alpha(alpha)),
            None,
            path,
        );
    }
}

/// Strokes `path` in `color` at the style's stroke width.
pub fn stroke_path(
    scene: &mut dyn Scene2D,
    path: &BezPath,
    style: &ResolvedStyle,
    color: peniko::Color,
) {
    if path.is_empty() {
        return;
    }
    scene.stroke(
        &ink_stroke(style.line_width),
        Affine::IDENTITY,
        &Brush::Solid(color),
        None,
        path,
    );
}

/// Fills `path` with `color`.
pub fn fill_path(scene: &mut dyn Scene2D, path: &BezPath, color: peniko::Color) {
    if path.is_empty() {
        return;
    }
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(color),
        None,
        path,
    );
}
