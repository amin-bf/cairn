//! **Motion and elevation** — the throwaway prototype for
//! [#154](https://github.com/amin-bf/cairn/issues/154), split out of
//! [The Craft](https://github.com/amin-bf/cairn/issues/149).
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-154`, the repo's
//! standing convention (`AGENTS.md`, *Rules that are easy to break silently* 3). Reachable from any
//! clone without merging:
//!
//! ```sh
//! git show prototypes/issue-154:docs/design/prototype-154/README.md
//! git checkout prototypes/issue-154 -- crates/app/src/proto.rs
//! ```
//!
//! # Why this is the application and not a desktop bin or an HTML page
//!
//! Two of the three questions here cannot be answered from a still. #124 judged variant E as a
//! **running sitting** for a weaker reason than this one has: a duration and a curve only exist in
//! time, and the ticket's own question — *does a fade actually stop the reveal reading as a
//! jump-cut* — is a question about what the eye does over 200 milliseconds. The third question, the
//! overlay's material, *can* be photographed, and is; it rides along because the combo box it lives
//! on is two taps away and switching prototypes mid-sitting costs a rebuild.
//!
//! So this is the shape [#141](https://github.com/amin-bf/cairn/issues/141) used: the shipped app,
//! varied behind this module, switched from Settings. **The switcher is on Settings and not on
//! Review** because a control added to the review screen changes the thing being judged — and it
//! sits directly under the Settings heading, **above** the Appearance control, for the reason
//! [`switcher`] records: Appearance's sentence wraps at 560 and moves everything below it by 17px.
//!
//! # The finding this prototype opened with, before any candidate was drawn
//!
//! **The prompt already moves 42px when the card is revealed**, at both judging widths.
//!
//! `surface::card` centres its content on the card's centre line, and the content grows at the
//! reveal — by the answer face, the two 24px face gaps and the hairline, less the badge line that
//! arrives with them. So the prompt is centred in 300px before the tap and in the top half after
//! it, and it jumps **−41.9px** to get there. Measured through `run_ui`, not read off the source:
//! the answer's arrival costs −47.5 and the badge's arrival gives back +5.6.
//!
//! This matters three ways.
//!
//! **It makes the ticket's premise wrong in the direction that helps.** #154 says the card "already
//! cannot" twitch, structurally, because `REVIEW_HEIGHT` is a 300px floor. That is true of the
//! *card* and false of everything drawn inside it. So the reveal does not merely *read* as a
//! jump-cut — it **is** one, under #149's rule in as many words: *motion may change what is on the
//! screen, it may never change where it is.*
//!
//! **It is content-dependent, which is why nobody has seen it.** A card whose content already
//! overflows the 300px budget has no centring space to redistribute, so a paragraph card moves
//! **+4.1px** — the badge line alone. The jump is a property of *short* cards, and the seed
//! collection is six French words. Every capture in this repository is of the fixture that shows it
//! worst, and a still cannot show a jump at all.
//!
//! **It decides what a fade has to be.** An opacity fade laid over today's layout leaves the prompt
//! jumping on frame zero while the answer fades in behind it — the movement the rule forbids,
//! wearing the motion the rule asks for. That is [#143](https://github.com/amin-bf/cairn/issues/143)'s
//! shape exactly: *the rule passes while the values fail*. It is drawn here as candidate **B** so it
//! can be seen rather than argued.
//!
//! # The reveal candidates
//!
//! | | the prompt at the reveal | the answer | the hairline |
//! |---|---|---|---|
//! | **A** today | jumps 42px | appears | appears |
//! | **B** naive fade | jumps 42px | fades up | fades up |
//! | **C** reserved fade | **holds still** | fades up | fades up |
//! | **D** reserved fade, standing hairline | **holds still** | fades up | always drawn |
//! | **E** wipe | **moves smoothly, all 42px** | wipes open | travels down |
//!
//! **C is the rule implemented honestly**: the card lays out as if both faces are present from the
//! first frame, so the prompt is placed once and never re-placed, and the things that arrive arrive
//! by fading. The cost is that an unrevealed short card now has a visibly empty lower half — which
//! is either the reveal invitation #124 wanted *inside* the card, drawn as silence, or a hole. That
//! is a judgement and it is the one C exists to collect.
//!
//! **D asks whether the hairline is part of the answer or part of the card.** ADR-0033 §1 says a
//! card is *one object with two faces divided by a hairline*; if that is true from the first frame,
//! the divider is not something the reveal delivers, and D draws it standing. It also gives the
//! empty half an edge, which may be the difference between a waiting face and a hole.
//!
//! **E is the rule's opponent, and the ticket asked for it by name.** The answer half opens rather
//! than fading, so the hairline travels down the card — motion that changes *where* something is,
//! which #149 §2 forbids. If it wins the sitting, §2 is superseded rather than quietly ignored.
//!
//! # The two knobs
//!
//! **Duration** and **curve**, both live, both read off the sitting rather than proposed. #141's
//! finding was that *judging a distance wants a knob, not a menu*, and a duration is a distance —
//! so the duration is dragged and the number is read off afterwards, in the same unit the ADR would
//! name it in. It starts at **200ms**, which is `Style::animation_time`'s stock 0.2 and therefore
//! the value the app would inherit by saying nothing.
//!
//! The curve is a menu rather than a knob because it is not a distance: egui's parameter is a bare
//! `fn(f32) -> f32` with no closures and no cubic-Bézier constructor, so the reachable set is the
//! twenty-two functions in `emath::easing` and the question is which of them, not where between two.
//!
//! # The overlay candidates
//!
//! The app's only overlay is three `ComboBox`es on Notes, and all three of its materials are
//! currently unchosen: `window_fill` is assigned `panel_fill` in both themes, so an open combo box
//! is drawn in **exactly the page colour**; `window_stroke` is never assigned, so it is stock grey
//! (60 in dark, 190 in light); both shadows are `NONE`.
//!
//! | | fill | edge | shadow |
//! |---|---|---|---|
//! | **1** today | the page | stock grey | none |
//! | **2** edge only | the page | on the ramp | none |
//! | **3** shadow only | the page | on the ramp | chosen |
//! | **4** rise | one rung **off** the page, away from the card | on the ramp | none |
//! | **5** rise and shadow | one rung off the page | on the ramp | chosen |
//!
//! **The rise goes the opposite way from the card, and that is the whole proposal.** ADR-0033 cuts
//! a card *into* the page — depth is subtractive everywhere permanent — so the one surface that is
//! *temporarily on top* is the one surface that rises. In dark that is up the ramp; in light it is
//! toward white, which is the direction ADR-0036 §2 found scarce for controls and which a popup
//! needs far less of, because it is not competing with a card for the eye.
//!
//! **The shadow is a knob per theme, and it has to be.** Stock's own defaults say a darkening is not
//! one gesture: identical offset and blur, `from_black_alpha(96)` in dark against
//! `from_black_alpha(25)` in light — a 4× difference nobody in this repo has decided. A shadow is a
//! darkening, and a darkening on a page two rungs off black has almost nowhere to go. So the alpha
//! is dragged separately in each theme and the two numbers are read off, exactly the way #143 found
//! that the same `weak_text_alpha` weighs differently on a light ground: **differ in mechanism,
//! agree in weight**.
//!
//! # The measurement
//!
//! [#123](https://github.com/amin-bf/cairn/issues/123) computed that tessellation is not cached
//! between frames, so a 0.2s transition is roughly twelve full layout-and-tessellation passes over
//! the whole viewport — and said explicitly that nothing was run. [`report_frame`] runs it:
//! `eframe`'s own `cpu_usage` for every frame, split into the frames where an animation is in
//! flight and the frames where none is, printed to stdout as the sitting goes. `AGENTS.md`'s
//! *verify on the real thing* rule makes the figure a measurement rather than arithmetic; the
//! handset half of it belongs to [#126](https://github.com/amin-bf/cairn/issues/126).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

