//! The sync **experience** — everything the user sees about sync, and almost all of it made of
//! things the application refuses to say (ADR-0015, ADR-0019). The `leitner-sync` crate holds the
//! *mechanism*; this module holds the *surface*, and it is deliberately pure: the trigger policy,
//! the notice channel, the resting statement, the enrolment sentences and the one capability
//! constant are decisions and words, testable with no window, no network and no handset.
//!
//! **What is not here, and why.** The live enrolment flow — the device flow, the credential file,
//! the UserInfo fetch — carries HTTP, TLS and OAuth and a Google Drive backend, none of which is
//! exercisable in this environment (ADR-0013 §11, and #91 deferred it for the same reason). So this
//! module is the surface's *shape and copy*, consumed by `lib.rs`; the code that obtains a grant and
//! fires a network sync is its own step. The value of pinning the surface now is that it is made
//! almost entirely of refusals — the parts that erode silently — and those are provable without any
//! of the machinery below them.
//!
//! The rules this module exists to keep, each one a defect that no test in the mechanism could catch:
//!
//! - **Exactly two things may speak about sync** (ADR-0015 §5): a dead grant, and ADR-0004 §8's
//!   clock-skew warning. [`Notice`] can express *only* those two — a network failure has no variant,
//!   because offline is normal and nagging about it is the defect.
//! - **The resting surface states a fact, never a claim** (ADR-0015 §4): [`last_caught_up`] is the
//!   only standing statement, and there is no "in sync", no badge, no checkmark anywhere.
//! - **Sync runs on three foreground triggers and never in the background** (ADR-0015 §2), and
//!   **never starts while a sitting is running** (ADR-0015 §6). [`Trigger`] enumerates the three and
//!   [`should_start`] is the whole gate; the absence of any scheduler is what makes "no background
//!   sync" true, and it is load-bearing for §6, not a limitation to lift.
//! - **The Android text-input limitation is stated in advance** (ADR-0015 §9), off [`LATIN_INPUT_ONLY`]
//!   — the one sanctioned `cfg(target_os)` capability constant (ADR-0015 §15), which makes a
//!   limitation *visible* rather than varying behaviour.

use std::time::Duration;

/// The three foreground triggers, and nothing else (ADR-0015 §2). There is deliberately no
/// `Background`, no `Timer` and no `PerCard` variant: **there is no background sync on either
/// platform**, and that absence is what makes ADR-0015 §6's mid-session gate enforceable rather than
/// a limitation to be lifted later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// Launch, or the window or app regaining focus — the trigger that matters, because it puts the
    /// queue in front of the user already current. Subject to [`RECENCY_FLOOR`] so alt-tabbing does
    /// not hammer the remote.
    BecameActive,
    /// The end of a sitting — the first moment the local log holds something worth publishing.
    SessionEnd,
    /// The **Sync now** control in sync settings, and Optimise's leading step.
    UserAction,
}

/// The debounce for [`Trigger::BecameActive`] (ADR-0015 §2, trigger 1). A *debounce, not a schedule*
/// — the open item is explicit that this is an implementation value, not a compatibility constant, so
/// it may be tuned freely. Sixty seconds is enough that alt-tabbing between two windows does not
/// re-list the remote on every switch, and short enough that returning to the app after a coffee
/// re-syncs.
pub const RECENCY_FLOOR: Duration = Duration::from_secs(60);

/// Whether a sync may **start** right now (ADR-0015 §2, §6). This is the whole gate, and it is a gate
/// on *starting*, never on reviewing — ADR-0015 §1 forbids ever blocking review.
///
/// - `session_running` is true while a sitting is in progress (a card is on screen). A sync **never
///   starts** then (ADR-0015 §6): it is what stops another device's merge recomputing every `(S, D)`
///   mid-session. The count picker is *not* a running sitting, and a sync landing while it is up is
///   deliberately not suppressed (ADR-0015 §6) — but that is a *landing*, not a start, and only a
///   start is gated here.
/// - `in_flight` true means a sync is already running: **a sync in flight is allowed to finish**
///   (ADR-0015 §6), so we never start a second on top of it.
/// - `since_last` is the time since the last sync completed, or `None` if none has. Only
///   [`Trigger::BecameActive`] consults it, against `floor`; the other two are explicit enough that a
///   debounce would only frustrate.
pub fn should_start(
    trigger: Trigger,
    session_running: bool,
    in_flight: bool,
    since_last: Option<Duration>,
    floor: Duration,
) -> bool {
    // A sitting on screen and a sync already running both forbid *starting* one, for every trigger.
    if session_running || in_flight {
        return false;
    }
    match trigger {
        // The becoming-active trigger is the only debounced one: alt-tabbing must not re-list.
        Trigger::BecameActive => since_last.is_none_or(|elapsed| elapsed >= floor),
        // Session end and an explicit press are the user's own rhythm — honoured immediately.
        Trigger::SessionEnd | Trigger::UserAction => true,
    }
}

