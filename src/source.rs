//! The sample signal every visualizer draws from.

use std::sync::Arc;

use waterui_core::{Binding, Computed, SignalExt as _};

/// One window of time-domain audio samples, oldest first.
///
/// Values are normalized to `-1.0..=1.0`. Every window a source publishes has
/// the same length, so a visualizer laying samples out across its width never
/// has to deal with a changing horizontal resolution.
pub type Samples = Arc<[f32]>;

/// A live source of the sample windows a visualizer draws.
///
/// [`subscribe`](SampleSource::subscribe) is called once, when a visualizer
/// starts drawing. A source that owns hardware opens it there rather than when
/// the view is constructed, so building a view never starts a microphone —
/// which is also what lets a visualizer be rendered from a synthetic signal
/// with no capture session at all.
pub trait SampleSource: 'static {
    /// Returns the signal carrying successive sample windows.
    fn subscribe(&self) -> Computed<Samples>;
}

impl SampleSource for Computed<Samples> {
    fn subscribe(&self) -> Computed<Samples> {
        self.clone()
    }
}

impl SampleSource for Binding<Samples> {
    fn subscribe(&self) -> Computed<Samples> {
        self.computed()
    }
}

/// A window of `length` silent samples.
#[must_use]
pub fn silence(length: usize) -> Samples {
    Samples::from(vec![0.0; length])
}