// --- axis one: the reveal ------------------------------------------------------------------------

/// How the answer arrives. See the module header for the table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    /// **A** — what the application draws today: the answer appears between two frames, and the
    /// prompt jumps 42px to make room for it.
    JumpCut,
    /// **B** — an opacity fade laid over today's layout. The answer fades up; the prompt still
    /// jumps. Drawn so the trap is visible rather than described.
    NaiveFade,
    /// **C** — both faces' space is reserved from the first frame, so nothing is re-placed; the
    /// answer, the hairline and the badge fade up into room that was always theirs.
    ReservedFade,
    /// **D** — [`Reveal::ReservedFade`] with the hairline standing from the first frame, so the
    /// card reads as one object with two faces before either is turned.
    ReservedFadeStandingRule,
    /// **E** — the answer half wipes open. The hairline travels, which #149 §2 forbids.
    Wipe,
}

impl Reveal {
    /// The cycle order, which is also the order they are judged in.
    pub const ALL: [Reveal; 5] = [
        Reveal::JumpCut,
        Reveal::NaiveFade,
        Reveal::ReservedFade,
        Reveal::ReservedFadeStandingRule,
        Reveal::Wipe,
    ];

    /// One line for the switcher — what the eye should check on this candidate.
    pub fn label(self) -> &'static str {
        match self {
            Reveal::JumpCut => "A — today: the answer appears, the prompt jumps 42px",
            Reveal::NaiveFade => {
                "B — fade, today's layout: the answer fades, the prompt still jumps"
            }
            Reveal::ReservedFade => "C — fade, room reserved: nothing moves",
            Reveal::ReservedFadeStandingRule => "D — C, with the hairline standing from the start",
            Reveal::Wipe => "E — the answer half wipes open (the hairline travels)",
        }
    }

    /// Whether the card knows about the answer before the reveal.
    ///
    /// For **C** and **D** that is what stops the prompt being re-placed: the room is kept from the
    /// first frame. For **E** it is not — the wipe *interpolates* the placement rather than fixing
    /// it — but the answer still has to be laid out before it can be uncovered, so the wipe needs
    /// the same knowledge for the opposite purpose.
    pub fn reserves_room(self) -> bool {
        matches!(
            self,
            Reveal::ReservedFade | Reveal::ReservedFadeStandingRule | Reveal::Wipe
        )
    }

    /// Whether the hairline is drawn before the answer arrives.
    pub fn standing_rule(self) -> bool {
        matches!(self, Reveal::ReservedFadeStandingRule)
    }

    /// Whether the answer arrives by opacity.
    pub fn fades(self) -> bool {
        matches!(
            self,
            Reveal::NaiveFade | Reveal::ReservedFade | Reveal::ReservedFadeStandingRule
        )
    }

    /// Whether the answer arrives by the half opening.
    pub fn wipes(self) -> bool {
        matches!(self, Reveal::Wipe)
    }
}