/// The only two things permitted to speak about sync (ADR-0015 §5). This type is the enforcement of
/// that rule: it can represent a dead grant and a clock-skew warning **and nothing else**. There is
/// no `NetworkFailure` — offline is normal and must never nag (ADR-0015 §5) — and no success, no
/// "in sync", no progress: those are ADR-0015 §4 claims the application cannot back. Adding a third
/// variant here is the defect that ADR the rule predicted would erode first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// The grant died — an expired refresh token, a specific response the app can tell from a
    /// timeout (ADR-0015 §5). The one sync state the user must act on; persistent and non-modal.
    DeadGrant,
    /// ADR-0004 §8's clock-skew warning, surfaced here (ADR-0015 §11). It **names the device** and
    /// **never offers the repair inline** — the cutoff discards good history with bad, so it lives in
    /// sync settings behind its explanation, never one tap from a notice met by surprise. Dismissible
    /// and keyed to the rows that triggered it, so it does not re-fire on every later merge.
    ClockSkew {
        /// The offending device's own label (ADR-0015 §8, §11) — the writer id is not a sentence.
        device: String,
    },
}

impl Notice {
    /// The persistent, non-modal line this notice shows. States the fact and, for skew, names the
    /// device; neither offers a repair (ADR-0015 §5, §11).
    pub fn message(&self) -> String {
        match self {
            Notice::DeadGrant => {
                "Sync access has expired. Set it up again in Settings to keep catching up."
                    .to_owned()
            }
            Notice::ClockSkew { device } => {
                format!("{device}'s clock looks wrong: it logged reviews dated far in the future.")
            }
        }
    }
}

/// The only resting statement the application makes about sync (ADR-0015 §4): *when* it last
/// completed one, a fact. Never "in sync", never "up to date" — after a sync the app knows every
/// writer's highest *published* sequence and never whether another device has reviewed since, so a
/// claim of agreement is unknowable.
///
/// `when_ms` is when sync last completed, or `None` on a device that has enrolled but not yet caught
/// up. `now_ms` is the current wall clock. A clock that has gone backwards (a negative delta) reads
/// as "just now" rather than a nonsensical future age.
pub fn last_caught_up(when_ms: Option<i64>, now_ms: i64) -> String {
    match when_ms {
        None => "Not caught up yet".to_owned(),
        Some(when) => format!(
            "Last caught up {}",
            relative_age(now_ms.saturating_sub(when))
        ),
    }
}

/// A coarse, human relative age for [`last_caught_up`]. Deliberately coarse — the resting surface is
/// a fact the user is not watching (ADR-0015 §4), so minute precision on a four-week-old sync would
/// be false detail. Rounds down; a negative delta (clock skew) is treated as the present.
fn relative_age(delta_ms: i64) -> String {
    const MINUTE: i64 = 60_000;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const WEEK: i64 = 7 * DAY;
    if delta_ms < MINUTE {
        "just now".to_owned()
    } else if delta_ms < HOUR {
        plural(delta_ms / MINUTE, "minute")
    } else if delta_ms < DAY {
        plural(delta_ms / HOUR, "hour")
    } else if delta_ms < WEEK {
        plural(delta_ms / DAY, "day")
    } else {
        plural(delta_ms / WEEK, "week")
    }
}

/// `"1 minute ago"`, `"3 minutes ago"` — the pluralisation the relative age needs.
fn plural(count: i64, unit: &str) -> String {
    let s = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{s} ago")
}

/// What enrolment found in the folder (ADR-0015 §7). The device that connects to the **wrong**
/// account gets an empty folder, identical to being the first device — so this fact, stated at the
/// one moment the user could notice, is the whole defence, and the connected account beside it is
/// what turns detection into diagnosis (ADR-0019 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// No other device's namespace is present. Said to someone who knows they enrolled another
    /// device, this is the sentence that catches a wrong-account enrolment (ADR-0019 §2).
    FirstDevice,
    /// The devices already publishing here, by their self-assigned labels (ADR-0015 §8).
    Others(Vec<String>),
}

