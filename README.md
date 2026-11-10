# waterui-visualizer

Real-time audio visualization components for WaterUI: an oscilloscope trace, a
frequency-bar spectrum, and a circular spectrum.

Each visualizer is a map from a sample signal to vector geometry, drawn on a
`waterui-graphics` canvas. The canvas records once, when it mounts: the
geometry is a live operand derived from the sample signal once per audio
window, and the style signals are live operands too, so nothing records again
as sound arrives. The crate owns no GPU device, pipeline or shader, so the
same views are realized by whichever renderer draws the screen around them.

A visualizer draws whatever `SampleSource` it is given: `AudioCapture` is the
microphone, and a `Computed<Samples>` is anything else — a decoded file, a
synthesized signal, a test fixture.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