/// **C** — the sitting opens on the candidate that implements the rule honestly.
static REVEAL: AtomicUsize = AtomicUsize::new(2);

/// The candidate being drawn this frame.
pub fn reveal() -> Reveal {
    Reveal::ALL[REVEAL.load(Ordering::Relaxed).min(Reveal::ALL.len() - 1)]
}

/// Choose the candidate. Called from the Settings switcher only.
pub fn set_reveal(candidate: Reveal) {
    let i = Reveal::ALL
        .iter()
        .position(|r| *r == candidate)
        .unwrap_or(0);
    REVEAL.store(i, Ordering::Relaxed);
}

// --- the duration knob ---------------------------------------------------------------------------

/// **200ms** — `Style::animation_time`'s stock 0.2, which is the value the application inherits by
/// saying nothing, and therefore the honest starting point for a knob that exists to move it.
static DURATION_MS: AtomicU32 = AtomicU32::new(200);

/// The transition's duration, in seconds — the unit `Style::animation_time` is in.
pub fn duration() -> f32 {
    DURATION_MS.load(Ordering::Relaxed) as f32 / 1000.0
}

/// The transition's duration in milliseconds, which is the unit the readout and the ADR use.
pub fn duration_ms() -> u32 {
    DURATION_MS.load(Ordering::Relaxed)
}

/// The knob's range. Zero is included on purpose — dragging to the left end is how the sitting
/// compares against today without leaving the candidate, and a duration of zero *is* candidate A's
/// timing wearing C's layout, which is a distinction worth being able to see.
pub const DURATION_MAX_MS: u32 = 600;

/// Move the duration by a drag's horizontal delta. One pixel is one millisecond, which makes the
/// knob's whole range 600px — wider than the 584px column at the judging width, so the drag is
/// continued rather than completed in one sweep. That is deliberate: a finer scale is what a
/// judgement about 40ms needs.
pub fn drag_duration(delta_x: f32) {
    if delta_x != 0.0 {
        let next = (duration_ms() as f32 + delta_x).clamp(0.0, DURATION_MAX_MS as f32);
        DURATION_MS.store(next as u32, Ordering::Relaxed);
    }
}

// --- the curve menu ------------------------------------------------------------------------------

/// An easing function, in the only shape egui accepts one: a bare `fn` pointer, with no closures and
/// no cubic-Bézier constructor. That is why the curve is a menu and not a knob.
pub type Curve = fn(f32) -> f32;

/// The reachable curves — a menu over `emath::easing` rather than a knob over a Bézier's control
/// points, for the reason [`Curve`] records.
///
/// Six of the twenty-two, and the omissions are not laziness. `*_in` curves start slow and end fast,
/// which reads as a thing being *thrown* at you; `back_*` and `bounce_*` overshoot, which is motion
/// that changes where something is even when the thing is only an opacity. What is left is the set
/// that decelerates or is symmetric.
pub const CURVES: [(&str, Curve); 6] = [
    (
        "linear — what animate_bool uses",
        egui::emath::easing::linear,
    ),
    ("quadratic_out", egui::emath::easing::quadratic_out),
    (
        "cubic_out — what animate_bool_responsive uses",
        egui::emath::easing::cubic_out,
    ),
    ("sin_out", egui::emath::easing::sin_out),
    ("quadratic_in_out", egui::emath::easing::quadratic_in_out),
    ("cubic_in_out", egui::emath::easing::cubic_in_out),
];

