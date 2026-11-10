//! The canvas every visualizer shares.
//!
//! A visualizer's canvas closure runs once, at mount. Everything that moves
//! afterwards is a live operand of one retained command: the geometry is a
//! signal derived from the sample windows, and the style is a set of signals,
//! so a new audio window or a style change updates those operands in place and
//! never records the canvas again.

use kurbo::{BezPath, PathEl};
use waterui_core::layout::{Point, Rect};
use waterui_core::reactive::Signal;
use waterui_core::{Computed, SignalExt as _};
use waterui_graphics::canvas::{Cap, Cx, Join, Path, StrokeStyle};

use crate::source::{SampleSource, Samples};
use crate::style::Style;

/// How many halo passes make up a glow.
///
/// A glow is a stack of progressively wider, fainter strokes of the same path.
/// Three is where another pass stops being visible against the one under it.
const GLOW_LAYERS: u16 = 3;

/// How much wider each halo pass is than the stroke it surrounds.
const GLOW_SPREAD: f32 = 3.0;

/// Opacity of the innermost halo pass at full glow intensity.
const GLOW_OPACITY: f32 = 0.35;

/// Subscribes to `source` for the lifetime of the canvas's recording.
///
/// Subscribing is what opens a capture session, so it happens when the canvas
/// mounts rather than when the view is built. The source is then kept by the
/// recording: when the canvas unmounts the recording drops, and with it the
/// source, which closes the hardware it opened.
pub fn subscribe<S: SampleSource>(cx: &mut Cx, source: S) -> Computed<Samples> {
    let samples = source.subscribe();
    cx.keep(source);
    samples
}

/// The path `build` makes of `inputs`, recomputed once per change of `inputs`
/// and shared by every command that draws it.
///
/// The returned signal is cached by the recording, not by its readers: a path
/// drawn by several commands (a halo, a fill and an outline) is built once per
/// audio window.
pub fn derive_path<I: Clone + 'static>(
    inputs: &impl Signal<Output = I>,
    build: impl Fn(I) -> BezPath + 'static,
) -> Computed<Path> {
    inputs
        .scan((), move |(), input| canvas_path(&build(input)))
        .computed()
}

/// `path` as the canvas's own path type.
pub fn canvas_path(path: &BezPath) -> Path {
    path.elements()
        .iter()
        .fold(Path::builder(), |builder, element| match *element {
            PathEl::MoveTo(to) => builder.move_to(point(to)),
            PathEl::LineTo(to) => builder.line_to(point(to)),
            PathEl::QuadTo(control, to) => builder.quad_to(point(control), point(to)),
            PathEl::CurveTo(first, second, to) => {
                builder.cubic_to(point(first), point(second), point(to))
            }
            PathEl::ClosePath => builder.close(),
        })
        .build()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "canvas geometry is authored in f32 points; visualizer geometry is f64"
)]
const fn point(point: kurbo::Point) -> Point {
    Point::new(point.x as f32, point.y as f32)
}

/// The stroke a visualizer's ink is drawn with.
const fn ink_stroke(width: f32) -> StrokeStyle {
    StrokeStyle::new(width.max(0.0))
        .cap(Cap::Round)
        .join(Join::Round)
}

/// Fills the whole canvas with the style's background.
pub fn draw_background(cx: &mut Cx, style: &Style) {
    cx.fill(cx.size().map(Rect::from_size), style.background().clone());
}

/// Draws the halo around `path`, widest and faintest pass first.
///
/// This is what the old fragment shader's exponential falloff becomes in vector
/// terms: the same path, stroked a few times at growing widths and shrinking
/// opacity, which any canvas target can draw without a shader of its own.
/// Whether there is a halo at all depends on the glow intensity, so that is the
/// one region that records again when the intensity crosses zero; its width,
/// colour and strength are live operands.
pub fn draw_glow(cx: &mut Cx, style: &Style, path: &Computed<Path>) {
    let (path, glow, width, intensity) = (
        path.clone(),
        style.glow().clone(),
        style.line_width().clone(),
        style.glow_intensity().clone(),
    );
    cx.scope(
        intensity.map(|intensity| intensity > 0.0),
        move |cx, visible: &bool| {
            if !*visible {
                return;
            }
            for layer in (1..=GLOW_LAYERS).rev() {
                let layer = f32::from(layer);
                let spread = GLOW_SPREAD * layer;
                let alpha = GLOW_OPACITY / layer;
                cx.stroke(
                    path.clone(),
                    width.map(move |width| ink_stroke(width.max(0.0) * (1.0 + spread))),
                    glow.zip(&intensity).map(move |(glow, intensity)| {
                        glow.with_alpha(glow.components[3] * intensity.clamp(0.0, 1.0) * alpha)
                    }),
                );
            }
        },
    );
}

/// Strokes `path` in the style's ink colour and stroke width.
pub fn draw_outline(cx: &mut Cx, style: &Style, path: &Computed<Path>) {
    cx.stroke(
        path.clone(),
        style.line_width().map(ink_stroke),
        style.line().clone(),
    );
}

/// Fills `path` in the style's ink colour, at `opacity` of its alpha.
pub fn draw_interior(cx: &mut Cx, style: &Style, path: &Computed<Path>, opacity: f32) {
    cx.fill(
        path.clone(),
        style
            .line()
            .map(move |line| line.with_alpha(line.components[3] * opacity)),
    );
}
