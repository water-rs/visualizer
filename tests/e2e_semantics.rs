//! End-to-end accessibility-semantics tests for the `visualizer` component.
//!
//! The semantics of a visualizer are a property of the view, not of the sound
//! going into it, so these draw from a fixed synthetic window. Reaching for a
//! microphone would make the assertions depend on the machine having one — the
//! testing backend draws the scene, so a live capture really would open.

use waterui::ViewExt as _;
use waterui::accessibility::AccessibilityRole;
use waterui_core::Computed;
use waterui_testing::{Role, SemanticApp};
use waterui_visualizer::{RadialSpectrum, SAMPLES_COUNT, Samples, SpectrumBars, Waveform, silence};

/// The fixed window every visualizer under test draws from.
fn samples() -> Computed<Samples> {
    Computed::constant(silence(SAMPLES_COUNT))
}

fn waveform_view() -> impl waterui::View {
    Waveform::new(samples())
        .width(180.0)
        .height(120.0)
        .a11y_role(AccessibilityRole::Image)
        .a11y_label("Waveform")
}

#[waterui::test(waveform_view)]
fn waveform_exposes_accessibility_image(app: &mut SemanticApp) {
    app.query()
        .role(Role::IMAGE)
        .label("Waveform")
        .assert_exists();
}

fn spectrum_bars_view() -> impl waterui::View {
    SpectrumBars::new(samples())
        .width(180.0)
        .height(120.0)
        .a11y_role(AccessibilityRole::Image)
        .a11y_label("Spectrum")
}

#[waterui::test(spectrum_bars_view)]
fn spectrum_bars_expose_accessibility_image(app: &mut SemanticApp) {
    app.query()
        .role(Role::IMAGE)
        .label("Spectrum")
        .assert_exists();
}

fn radial_spectrum_view() -> impl waterui::View {
    RadialSpectrum::new(samples())
        .width(160.0)
        .height(160.0)
        .a11y_role(AccessibilityRole::Image)
        .a11y_label("Radial spectrum")
}

#[waterui::test(radial_spectrum_view)]
fn radial_spectrum_exposes_accessibility_image(app: &mut SemanticApp) {
    app.query()
        .role(Role::IMAGE)
        .label("Radial spectrum")
        .assert_exists();
}