/// **cubic_out** — the one curve egui itself picks anywhere, and therefore the starting point a
/// choice has to beat rather than a preference of this prototype's.
static CURVE: AtomicUsize = AtomicUsize::new(2);

/// The curve being applied this frame.
pub fn curve() -> Curve {
    CURVES[CURVE.load(Ordering::Relaxed).min(CURVES.len() - 1)].1
}

/// The curve's name, for the readout.
pub fn curve_name() -> &'static str {
    CURVES[CURVE.load(Ordering::Relaxed).min(CURVES.len() - 1)].0
}

/// Choose the curve. Called from the Settings switcher only.
pub fn set_curve(i: usize) {
    CURVE.store(i.min(CURVES.len() - 1), Ordering::Relaxed);
}

// --- the hold ---------------------------------------------------------------------------------

/// A transition pinned at a fixed fraction, or `None` when it runs.
///
/// **This exists because a still cannot photograph a jump.** The whole reason the reveal is judged
/// as a running sitting is that its defect lives between two frames — but a write-up still has to
/// *show* someone what the middle of a transition looks like, and the capture harness settles for
/// four seconds before it shoots. Holding `t` makes the middle frame reachable by a storyboard, so
/// the readme can carry a picture of the thing rather than a description of it.
///
/// 101 means *not held*, which is outside the 0..=100 the knob produces.
static HOLD: AtomicU32 = AtomicU32::new(101);

/// The held fraction, or `None` when the transition runs at its own speed.
pub fn held() -> Option<f32> {
    let raw = HOLD.load(Ordering::Relaxed);
    (raw <= 100).then(|| raw as f32 / 100.0)
}

/// Set or clear the hold. Called from the Settings switcher only.
pub fn set_hold(percent: Option<u32>) {
    HOLD.store(percent.map_or(101, |p| p.min(100)), Ordering::Relaxed);
}

/// How far through the reveal this frame is, eased.
///
/// # Both of #123's silent-failure traps live in this one call, and they pull opposite ways
///
/// egui's animation state is an `IdMap<BoolAnim>` that is **inserted into and never removed from** —
/// there is no eviction anywhere in `animation_manager.rs`, and `Context::clear_animations` drops
/// the whole map or nothing. It also **snaps on first sight**: the `None` arm returns the *target*
/// rather than starting from the other end. Both are exactly as #123 described them, confirmed at
/// the source rather than inferred, and together they make the choice of id the whole design.
///
/// **Key it on the card and the leak is unbounded.** A `BoolAnim` is never freed, so one entry
/// accumulates per card reviewed, for the life of the process. It is small — twelve bytes and a
/// `u64` key — and on a thousand-card day it is tens of kilobytes, which is nothing. It is still
/// growth with no ceiling, in a loop the application runs all day, and it was reached by accident
/// rather than chosen.
///
/// **Key it on the `Ui` and the state is too stable.** It survives the card changing, so grading
/// leaves `revealed` false with the value still at 1.0 and the **next card's answer is drawn, fading
/// out, for the whole duration** — a card nobody has turned over, showing its answer. The first
/// gated measurement run counted 24 animating frames per reveal where twelve were expected, which is
/// what exposed it: a second transition was running on every grade, and nothing failed.
///
/// # So it is keyed on the `Ui` and **reset** when the card changes
///
/// `card_changed` is the caller's own `s.shown != Some(offered.card)`, which the review screen
/// already computes, and on that frame the animation is snapped with an `animation_time` of **zero**.
/// That is not a hack: with a zero duration the manager's step is `last_value ± elapsed/0.0`, which
/// is infinite, which fails its own `is_finite()` check and falls through to the target. Snapping is
/// the documented behaviour of a zero-length animation, reached deliberately here.
///
/// One id, O(1) memory, and a new card that starts unrevealed with no fade. Both traps discharged
/// rather than traded against each other.
pub fn progress(ui: &egui::Ui, card_changed: bool, revealed: bool) -> f32 {
    if let Some(held) = held() {
        // A held frame is eased too, so what the still shows is the frame the sitting would see at
        // that fraction of the way through rather than a linear slice of it.
        return if revealed { curve()(held) } else { 0.0 };
    }
    if reveal() == Reveal::JumpCut {
        return f32::from(revealed);
    }
    let id = ui.id().with("proto-154-reveal");
    if card_changed {
        ui.ctx().animate_bool_with_time(id, false, 0.0);
    }
    ui.ctx()
        .animate_bool_with_time_and_easing(id, revealed, duration(), curve())
}