impl Found {
    /// Build from a device-label list, collapsing the empty case to [`Found::FirstDevice`] so the
    /// enrolment sentence never reads "Found 0 other devices" — an empty folder *is* being first.
    pub fn from_devices(devices: Vec<String>) -> Self {
        if devices.is_empty() {
            Found::FirstDevice
        } else {
            Found::Others(devices)
        }
    }
}

/// The sentence enrolment ends on (ADR-0015 §7, amended by ADR-0019 §1): the account it connected as,
/// then what it found. The two halves do different jobs and neither is redundant — the account
/// **diagnoses** a wrong-account enrolment and "first device here" **detects** it (ADR-0019 §2) — so
/// both are always present.
///
/// This is two of the three things enrolment states; the third, what leaves the device, is
/// [`DISCLOSURE_CLAUSE`], said once alongside this (ADR-0020 §7).
pub fn enrolment_summary(account: &str, found: &Found) -> String {
    match found {
        Found::FirstDevice => format!("Connected as {account}. This is the first device here."),
        Found::Others(devices) => {
            let noun = if devices.len() == 1 {
                "device"
            } else {
                "devices"
            };
            format!(
                "Connected as {account}. Found {} other {noun}: {}.",
                devices.len(),
                devices.join(", ")
            )
        }
    }
}

/// The name of the action that grants this device access, once (ADR-0015 §7). Never *login*,
/// *sign-in* or *pairing* — there is no account of ours and no device-to-device step.
pub const SET_UP_SYNC: &str = "Set up sync";

/// The promise, worded exactly once and reused wherever it appears (ADR-0015 §3): enrolment and sync
/// settings, and nowhere else. **Never "automatic", never "always in sync", never "in the
/// background"** — it is the copy a later hand would get wrong, so it is a constant.
pub const PROMISE: &str = "Your devices catch up when you open the app.";

/// What the consent screen asks for, in plain words (ADR-0015 §7, amended by ADR-0019 §4). The scope
/// set is `openid email drive.appdata`, so it asks for two things: the email address (the diagnostic
/// of ADR-0019 §2) and a hidden folder only this application can see — **not the user's files**.
pub const SCOPE_PLAIN_WORDS: &str = "This grants access to your email address and to a private folder only this app can see — never \
     your own files.";

/// The disclosure clause: what leaves the device, stated **once** at enrolment (ADR-0020 §7, sync
/// `CONTEXT.md`). It is not a status message and must never be promoted to a resting surface
/// (ADR-0015 §5). Its durable half — *how to remove it* — lives permanently in sync settings as
/// [`revocation_and_removal`], because the failure it guards against is discovered months later.
pub const DISCLOSURE_CLAUSE: &str = "Your review history is published to that folder in plain text. Nothing published is encrypted.";

/// The name this application appears under in the drive's connected-applications settings (ADR-0015
/// §10, ADR-0013 §3's fourth console trap). It cannot be validated by any code here — it is the
/// consent screen's application name, a console setting — so if it does not match what the user knows
/// the application as, *"find it in the list"* fails silently. Kept as a constant so there is one
/// place to correct it when the console is set.
pub const APP_NAME: &str = "Leitner";

/// The connected account row's label in settings (ADR-0019 §1): *"Connected as you@example.com"*,
/// kept — not shown once and discarded — because the wrong-account failure surfaces months later and
/// two settings screens read side by side are the only cross-device account comparison that exists
/// (ADR-0019 §3, §7).
pub fn connected_as(account: &str) -> String {
    format!("Connected as {account}")
}

/// How to stop syncing and how to remove published data (ADR-0015 §10, ADR-0020 §4). **Disconnect is
/// the only control this app owns**; deletion is the provider's, because the `drive.appdata` grant
/// reaches the whole folder and a delete from here would destroy other writers' rows. So settings
/// *names* the route and the application name rather than pretending to a control it must not have.
pub fn revocation_and_removal() -> String {
    format!(
        "Disconnect stops syncing on this device and deletes nothing — reconnect any time. To remove \
         the published data, delete this app's data from your drive's connected-apps settings, where \
         it appears as \"{APP_NAME}\". Revoking access there signs out every device you own."
    )
}

/// True on platforms whose text input cannot accept non-Latin script (ADR-0015 §9). This is the
/// **one sanctioned exception** to the no-`cfg(target_os)` rule (ADR-0015 §15, client-stack rule 8):
/// a compile-time constant whose only job is to make a limitation *visible*, which is the inverse of
/// the behaviour-divergence that rule guards against. winit's Android backend has no IME path, so
/// composed non-Latin text never reaches the app and the failure is *silence* — nothing happens at
/// all — which can only be stated in advance.
#[cfg(target_os = "android")]
pub const LATIN_INPUT_ONLY: bool = true;

