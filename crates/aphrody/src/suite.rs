//! Consolidated compile-time + runtime Aphrody fork capability registry.
//!
//! Enriches [`brand::suite::APHRODY_PILLARS`] with live `#[cfg(feature = "...")]` activation
//! state, unified Material 3 surface modes, and structured JSON export for CLI, MCP, and UI.

use brand::design::ThemeModeSlug;
use brand::suite::{APHRODY_PILLARS, AphrodyPillarSpec, AphrodySurface};
use serde::Serialize;

/// Runtime status of a single consolidated Aphrody fork pillar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AphrodyPillarStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub primary_crate: &'static str,
    pub cargo_feature: &'static str,
    pub summary: &'static str,
    pub compiled: bool,
    pub surfaces: Vec<&'static str>,
}

/// Complete self-describing profile of the consolidated `aphrody` feature set compiled into this binary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AphrodySuiteProfile {
    pub product: &'static str,
    pub slug: &'static str,
    pub design_source_of_truth: &'static str,
    pub theme_modes: Vec<&'static str>,
    pub wcag_aa_min_contrast: f32,
    pub compiled_pillars: usize,
    pub total_pillars: usize,
    pub web_slim: bool,
    pub webgpu_only: bool,
    pub pillars: Vec<AphrodyPillarStatus>,
}

fn is_pillar_compiled(spec: &AphrodyPillarSpec) -> bool {
    match spec.id {
        "brand-design" => true,
        "material3-kit" => cfg!(feature = "kit"),
        "nsh-studio" => cfg!(feature = "gpui"),
        "design-stitch" => cfg!(feature = "design"),
        "craft-suite" => cfg!(feature = "craft"),
        "cosmic-os" => cfg!(any(feature = "desktop-shell", feature = "cosmic")),
        "pilot-mcp" => cfg!(feature = "pilot"),
        "browser" => cfg!(feature = "browser"),
        "store" => cfg!(feature = "store"),
        "installer" => cfg!(feature = "installer"),
        "vpn" => cfg!(feature = "vpn"),
        "auto" => cfg!(feature = "auto"),
        _ => false,
    }
}

impl AphrodySuiteProfile {
    /// Builds the live capability profile for the currently compiled feature set.
    #[must_use]
    pub fn current() -> Self {
        let pillars: Vec<AphrodyPillarStatus> = APHRODY_PILLARS
            .iter()
            .map(|spec| AphrodyPillarStatus {
                id: spec.id,
                name: spec.name,
                primary_crate: spec.primary_crate,
                cargo_feature: spec.cargo_feature,
                summary: spec.summary,
                compiled: is_pillar_compiled(spec),
                surfaces: spec
                    .surfaces
                    .iter()
                    .copied()
                    .map(AphrodySurface::as_str)
                    .collect(),
            })
            .collect();
        let compiled_pillars = pillars.iter().filter(|p| p.compiled).count();
        Self {
            product: brand::APP_NAME,
            slug: brand::APP_SLUG,
            design_source_of_truth: brand::DESIGN_SOURCE_OF_TRUTH,
            theme_modes: ThemeModeSlug::ALL.iter().map(|m| m.slug()).collect(),
            wcag_aa_min_contrast: brand::design::WCAG_AA_MIN_CONTRAST,
            compiled_pillars,
            total_pillars: pillars.len(),
            web_slim: cfg!(feature = "web-slim"),
            webgpu_only: cfg!(feature = "webgpu-only"),
            pillars,
        }
    }

    /// Returns whether a pillar `id` is compiled into this binary.
    #[must_use]
    pub fn has_pillar(&self, id: &str) -> bool {
        self.pillars.iter().any(|p| p.id == id && p.compiled)
    }
}
