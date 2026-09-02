# Changelog

All notable changes to `waterui-visualizer` are documented in this file.

## [Unreleased]

## [0.1.0](https://github.com/water-rs/visualizer/releases/tag/v0.1.0) - 2026-09-02

### Fixed

- *(release)* close package rehearsal gaps
- *(release)* verify registry-only package graph
- fix repository rule violations and refresh documentation
- *(apple)* link native media session provider

### Other

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