// --- axis two: the overlay's material --------------------------------------------------------------

/// What an open combo box is made of. See the module header for the table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// **1** — what the application draws today: the page's own colour, stock egui's grey hairline,
    /// no shadow.
    Today,
    /// **2** — the page's colour, an edge chosen off the ramp.
    EdgeOnly,
    /// **3** — the page's colour, a chosen edge, and a shadow. Tests whether the shadow alone
    /// carries the separation.
    ShadowOnly,
    /// **4** — a fill one rung *off* the page in the direction the card does not go, and a chosen
    /// edge. Tests whether the rise alone carries it.
    Rise,
    /// **5** — the rise, the edge and the shadow together.
    RiseAndShadow,
}

impl Overlay {
    /// The cycle order, which is also the order they are judged in.
    pub const ALL: [Overlay; 5] = [
        Overlay::Today,
        Overlay::EdgeOnly,
        Overlay::ShadowOnly,
        Overlay::Rise,
        Overlay::RiseAndShadow,
    ];

    /// One line for the switcher.
    pub fn label(self) -> &'static str {
        match self {
            Overlay::Today => "1 — today: the page's colour, a stock hairline, no shadow",
            Overlay::EdgeOnly => "2 — the page's colour, an edge on the ramp",
            Overlay::ShadowOnly => "3 — the page's colour, an edge, and a shadow",
            Overlay::Rise => "4 — a fill that rises off the page, and an edge",
            Overlay::RiseAndShadow => "5 — the rise, the edge and the shadow",
        }
    }

    /// Whether the popup's fill leaves the page's colour.
    pub fn rises(self) -> bool {
        matches!(self, Overlay::Rise | Overlay::RiseAndShadow)
    }

    /// Whether the popup casts a shadow.
    pub fn casts(self) -> bool {
        matches!(self, Overlay::ShadowOnly | Overlay::RiseAndShadow)
    }

    /// Whether the edge is chosen rather than left on stock grey.
    pub fn chosen_edge(self) -> bool {
        !matches!(self, Overlay::Today)
    }
}

/// **5** — the sitting opens on the full proposal, so the first thing seen is the thing being
/// argued for and every switch after that is a subtraction.
static OVERLAY: AtomicUsize = AtomicUsize::new(4);

/// The material being drawn this frame.
pub fn overlay() -> Overlay {
    Overlay::ALL[OVERLAY.load(Ordering::Relaxed).min(Overlay::ALL.len() - 1)]
}

/// Choose the material. Called from the Settings switcher only.
pub fn set_overlay(candidate: Overlay) {
    let i = Overlay::ALL
        .iter()
        .position(|o| *o == candidate)
        .unwrap_or(0);
    OVERLAY.store(i, Ordering::Relaxed);
}

// --- the shadow knobs, one per theme ---------------------------------------------------------------

/// Stock egui's dark alpha. Kept as the opening value so the knob's first position is the thing a
/// palette that said nothing would inherit.
static SHADOW_ALPHA_DARK: AtomicU32 = AtomicU32::new(96);

/// Stock egui's light alpha — a quarter of dark's, at identical offset and blur, which is the 4×
/// nobody in this repo has decided.
static SHADOW_ALPHA_LIGHT: AtomicU32 = AtomicU32::new(25);

/// The shadow's alpha in the theme currently drawn.
pub fn shadow_alpha(dark: bool) -> u8 {
    let raw = if dark {
        SHADOW_ALPHA_DARK.load(Ordering::Relaxed)
    } else {
        SHADOW_ALPHA_LIGHT.load(Ordering::Relaxed)
    };
    raw.min(255) as u8
}

/// Move the current theme's shadow alpha by a drag's horizontal delta, at one alpha step per pixel.
pub fn drag_shadow_alpha(dark: bool, delta_x: f32) {
    if delta_x == 0.0 {
        return;
    }
    let slot = if dark {
        &SHADOW_ALPHA_DARK
    } else {
        &SHADOW_ALPHA_LIGHT
    };
    let next = (shadow_alpha(dark) as f32 + delta_x).clamp(0.0, 255.0);
    slot.store(next as u32, Ordering::Relaxed);
}