/// See the Android arm above — false everywhere the platform has a working IME.
#[cfg(not(target_os = "android"))]
pub const LATIN_INPUT_ONLY: bool = false;

/// The standing quiet line the editor carries when [`LATIN_INPUT_ONLY`] holds (ADR-0015 §9). Stated
/// in advance, at the point of authoring, because the failure it describes arrives as silence.
pub const DESKTOP_AUTHORING_LINE: &str =
    "This device types Latin text only — author other scripts on the desktop and they sync here.";

#[cfg(test)]
mod tests {
    use super::*;

    // --- the trigger gate (ADR-0015 §2, §6) ---

    #[test]
    fn a_sync_never_starts_while_a_sitting_is_running() {
        // ADR-0015 §6: the mid-session gate, applied to every trigger. This is the whole reason a
        // merge cannot recompute every (S, D) under the user mid-review.
        for trigger in [
            Trigger::BecameActive,
            Trigger::SessionEnd,
            Trigger::UserAction,
        ] {
            assert!(
                !should_start(trigger, true, false, None, RECENCY_FLOOR),
                "{trigger:?} must not start a sync while a sitting is running",
            );
        }
    }

    #[test]
    fn a_sync_in_flight_is_left_to_finish_and_no_second_starts() {
        // ADR-0015 §6: one in flight finishes; we never start a second on top of it.
        for trigger in [
            Trigger::BecameActive,
            Trigger::SessionEnd,
            Trigger::UserAction,
        ] {
            assert!(!should_start(trigger, false, true, None, RECENCY_FLOOR));
        }
    }

    #[test]
    fn becoming_active_is_debounced_by_the_recency_floor() {
        // ADR-0015 §2 trigger 1: a debounce so alt-tabbing does not hammer the remote.
        let floor = RECENCY_FLOOR;
        // Never synced → allowed.
        assert!(should_start(
            Trigger::BecameActive,
            false,
            false,
            None,
            floor
        ));
        // Within the floor → suppressed.
        assert!(!should_start(
            Trigger::BecameActive,
            false,
            false,
            Some(floor - Duration::from_secs(1)),
            floor,
        ));
        // At or past the floor → allowed.
        assert!(should_start(
            Trigger::BecameActive,
            false,
            false,
            Some(floor),
            floor,
        ));
    }

    #[test]
    fn session_end_and_user_action_ignore_the_recency_floor() {
        // ADR-0015 §2: the user's own rhythm is honoured immediately — a debounce there only
        // frustrates. Even a sync one second ago does not suppress them.
        let just_now = Some(Duration::from_secs(1));
        assert!(should_start(
            Trigger::SessionEnd,
            false,
            false,
            just_now,
            RECENCY_FLOOR,
        ));
        assert!(should_start(
            Trigger::UserAction,
            false,
            false,
            just_now,
            RECENCY_FLOOR,
        ));
    }

    // --- the notice channel: exactly two speakers (ADR-0015 §5) ---

    #[test]
    fn the_clock_skew_notice_names_the_device_and_offers_no_repair() {
        // ADR-0015 §11: names the device, states the fact, never offers the cutoff inline.
        let msg = Notice::ClockSkew {
            device: "Laptop".to_owned(),
        }
        .message();
        assert!(msg.contains("Laptop"), "must name the device: {msg}");
        // The repair (history cutoff) is never one tap from the notice.
        let lower = msg.to_lowercase();
        assert!(
            !lower.contains("cutoff") && !lower.contains("repair") && !lower.contains("fix"),
            "must not offer the repair inline: {msg}",
        );
    }

    #[test]
    fn the_dead_grant_notice_is_the_one_thing_the_user_must_act_on() {
        // ADR-0015 §5: the single sync state that warrants a persistent notice.
        assert!(!Notice::DeadGrant.message().is_empty());
    }

    // --- the resting statement (ADR-0015 §4) ---

    #[test]
    fn the_resting_statement_is_a_fact_never_a_claim() {
        // ADR-0015 §4: "Last caught up ⟨when⟩", and never a claim of agreement.
        let now = 1_000_000_000_000;
        let line = last_caught_up(Some(now - 5 * 60_000), now);
        assert_eq!(line, "Last caught up 5 minutes ago");
        // No forbidden claim appears in any resting statement this function produces.
        for when in [None, Some(now), Some(now - 4 * 7 * 24 * 60 * 60_000)] {
            let l = last_caught_up(when, now).to_lowercase();
            for banned in ["in sync", "up to date", "synced", "✓"] {
                assert!(
                    !l.contains(banned),
                    "resting statement claimed {banned:?}: {l}"
                );
            }
        }
    }

