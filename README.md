# waterui-visualizer

Real-time audio visualization components for WaterUI: an oscilloscope trace, a
frequency-bar spectrum, and a circular spectrum.

Each visualizer is a map from a sample signal to `kurbo` geometry, drawn through
`waterui-graphics`' engine-neutral `Scene2D` contract. The crate owns no GPU
device, pipeline or shader, so the same views render on the GPU compute
renderer, the CPU sparse-strip renderer used on embedded targets, and any
backend that draws into its own scene.

A visualizer draws whatever `SampleSource` it is given: `AudioCapture` is the
microphone, and a `Computed<Samples>` is anything else — a decoded file, a
synthesized signal, a test fixture.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
