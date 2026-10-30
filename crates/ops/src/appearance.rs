//! **Which theme the user asked for** (ADR-0036 §3) — the decision, as distinct from how a renderer
//! carries it. The egui app maps it onto `egui::ThemePreference`; the native client maps it onto the
//! platform's own light/dark setting.
//!
//! It lives here, and not in the renderer, because the store keeps the stored string uninterpreted on
//! purpose: [`ThemeChoice::parse`] is the only place that decides what it means, and a second parser
//! in Kotlin would be a second opinion about a downgraded value (ADR-0046 §3).

use cairn_store::Collection;

/// Stored device-local and never synced (`Collection::theme_preference`), because a desktop under a
/// lamp and a handset in bed want opposite answers. [`ThemeChoice::System`] is the default and the
/// fallback for anything unrecognised, which is what an older build's value looks like after a
/// downgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Follow the platform's own light/dark setting. The default.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    /// The stored form. Lower-case and stable — it is written to a database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Read a stored value. **Anything unrecognised — including `None` — is [`Self::System`]**,
    /// never an error: the store keeps this string uninterpreted on purpose, so this is the only
    /// place that decides what it means, and a value from a build that knew more themes than this
    /// one must degrade to following the platform rather than refusing to start.
    pub fn parse(stored: Option<&str>) -> Self {
        match stored {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }
}

/// The stored choice, read: what both renderers consult before their first frame.
pub fn theme_choice(coll: &Collection) -> Result<ThemeChoice, String> {
    let stored = coll.theme_preference().map_err(|e| e.to_string())?;
    Ok(ThemeChoice::parse(stored.as_deref()))
}