    #[test]
    fn relative_age_is_coarse_and_pluralises() {
        let now = 10_000_000_000_000;
        assert_eq!(last_caught_up(Some(now), now), "Last caught up just now");
        assert_eq!(
            last_caught_up(Some(now - 60_000), now),
            "Last caught up 1 minute ago",
        );
        assert_eq!(
            last_caught_up(Some(now - 2 * 60 * 60_000), now),
            "Last caught up 2 hours ago",
        );
        assert_eq!(
            last_caught_up(Some(now - 24 * 60 * 60_000), now),
            "Last caught up 1 day ago",
        );
        assert_eq!(
            last_caught_up(Some(now - 4 * 7 * 24 * 60 * 60_000), now),
            "Last caught up 4 weeks ago",
        );
    }

    #[test]
    fn a_backwards_clock_reads_as_the_present_not_a_future_age() {
        // A device whose clock jumped back would otherwise show a nonsensical negative age.
        let now = 1_000;
        assert_eq!(
            last_caught_up(Some(now + 5_000), now),
            "Last caught up just now"
        );
    }

    #[test]
    fn a_never_caught_up_device_says_so_without_claiming_anything() {
        assert_eq!(last_caught_up(None, 1_000), "Not caught up yet");
    }

    // --- enrolment states three things (ADR-0015 §7, ADR-0019 §1) ---

    #[test]
    fn enrolment_names_the_account_then_states_it_is_the_first_device() {
        // ADR-0019 §1 + ADR-0015 §7: the sentence that both diagnoses and detects a wrong account.
        assert_eq!(
            enrolment_summary("you@example.com", &Found::FirstDevice),
            "Connected as you@example.com. This is the first device here.",
        );
    }

    #[test]
    fn enrolment_names_the_account_then_lists_the_devices_it_met() {
        assert_eq!(
            enrolment_summary(
                "you@example.com",
                &Found::Others(vec!["Laptop".to_owned(), "Pixel".to_owned()]),
            ),
            "Connected as you@example.com. Found 2 other devices: Laptop, Pixel.",
        );
    }

    #[test]
    fn one_other_device_is_singular() {
        assert_eq!(
            enrolment_summary("a@b.c", &Found::Others(vec!["Laptop".to_owned()])),
            "Connected as a@b.c. Found 1 other device: Laptop.",
        );
    }

    #[test]
    fn an_empty_device_list_is_being_the_first_device() {
        // "Found 0 other devices" is never a sentence the user reads — an empty folder is being first.
        assert_eq!(Found::from_devices(vec![]), Found::FirstDevice);
        assert_eq!(
            Found::from_devices(vec!["Laptop".to_owned()]),
            Found::Others(vec!["Laptop".to_owned()]),
        );
    }

    // --- the copy that names the console route (ADR-0015 §10, ADR-0020 §4) ---

    #[test]
    fn settings_names_disconnect_the_removal_route_and_the_app_name() {
        let copy = revocation_and_removal();
        assert!(
            copy.contains("Disconnect"),
            "names the only control: {copy}"
        );
        assert!(
            copy.contains("deletes nothing"),
            "disconnect deletes nothing"
        );
        assert!(
            copy.contains(APP_NAME),
            "names the app for the console list"
        );
        assert!(
            copy.contains("connected-apps") || copy.contains("connected-applications"),
            "points at the provider's own route: {copy}",
        );
    }

    #[test]
    fn the_promise_never_says_automatic_or_in_sync() {
        // ADR-0015 §3: the copy written later by someone not in the conversation.
        let p = PROMISE.to_lowercase();
        for banned in [
            "automatic",
            "always in sync",
            "in the background",
            "background",
        ] {
            assert!(!p.contains(banned), "promise said {banned:?}: {PROMISE}");
        }
    }

    #[test]
    fn the_capability_constant_matches_the_target() {
        // ADR-0015 §9, §15: the one sanctioned cfg constant. Its value is the platform's, and the
        // standing line exists to be shown where it holds.
        assert!(!DESKTOP_AUTHORING_LINE.is_empty());
        // Latin-only exactly on Android, false everywhere with a working IME.
        assert_eq!(LATIN_INPUT_ONLY, cfg!(target_os = "android"));
    }
}
