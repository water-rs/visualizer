//! Shared audio capture logic for visualizers.

use futures::StreamExt;
use std::{
    cell::Cell,
    fmt,
    rc::{Rc, Weak},
};
use waterkit_audio::{AudioBuffer, AudioRecorderBuilder};
use waterui_core::{Binding, Computed, SignalExt as _};

use crate::source::{SampleSource, Samples, silence};

/// Number of audio samples in the buffer.
pub const SAMPLES_COUNT: usize = 1024;

/// Shared microphone capture, published as a [`Samples`] signal.
///
/// Construction is side-effect free. The first visualizer drawing from the
/// capture starts one recorder task; clones share that task and the signal it
/// feeds, so several visualizers on one microphone cost one recording session.
#[derive(Clone)]
pub struct AudioCapture {
    state: Rc<AudioCaptureState>,
}

struct AudioCaptureState {
    samples: Binding<Samples>,
    active: Cell<bool>,
}

impl fmt::Debug for AudioCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioCapture")
            .field("active", &self.state.active.get())
            .finish_non_exhaustive()
    }
}

impl AudioCapture {
    /// Create a shared audio capture session.
    ///
    /// Recording starts when the first visualizer using this value starts
    /// drawing, not here.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Rc::new(AudioCaptureState {
                samples: Binding::container(silence(SAMPLES_COUNT)),
                active: Cell::new(false),
            }),
        }
    }

    /// Starts the recorder task, unless this capture already has one.
    fn activate(&self) {
        if self.state.active.replace(true) {
            return;
        }

        let (sender, receiver) = async_channel::bounded(1);
        let state = Rc::downgrade(&self.state);

        executor_core::spawn_local(async move {
            let capture = blocking::unblock(move || capture_audio(sender));
            futures::future::join(capture, publish(receiver, state)).await;
        })
        .detach();
    }
}

impl SampleSource for AudioCapture {
    fn subscribe(&self) -> Computed<Samples> {
        self.activate();
        self.state.samples.computed()
    }
}

/// Feeds recorded buffers into `state` as complete, in-order sample windows.
///
/// The recorder hands over buffers of whatever size the platform chose, so the
/// window is assembled in a ring and published rotated: the oldest sample is
/// always first, and a visualizer never sees the write cursor as a step in the
/// middle of its drawing.
#[expect(
    clippy::future_not_send,
    reason = "the capture state is main-thread `Rc` state, spawned onto the local executor"
)]
async fn publish(receiver: async_channel::Receiver<AudioBuffer>, state: Weak<AudioCaptureState>) {
    let mut ring = vec![0.0_f32; SAMPLES_COUNT];
    let mut write_pos = 0;
    while let Ok(buffer) = receiver.recv().await {
        let Some(state) = state.upgrade() else {
            break;
        };
        for &sample in buffer.samples() {
            ring[write_pos] = sample;
            write_pos = (write_pos + 1) % SAMPLES_COUNT;
        }
        let mut window = Vec::with_capacity(SAMPLES_COUNT);
        window.extend_from_slice(&ring[write_pos..]);
        window.extend_from_slice(&ring[..write_pos]);
        state.samples.set(Samples::from(window));
    }
}

fn capture_audio(sender: async_channel::Sender<AudioBuffer>) {
    futures::executor::block_on(async move {
        let mut recorder = AudioRecorderBuilder::new()
            .build()
            .expect("AudioCapture failed to create the platform recorder");
        recorder
            .start()
            .await
            .expect("AudioCapture failed to start the platform recorder");
        tracing::info!("Audio capture started");

        let mut audio_stream = Box::pin(recorder.stream());
        loop {
            let buffer = audio_stream
                .next()
                .await
                .expect("AudioCapture stream ended while the capture session was active");
            if sender.force_send(buffer).is_err() {
                break;
            }
        }
        drop(audio_stream);

        recorder
            .stop()
            .await
            .expect("AudioCapture failed to stop the platform recorder");
    });
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}