/// Write the chosen material into the **active** theme's visuals, every frame.
///
/// It is a per-frame write because the candidate changes at runtime and `theme::install` runs once,
/// at construction. Nothing shipping would do this: the decision this collects is three assignments
/// inside `cairn_dark` and `cairn_light`, with no runtime branch at all — which is what #154 asks
/// for and what `theme::card_divider`'s branch is the one accepted exception to.
pub fn apply_overlay(ctx: &egui::Context) {
    let candidate = overlay();
    ctx.style_mut_of(ctx.theme(), |style| {
        let v = &mut style.visuals;
        let dark = v.dark_mode;
        v.window_fill = crate::theme::popup_fill(v, candidate.rises());
        if candidate.chosen_edge() {
            v.window_stroke = crate::theme::popup_stroke(v);
        }
        v.popup_shadow = if candidate.casts() {
            // Stock's offset and blur, whose *shape* nothing in this ticket disputes; only the
            // darkening is in question, and only the darkening is on a knob.
            egui::epaint::Shadow {
                offset: [6, 10],
                blur: 8,
                spread: 0,
                color: egui::Color32::from_black_alpha(shadow_alpha(dark)),
            }
        } else {
            egui::epaint::Shadow::NONE
        };
    });
}

// --- the measurement -------------------------------------------------------------------------------

/// Frames counted, and the microseconds they cost, split by whether an animation was in flight.
static STILL_FRAMES: AtomicU32 = AtomicU32::new(0);
static STILL_MICROS: AtomicU32 = AtomicU32::new(0);
static BUSY_FRAMES: AtomicU32 = AtomicU32::new(0);
static BUSY_MICROS: AtomicU32 = AtomicU32::new(0);
/// Whether the frame just drawn had a transition in flight, written by [`progress`]' caller.
static ANIMATING: AtomicBool = AtomicBool::new(false);
/// Whether the frame just drawn was a **review card** frame at all.
///
/// **The measurement is worthless without this gate, and the first run proved it.** Counting every
/// frame put Settings — which draws the rendering specimen, every script in three families — into
/// the *still* bucket and Review into the *animating* one, and reported that animating frames were
/// **twice as cheap** as still ones. The two buckets have to be the same screen or the difference
/// between them is a difference of screens.
static ON_REVIEW_CARD: AtomicBool = AtomicBool::new(false);

/// Record that this frame drew a review card, and whether a transition was in flight.
pub fn note_frame(animating: bool) {
    ON_REVIEW_CARD.store(true, Ordering::Relaxed);
    ANIMATING.store(animating, Ordering::Relaxed);
}

/// Fold `eframe`'s own per-frame CPU figure into the running split, and print a line every 120
/// frames of each kind.
///
/// **`cpu_usage` is `App::ui` plus rendering, excluding the vsync wait** — eframe's own words — so
/// it is the layout-and-tessellation cost #123 computed, measured rather than derived. The split is
/// the whole point: the difference between the two means is what a transition costs, and #123's
/// figure is that difference times the frames a 0.2s transition spans.
pub fn report_frame(cpu_seconds: Option<f32>) {
    let Some(seconds) = cpu_seconds else { return };
    // `cpu_usage` describes the frame *before* this one, and so does the flag: it is written while
    // that frame draws and cleared here, so the two always describe the same frame.
    if !ON_REVIEW_CARD.swap(false, Ordering::Relaxed) {
        return;
    }
    let micros = (seconds * 1_000_000.0) as u32;
    let (frames, total, kind) = if ANIMATING.load(Ordering::Relaxed) {
        (&BUSY_FRAMES, &BUSY_MICROS, "animating")
    } else {
        (&STILL_FRAMES, &STILL_MICROS, "still")
    };
    let n = frames.fetch_add(1, Ordering::Relaxed) + 1;
    let sum = total.fetch_add(micros, Ordering::Relaxed) + micros;
    if n % 20 == 0 {
        println!(
            "proto-154 cpu: {kind} n={n} mean={:.3}ms",
            sum as f64 / n as f64 / 1000.0
        );
    }
}

/// The running split, for the readout drawn on Settings.
pub fn cpu_summary() -> String {
    let mean = |frames: &AtomicU32, micros: &AtomicU32| {
        let n = frames.load(Ordering::Relaxed);
        if n == 0 {
            return f64::NAN;
        }
        micros.load(Ordering::Relaxed) as f64 / n as f64 / 1000.0
    };
    let still = mean(&STILL_FRAMES, &STILL_MICROS);
    let busy = mean(&BUSY_FRAMES, &BUSY_MICROS);
    format!(
        "cpu/frame — still {still:.3}ms over {} frames, animating {busy:.3}ms over {} frames",
        STILL_FRAMES.load(Ordering::Relaxed),
        BUSY_FRAMES.load(Ordering::Relaxed),
    )
}

// --- the switcher ----------------------------------------------------------------------------------

