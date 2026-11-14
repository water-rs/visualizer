# Changelog

All notable changes to `waterui-visualizer` are documented in this file.

## [Unreleased]

## [0.6.0](https://github.com/water-rs/visualizer/compare/v0.5.0...v0.6.0) - 2026-11-14

### Added

- [**breaking**] draw through the waterui Canvas API
- port to the render-target-neutral recording contract

### Fixed

- drop the spectrum keep-alives now that a watched scan holds its upstream
- *(ci)* make the MSRV job build with the declared floor ([#29](https://github.com/water-rs/visualizer/pull/29))

### Other

- release from main and point sibling deps to main
- restore the Hydrolysis test fonts from the Actions cache ([#41](https://github.com/water-rs/visualizer/pull/41))
- *(deps)* point migrated Batch 1 git dependencies to main ([#38](https://github.com/water-rs/visualizer/pull/38)) ([#39](https://github.com/water-rs/visualizer/pull/39))
- pass the release pull request's gate without CI
- drop the nami git pin and ride waterui dev at ec09f64
- target the engine's Apple deployment floor
- trim the patch table to the entries the graph uses
- repin the vello fork to the glifo canonical-location fix
- consume the 0.5.0 framework wave from crates.io ([#28](https://github.com/water-rs/visualizer/pull/28))

## [0.4.0](https://github.com/water-rs/visualizer/compare/v0.3.0...v0.4.0) - 2026-09-11

### Added

- [**breaking**] draw every visualizer through Scene2D instead of a wgpu pipeline

### Fixed

- *(ci)* install the same Linux packages for the release preflight
- *(release)* close package rehearsal gaps
- *(release)* verify registry-only package graph
- fix repository rule violations and refresh documentation
- *(apple)* link native media session provider

### Other

- link the test graph to the waterui 0.4 release commit
- *(deps)* waterui-graphics 0.4 and waterui-testing 0.4; version 0.3.0 matches the registry
- resolve gpu-allocator against windows 0.62, as wgpu-hal does
- depend on the released waterui crates instead of the monorepo dev branch
- draw the semantics tests from a synthetic window, not a microphone
- refresh git dependency revisions after the upstream history rewrite
- update Linux package matrix and add dxc on Windows
- setup standalone crate files, CI workflows, and release-plz
- consolidate GPU glue into waterui-graphics helpers
- ship the licence texts in every published crate
- depend on shaderloom directly, and give the icon codegen its own name
- prepare WaterUI 0.3 release versions
- Fix workspace CI failures
- Fix Android runtime and device workflows
- upgrade workspace dependencies
- Add cross-platform shader AOT with Shaderloom
- refactor native backends and GPU surface integration
- achieve zero clippy warnings across the workspace
- clean up clippy warnings across the workspace
- SubView: Send + Sync; decouple GpuView from SubView
- Lean dependency graph for embedded: gpu/widgets/gestures features
- Restore WaterUI CI gates and reactive map API
- reorganize the project

## [0.3.0] - 2026-08-25

- Updated audio visualization to the WaterKit 0.1.1 and WaterUI 0.3 GPU contracts.