/// A horizontal drag surface with a live readout, drawn as one full-width row.
///
/// #141's finding, applied a second time: *judging a distance wants a knob, not a menu*. Both
/// distances on this ticket — a duration and a shadow's weight — are dragged and read off, so the
/// sitting produces a number in the unit the ADR names rather than a preference between three
/// photographs.
fn knob(ui: &mut egui::Ui, readout: &str) -> f32 {
    let height = crate::typography::BODY * 2.4;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(2),
        crate::theme::control_fill(ui.visuals()),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        readout,
        egui::FontId::proportional(crate::typography::SMALL),
        ui.visuals().text_color(),
    );
    response.drag_delta().x
}

/// The switcher, drawn on Settings **directly under the heading**, above everything else.
///
/// Deliberately ugly and deliberately labelled: it is a harness control and nothing about it is
/// being judged.
///
/// **Above Appearance, and that position is forced rather than preferred.** The Appearance control's
/// explanatory sentence wraps to two lines at 560 and one at 1280, so everything below it sits 17px
/// lower at the narrow width — measured, not feared. Anything the storyboard must click at both
/// judging widths therefore has to sit *above* that sentence, and only the one-word heading can be
/// above this. It is the same trap #143 hit from the other side, where the control that moved was
/// Appearance itself.
///
/// Every row here is a fixed height at both widths: the two knobs paint unwrapped text into a rect
/// they size themselves, and every label is short enough to hold one line in a 504px column. The one
/// control that cannot promise that — the frame-cost readout, whose counters grow — is
/// [`cpu_readout`], drawn last, where nothing is below it to move.
pub fn switcher(ui: &mut egui::Ui) {
    use crate::{field_label, spacing};

    field_label(
        ui,
        "PROTOTYPE #154 — motion and elevation. Pick a reveal, then go to Review.",
    );
    ui.add_space(spacing::gap(1));

    let current = reveal();
    for candidate in Reveal::ALL {
        if ui
            .selectable_label(current == candidate, crate::text(ui, candidate.label()))
            .clicked()
        {
            set_reveal(candidate);
        }
    }

    ui.add_space(spacing::gap(2));
    let delta = knob(
        ui,
        &format!("drag — duration {}ms of {DURATION_MAX_MS}", duration_ms()),
    );
    drag_duration(delta);

    ui.add_space(spacing::gap(2));
    field_label(
        ui,
        "hold the transition — for stills of the middle frame, which a settled capture cannot reach",
    );
    ui.add_space(spacing::gap(1));
    spacing::row(ui, 1, |ui| {
        for held_at in [None, Some(0), Some(25), Some(50), Some(75), Some(100)] {
            let label = match held_at {
                None => "running".to_owned(),
                Some(p) => format!("{p}%"),
            };
            if ui
                .selectable_label(
                    held() == held_at.map(|p| p as f32 / 100.0),
                    crate::text(ui, &label),
                )
                .clicked()
            {
                set_hold(held_at);
            }
        }
    });

    ui.add_space(spacing::gap(2));
    field_label(ui, &format!("curve — {}", curve_name()));
    ui.add_space(spacing::gap(1));
    for (i, (name, _)) in CURVES.iter().enumerate() {
        if ui
            .selectable_label(curve_name() == *name, crate::text(ui, name))
            .clicked()
        {
            set_curve(i);
        }
    }

    ui.add_space(spacing::gap(3));
    field_label(
        ui,
        "The overlay's material — pick one, then open a dropdown on Notes.",
    );
    ui.add_space(spacing::gap(1));
    let current = overlay();
    for candidate in Overlay::ALL {
        if ui
            .selectable_label(current == candidate, crate::text(ui, candidate.label()))
            .clicked()
        {
            set_overlay(candidate);
        }
    }

    ui.add_space(spacing::gap(2));
    let dark = ui.visuals().dark_mode;
    let theme = if dark { "dark" } else { "light" };
    let delta = knob(
        ui,
        &format!(
            "drag — {theme} shadow alpha {} (stock: 96 dark, 25 light)",
            shadow_alpha(dark)
        ),
    );
    drag_shadow_alpha(dark, delta);
}

/// The frame-cost readout, drawn at the **bottom** of Settings rather than in the switcher.
///
/// **It is the one control here whose height is not fixed**, because its text carries running
/// counters that grow, and at 560 the line is already nearly the full column — so a wrap would move
/// everything below it by 17px and a storyboard aimed at the switcher would silently miss. That is
/// the failure this map has now paid for twice (#143's seven valid captures of the wrong theme), and
/// the cheapest defence is to put the variable-height thing last, where nothing is below it.
pub fn cpu_readout(ui: &mut egui::Ui) {
    use crate::{controls, field_label, spacing};

    field_label(ui, &cpu_summary());
    ui.add_space(spacing::gap(1));
    if controls::wide(ui, "print the cpu split to stdout").clicked() {
        println!("proto-154 {}", cpu_summary());
    }
}

/// The two traps #123 named, pinned against the renderer rather than against this module.
///
/// **These are the tests the shipping change owes**, written here first because the mechanism they
/// describe is the one the sitting is about to judge, and a prototype running on a broken mechanism
/// collects a verdict about the wrong thing. They assert on `egui::Context` directly, so they keep
/// meaning after `proto` is gone: what they pin is *what the renderer does*, which is the thing a
/// later egui release could change underneath a decision recorded here.
#[cfg(test)]
mod tests {
    use egui::{Context, Id};

    /// Step a context forward one frame so the animation manager sees time pass.
    fn frame(ctx: &Context) {
        let input = egui::RawInput {
            predicted_dt: 1.0 / 60.0,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |_| {});
    }

    /// **Trap one: an id that changes between frames produces no motion at all.**
    ///
    /// The manager's miss arm inserts the *target* and returns it, so a fresh id is always fully on
    /// or fully off. Nothing fails, nothing is logged, and the screen simply does not animate —
    /// which on this ticket would present as "the fade does not work" with a correct-looking fade in
    /// the source.
    #[test]
    fn an_unstable_id_produces_no_motion() {
        let ctx = Context::default();
        for i in 0..10 {
            frame(&ctx);
            let value = ctx.animate_bool_with_time(Id::new(("unstable", i)), true, 0.2);
            assert_eq!(
                value, 1.0,
                "a first-sight id snaps to its target; frame {i} gave {value}"
            );
        }
    }

    /// **The same call with a stable id does animate**, which is what makes the test above a
    /// statement about the id rather than about the duration.
    #[test]
    fn a_stable_id_animates() {
        let ctx = Context::default();
        let id = Id::new("stable");
        frame(&ctx);
        assert_eq!(ctx.animate_bool_with_time(id, false, 0.2), 0.0);
        frame(&ctx);
        let mid = ctx.animate_bool_with_time(id, true, 0.2);
        assert!(
            0.0 < mid && mid < 1.0,
            "a stable id must be partway through, not snapped; got {mid}"
        );
    }

    /// **Trap two: the state is never evicted**, so an id animated once holds its value for the life
    /// of the process rather than being forgotten when nothing draws it.
    ///
    /// `AnimationManager`'s map is insert-only — there is no eviction in the whole module, and
    /// `Context::clear_animations` drops all of it or none. That cannot be observed by counting
    /// entries from outside, so it is observed by its consequence: an id left alone for many frames
    /// **resumes** from where it was, which is only possible if it was kept.
    ///
    /// This is why the reveal is keyed on one id and *reset*, rather than keyed per card: per card
    /// would be one retained entry for every card ever reviewed.
    #[test]
    fn animation_state_is_retained_indefinitely() {
        let ctx = Context::default();
        let id = Id::new("retained");
        frame(&ctx);
        ctx.animate_bool_with_time(id, false, 0.2);
        frame(&ctx);
        let partway = ctx.animate_bool_with_time(id, true, 0.2);
        assert!(0.0 < partway && partway < 1.0);

        // Many frames in which nothing touches this id at all.
        for _ in 0..600 {
            frame(&ctx);
        }
        frame(&ctx);
        let resumed = ctx.animate_bool_with_time(id, true, 0.2);
        assert!(
            resumed > partway,
            "the id must resume from its kept value rather than snap; {partway} -> {resumed}"
        );
        assert!(
            resumed < 1.0,
            "600 untouched frames must not have advanced it to the target — the manager steps only \
             when asked, so a retained entry is frozen rather than stale; got {resumed}"
        );
    }

    /// **The reset the shipping change depends on.** A zero `animation_time` snaps a *stable* id to
    /// its target, which is how one id can serve every card without carrying the previous card's
    /// reveal into the next one.
    ///
    /// It works because the manager's step divides by `animation_time`: at zero the result is
    /// infinite, fails the `is_finite()` check, and falls through to the target. That is a
    /// documented path and not an accident, but it is the kind of thing an egui release could
    /// tighten, so it is pinned here rather than trusted.
    #[test]
    fn a_zero_duration_snaps_a_stable_id() {
        let ctx = Context::default();
        let id = Id::new("snapped");
        frame(&ctx);
        ctx.animate_bool_with_time(id, false, 0.2);
        frame(&ctx);
        let partway = ctx.animate_bool_with_time(id, true, 0.2);
        assert!(0.0 < partway && partway < 1.0);

        frame(&ctx);
        let snapped = ctx.animate_bool_with_time(id, false, 0.0);
        assert_eq!(
            snapped, 0.0,
            "a zero-length animation must land on its target immediately; got {snapped}"
        );
    }
}
