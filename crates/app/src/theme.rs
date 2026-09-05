//! The palette. **The only module in this crate that names a colour** (ADR-0030 §1); every screen
//! keeps reading the *ambient* `ui.visuals()` — `text_color()`, `weak_text_color()`,
//! `hyperlink_color`, the selection accent — exactly as it did against stock egui, so adopting the
//! palette changes *where the values come from*, not a single call site. A colour literal anywhere
//! outside this module is the defect.
//!
//! egui's stock theme is what shipped through the walking skeleton: legible but loud — pure-grey
//! neutrals, a saturated blue selection, `#ff0000` for error. This is the same structure in stone —
//! cool slate neutrals and four desaturated accents, flat fills, 2px corners, no shadow — so nothing
//! above this module has to know it exists.
//!
//! # Both slots are filled, and the user chooses (ADR-0036 §3, superseding ADR-0030 §2)
//!
//! ADR-0030 §2 pinned dark and dropped OS-theme following, for exactly one reason: only a dark
//! palette was drawn, so following the OS would silently hand a light-preferring user **stock
//! egui** — the 5.12:1 body §3 exists to leave behind — reached by omission rather than by anyone
//! choosing it. A light palette now exists ([`cairn_light`]), which discharges that reason, and
//! ADR-0036 §3 restores following as a **user preference** rather than as OS obedience.
//!
//! [`install`] therefore does three things, and the first is the one that was a bug before:
//!
//! 1. It writes each palette into **its own slot** — `set_visuals_of(Theme::Dark, …)` and
//!    `set_visuals_of(Theme::Light, …)`, never the untargeted `set_visuals`, which writes to
//!    *whichever slot is active when it is called*. That setter is now doubly wrong: it would fill
//!    one slot and leave the other on stock, and which one depends on what the OS happened to
//!    prefer at construction.
//! 2. It **fills both**, so no reachable theme is stock. This is what ADR-0030 §2's refusal becomes
//!    once light exists: the outcome it forbade was *an unfilled slot*, not *following the OS*, and
//!    the way to honour it is to leave no slot unfilled rather than to leave no slot reachable.
//! 3. It applies the stored [`ThemeChoice`] — `System`, `Light` or `Dark`, defaulting to `System`.
//!    The preference is **device-local** and never syncs (`Collection::theme_preference`): a desktop
//!    under a lamp and a handset in bed want opposite answers.
//!
//! **The choice is the decision; `ThemePreference` is the mechanism.** A native client that never
//! links egui honours ADR-0036 §3 by offering the same three options against its own platform
//! setting — the rule is stated in terms of what the *user* picked, not of what egui exposes.
//!
//! Unlike `fonts`, this needs **no first-frame deferral**: visuals allocate no texture, so ADR-0012
//! §8's `CreationContext` hazard does not reach them, and setting them at construction avoids a frame
//! of the wrong theme. It is called from [`crate::CairnApp::new`].
//!
//! # The light palette is re-derived, never re-hued (ADR-0036 §1, §2)
//!
//! The three control weights — 1.099:1 ordinary, 1.121:1 the card, 1.293:1 `primary` — were every
//! one of them measured against a page near the **bottom** of the ramp, and they use **both
//! directions**: the card is a well *below* the page and the two controls rise *above* it. A light
//! page sits near the top, which inverts which direction is scarce — and the scarce direction is the
//! one dark puts **two** of its three weights in.
//!
//! **How scarce depends on where the page sits, and the honest numbers are these.** Above a
//! paper-white page like the `#f2f1ed` the design project carried as a placeholder there is
//! **1.130:1** in total — less than the *ordinary* control's 1.099:1 leaves room to work with, and
//! nowhere near the primary's 1.293:1, so on that page a `primary` lighter than the page does not
//! exist at any hue. Above the `#dee2e3` this palette chose there is **1.305:1**, which does clear
//! 1.293 — by **0.012**, with the primary a hair off pure white. So the mirrored construction is
//! impossible on a paper page and merely unbuildable on this one.
//!
//! It was not rejected on that arithmetic, though: all three constructions were drawn and judged
//! blurred, which is ADR-0033 §3's own instrument, and **ink won by eye**. The arithmetic explains
//! why it had to be considered at all.
//!
//! So the light palette places every value at the ratio the dark palette gives the *same role
//! against the same reference*, moving **away from the page** — which on a light page means
//! downward for all three fills. Nothing here is picked by eye; [`cairn_light`] names what the
//! arithmetic produced and the tests re-derive it.
//!
//! # The contrast floor (ADR-0030 §3)
//!
//! The floor is **7:1** — WCAG AAA for body text. It binds **text against the surface it is drawn
//! on**, in **both** themes: `text_pairs_clear_the_contrast_floor` holds every reading-text pair the
//! app draws, for dark and light alike — body on the page (13.34:1 dark, 13.29:1 light, against
//! stock's 5.12:1), on a card, on each control weight, and over the one reachable accent, the
//! selection fill.
//!
//! **§3's stated premise is no longer true, and the number outlived it.** ADR-0030 §3 argued 7:1
//! from the small text style being 9px, where WCAG AA's 4.5:1 is the marginal case. #132 raised
//! small to **12px** and #125 then judged that tier legible at arm's length on a handset at low
//! brightness, so nothing is holding the floor up from the legibility side any more. It is kept
//! here because no evidence pushes it either way, not because the original argument survives —
//! ADR-0036 §4 records that, and reopening the *number* needs no permission.
//!
//! **The tightest reading pair in either theme is light's body-on-`primary`, at 7.06:1.** It clears
//! the floor by 0.06 and `the_light_primary_is_the_tightest_reading_pair` pins the figure rather
//! than only the inequality, so a later nudge to either colour fails loudly instead of slipping
//! under. That margin is the ink construction's price: the light `primary` is 1.883:1 *below* the
//! page where dark's is 1.293:1 *above* it, so it eats into the body text's headroom in a way dark's
//! never could (ADR-0036 §2).
//!
//! **Weak text is the one text pair below the floor, and it is left there deliberately.** The box
//! badge and field labels read `weak_text_color()` — body gamma-multiplied toward the background —
//! which lands near `#8b979b`, ~5.6:1 on the panel. Lifting it to 7:1 would make the badge *louder*,
//! which is precisely what ADR-0030 §4 forbids ("small, non-interactive footnote … quiet aside"), and
//! would diverge the app from the design system's `--text-weak` token. It is a **pre-existing
//! weakness** — stock's weak text is 5.12:1, so the palette *improves* it (5.12 → 5.6) and never
//! regresses it — of the same kind as the non-text pairs §3 leaves out of scope, not the lone
//! regression §3 records (the hover stroke, lifted back over 3:1 here). `weak_text_is_not_a_regression`
//! pins that it stays at least as legible as stock rather than silently drifting below it. This is in
//! tension with §3's listing weak-text-on-panel among the *bound* pairs; §4's quiet-footnote
//! requirement is the one that governs the colour, and the tension is recorded for the finish pass.
//!
//! **Light has to ask for that weight; dark gets it free** (ADR-0036 §2). `weak_text_alpha` is 0.6,
//! so egui composites 60% of the body colour over the surface behind it. On a dark page that lands
//! near `STONE_9` and ~5.6:1 — the ramp carries one entry for both, and the derivation happens to
//! give §4 exactly what it asked for. On a **light** page the same 60% of a near-black over a light
//! ground lands *much closer to the ground*, around 4.2:1: quieter than dark's badge by an accident
//! of compositing rather than by any decision. So the light visuals set `weak_text_color`
//! explicitly, to the value that measures dark's own **5.59:1** against the light page. The two
//! themes therefore differ in *mechanism* — derived in dark, named in light — and agree on the
//! *weight*, which is the thing ADR-0030 §4 actually decided. `weak_text_carries_the_same_weight_in_
//! both_themes` pins the agreement. Dark is deliberately left derived: naming it there would change
//! shipped pixels for no reason.

use egui::{Color32, CornerRadius, Stroke, Theme, ThemePreference, Visuals};

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

// --- stone: the neutral ramp, cool slate with a trace of blue ---
const STONE_0: Color32 = rgb(0x0f1214); // card faces
// **The rung the ramp had numbered and never filled** (#163). `STONE_0` → `STONE_2` was the only
// double step in the whole ramp — a delta of (11, 12, 13) where its neighbours move (7, 8, 9) and
// (4, 4, 4) — because until now nothing had ever needed a value between the well and the page. A
// text field does: ADR-0033 §2 accepted a card and a field sharing `extreme_bg_color` partly on the
// grounds that the two never appear on the same screen, the editor has drawn them side by side in
// both themes since the card landed, and #150 measured the separation at **1.000:1**.
//
// **Placed by thumb, not by arithmetic.** A knob moved the field from the card's fill toward the
// page with a live readout, and it stopped here — one unit per channel off a true midpoint of the
// gap. That the gap was already there, and already numbered, is the finding: #143 had recorded the
// cost of giving a field its own fill as *"it spends a rung of a ramp that had none to spare in
// dark"*, and nothing is spent.
const STONE_1: Color32 = rgb(0x15191b); // text-field wells
const STONE_2: Color32 = rgb(0x1a1e21); // panels and windows
const STONE_3: Color32 = rgb(0x21262a); // faint fills
const STONE_4: Color32 = rgb(0x282e33); // pressed widgets, separators
const STONE_5: Color32 = rgb(0x2c3237); // widgets at rest
const STONE_6: Color32 = rgb(0x363d43); // widgets hovered
// The hovered stroke, at the light-stone rung. The draft put it a rung lower, at `#6d7a80`, which
// measured **2.49:1** against the hovered fill (`STONE_6`) — the one pair the palette turned from
// passing (stock's 3.19:1) to failing, and hover is exactly the state the non-text contrast rule
// covers (ADR-0030 §3). There is no rung between the two that clears 3:1 with margin, so it moves up
// to `#8b979b`; `hover_stroke_clears_three_to_one` pins it. Weak text — derived by egui, never named
// here — lands near this same value, which is why the ramp carried one entry for both.
const STONE_9: Color32 = rgb(0x8b979b); // hovered strokes — lifted from #6d7a80 to clear 3:1
const STONE_10: Color32 = rgb(0xb9c2c3); // text on a widget
const STONE_11: Color32 = rgb(0xe2e6e6); // body text
const QUIET: Color32 = rgb(0x333b40); // strokes at rest

// --- the four desaturated accents ---
const LICHEN: Color32 = rgb(0x6f93a8); // links
const LICHEN_DEEP: Color32 = rgb(0x2a4453); // selection fill
const LICHEN_PALE: Color32 = rgb(0xcfe3ec); // selection stroke and text
const CLAY: Color32 = rgb(0xc2a37a); // warn — warm, never alarming
const ROSE: Color32 = rgb(0xb57e79); // error — softened, never #ff0000

// --- the light ramp: the same stone in daylight (ADR-0036 §1, §2) ---------------------------
//
// **These are outputs, not choices.** Each is the dark palette's ratio for the *same role against
// the same reference*, re-placed away from the light page; `the_light_ramp_is_re_derived_not_re_hued`
// recomputes every one of them from the dark constants and fails if a hand has been laid on any.
// The tint is carried from the shipped ramp rather than re-picked, so the neutrals stay cool slate
// — the warm `#f2f1ed` the design project carried as a placeholder was a re-hue nobody decided.
//
// The three fills all sit **below** the page, which is the whole of why this is not a mirror: on a
// light page there is no room above it for a control to be (module header).
const STONE_L_PAGE: Color32 = rgb(0xdee2e3); // panels and windows — 1.000
const STONE_L_CONTROL: Color32 = rgb(0xd9dddd); // faint fills, an ordinary control — 1.049
const STONE_L_EDGE: Color32 = rgb(0xced2d3); // separators, pressed widgets, a control's edge — 1.168
const STONE_L_CARD: Color32 = rgb(0xc4c8c9); // card faces — 1.292
// **Light mints its own rather than reusing the edge rung, and that was a decision** (#163). The
// same knob position that found dark's `STONE_1` lands here, **four of 255 per channel** from
// `STONE_L_CARD_EDGE`'s neighbour `STONE_L_EDGE` (`#ced2d3`) — near enough that #155's precedent
// applied, where a thumb stopped six away from an existing role and the existing role served.
//
// It was refused here because the two cases differ in *meaning* rather than in distance. #155's
// candidates both meant **quiet ink**, so one of them serving cost nothing. `STONE_L_EDGE` means
// separators, pressed widgets and a control's edge — so reusing it would paint a **resting** text
// field the value of a **pressed** control, which is the category error ADR-0033 §2 already made
// once, arriving one rung over.
//
// **The cost is stated rather than hidden**: light now carries two rungs four units apart, where
// dark's field rung sits alone in a two-step gap. Light's ramp is simply denser here — it has a
// control rung and an edge rung between its page and its card where dark has nothing at all — so
// the same decision is a gap being filled in one theme and a thread being passed in the other.
const STONE_L_FIELD: Color32 = rgb(0xd2d6d7); // text-field wells — 1.122
const STONE_L_HOVER: Color32 = rgb(0xb9bdbe); // widgets hovered — 1.452
const STONE_L_CARD_EDGE: Color32 = rgb(0xa7abac); // a card's edge and divider — 1.776
const STONE_L_PRIMARY: Color32 = rgb(0xa2a6a7); // widgets at rest, the primary — 1.883
const STONE_L_HOVER_STROKE: Color32 = rgb(0x565a5c); // hovered strokes — 3.68:1 on its own fill
const STONE_L_ON_WIDGET: Color32 = rgb(0x333739); // text on a widget — 9.216
const STONE_L_BODY: Color32 = rgb(0x171b1d); // body text — 13.290
const QUIET_L: Color32 = rgb(0x979b9d); // strokes at rest — 2.148
// Named rather than derived, because egui's 0.6 alpha lands in a different place on a light ground
// (module header, *Weak text*). This is dark's own 5.59:1 weight, measured against the light page.
const STONE_L_WEAK: Color32 = rgb(0x535759); // the box badge, field labels — 5.597

// The light accents: the hue is carried, the value re-placed. **Selection inverts**, and it has to:
// body text is drawn *over* the selection fill and must clear the 7:1 floor there, so on a light
// page the pale lichen becomes the fill and the deep one the stroke. That is the same relationship
// as dark's, said on a light ground, not a different decision.
const LICHEN_L: Color32 = rgb(0x485f6d); // links — 5.136, dark's 5.125
const CLAY_L: Color32 = rgb(0x534634); // warn — still dormant (ADR-0030 §5)
const ROSE_L: Color32 = rgb(0x7a5551); // error — still dormant (ADR-0030 §5)

/// The fill of a **card-like surface** — the review card, the editor's card rows (ADR-0033 §2).
///
/// A card is a **well**: darker than the page, so it is a hole you read into rather than a slab
/// sitting on top of it. Until #133 the card was drawn on `widgets.inactive.bg_fill`, which is
/// *lighter* than the page, and the whole complaint was that the thing being studied was made of
/// the same material as the buttons under it.
///
/// **It no longer shares a fill with a text field, and this function is where that happened**
/// (#163). ADR-0033 §2 accepted the sharing on two grounds — an 8px corner against the widget's 2px,
/// **and** that the two never appear on the same screen — and said that *"if they ever must diverge,
/// this function is the one place that changes"*. The second ground was untrue from the day the card
/// landed: the editor draws the Front and Back fields in one column and the card faces in the other,
/// in both themes. #150 put the number on it — **1.000:1**, not similar but identical — leaving the
/// corner carrying the whole distinction alone. So the function changed, exactly where §2 said it
/// would.
///
/// **The card kept its value and the field moved**, which is the half worth stating. The card's fill
/// is ADR-0033's decided well and #125 banked a result on it — 1.121:1 still reading as cut into the
/// page on an OLED panel at low brightness — so moving it would have spent evidence already in hand.
/// The field's fill had no argument behind it at all; it was the rung egui happens to put text edits
/// on. The field is therefore what a knob moved, and where it stopped is `STONE_1` / `STONE_L_FIELD`.
///
/// **It names a constant per theme now instead of reading `extreme_bg_color`, and that is a real
/// loss.** Riding the ambient slot was what made this theme-correct for free (ADR-0036 §1), and a
/// branch is the narrow case that ADR accepts rather than the pattern it prefers — the same shape
/// [`card_divider`] already had to take, for the same reason: egui has no "card" slot, only a
/// deepest-background one, and once the card and the deepest background are different colours, one
/// of them has to be named. `extreme_bg_color` keeps its own meaning and goes to the **field**,
/// which is what egui means by it, so every text edit in the app still gets its fill ambiently and
/// no call site names a colour.
pub fn card_fill(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        STONE_0
    } else {
        STONE_L_CARD
    }
}

/// The edge of a card-like surface (ADR-0033 §2) — one rung further from the page than the fill, so
/// a card has a boundary without a line anyone would call a border.
///
/// This is the one card role with **no ambient slot to ride**: egui has no "card edge", and
/// `widgets.noninteractive.bg_stroke` is the separator, which the light palette had to pull apart
/// from the card's edge (they are one rung in dark and two in light, because a well's edge follows
/// the well). So it branches on the theme — the narrow case ADR-0036 §1 accepts, and the same shape
/// as the map's open question about values the renderer offers no slot for.
pub fn card_stroke(visuals: &Visuals) -> Stroke {
    Stroke::new(1.0, card_divider(visuals))
}

/// The hairline dividing a card's two faces (ADR-0033 §1). The same rung as the edge, because it is
/// the same claim — *this is one object* — said on the inside.
pub fn card_divider(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        STONE_4
    } else {
        STONE_L_CARD_EDGE
    }
}

/// The fill of an ordinary **control** — a grade, *Edit note*, the settings and notes buttons
/// (ADR-0034 §1).
///
/// The `faint_bg_color` rung, where until #134 every control took the `widgets.inactive` one. The
/// reason is a measurement rather than a preference: against the page the application draws, that
/// rung is **1.293:1** and [`card_fill`] is **1.121:1**, so a control was more separated from the
/// page than the thing being studied and [ADR-0033 §3](../../../docs/adr/0033-the-card.md) required
/// that inverted. `faint_bg_color` measures **1.099:1** in dark and **1.049:1** in light — quieter
/// than the card in both, with a fill still there.
///
/// **Outline-or-slab was a false pair, and this rung is why.** ADR-0033 §3 photographed a control
/// with no fill at all and drew the right conclusion from it, having drawn only the two ends of the
/// ramp. A control that keeps a surface keeps looking like a control, which is the property the
/// judging session turned out to care about most.
///
/// **`widgets.inactive.bg_fill` now holds this same value, and that is argued rather than
/// accidental** (#163). Two slots at one colour is exactly what #149 said must be read as a finding
/// unless somebody defends it, so: `widgets.inactive` is the rung **every un-wrapped egui widget
/// inherits**, and an un-wrapped widget is an ordinary control — that is what ADR-0034 §1's *"the
/// default weight, and everything the user can press that is not the way forward on a card-less
/// screen"* means when the renderer, not this crate, is doing the drawing. They agree because the
/// role is the same, and `an_unwrapped_widget_inherits_the_ordinary_weight` pins the agreement.
pub fn control_fill(visuals: &Visuals) -> Color32 {
    visuals.faint_bg_color
}

/// The edge of an ordinary control (ADR-0034 §1). One rung further from the page than its fill, the
/// same relationship [`card_stroke`] has to [`card_fill`] — and it rides the separator slot, which
/// is the rung it shares in both themes.
pub fn control_stroke(visuals: &Visuals) -> Stroke {
    Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color)
}

/// The fill of the **one control on a screen that is the way forward** — *Start*, the durable leech
/// entrance, the end-of-session pointer's pair (ADR-0034 §2).
///
/// The `widgets.inactive` rung, which is what *every* control was before #134. ADR-0033 §3 is a
/// **relationship** — the controls are quieter *than the card* — and a screen with no card on it has
/// nothing for that comparison to be about. Drawn at [`control_fill`], the entrance is a faint
/// rectangle on an empty page that reads as disabled; this is the rung it keeps.
///
/// **It is 1.293:1 above the page in dark and 1.883:1 below it in light**, and the asymmetry is the
/// ink construction's, not a drift: on a light page there is no room above for a control to be, so
/// the primary is placed by its gap from the card instead (ADR-0036 §2). That is also why light's
/// body-on-primary is the tightest reading pair either theme has — see the module header.
///
/// **It names its rung instead of riding `widgets.inactive.bg_fill`, and #163 is why.** #134 put it
/// on that slot following this crate's own ambient-role discipline, and the discipline was the wrong
/// instrument here: `widgets.inactive` is not merely *a* slot, it is the one **every un-wrapped egui
/// widget already reads**. Riding it put the loudest role in the most-inherited place, so fifteen
/// call sites that never went through `controls::` — six raw `ui.button`s and three `ComboBox`es on
/// Notes, five on the leech screen, one on Settings — drew themselves as primaries, beside a card,
/// which is exactly what ADR-0033 §3 forbids. Measured off the shipped app: `#2c3237` where an
/// ordinary control is `#21262a`.
///
/// The lesson is narrower than *don't ride slots* and is worth stating: the ambient-role pattern asks
/// *which slot carries this family* and never asks **who else already reads it**. A slot with its own
/// population is a broadcast, not a name — so the rung every widget inherits belongs to the role
/// every widget should have, and the exception is what names a value.
pub fn primary_fill(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        STONE_5
    } else {
        STONE_L_PRIMARY
    }
}

/// The edge of a primary control (ADR-0034 §2) — the stroke rung every widget already rests at.
pub fn primary_stroke(visuals: &Visuals) -> Stroke {
    Stroke::new(1.0, visuals.widgets.inactive.bg_stroke.color)
}

/// The **link** accent, and #134 is its first caller (ADR-0034 §2).
///
/// [ADR-0030 §5](../../../docs/adr/0030-the-first-finish-pass-decisions.md) recorded warn, error and
/// link as *"defined-and-dormant"*, explicitly to stop a later reader finding a call site for a
/// colour because the colour exists. Waking one is therefore a decision that has to be taken rather
/// than a use that may be made, and ADR-0034 takes it: the entrance's second line — *"or a shorter
/// sitting: 5 10 20"* — is a set of text actions with no surface of their own, and at weak-text
/// weight they were very nearly invisible. §5's *rule* is unchanged and warn and error stay dormant.
pub fn link(visuals: &Visuals) -> Color32 {
    visuals.hyperlink_color
}

/// The fill of a surface that is **temporarily on top of the page** (ADR-0037 §1).
///
/// **It rises by exactly as much as a card sinks, and that is the whole construction.** ADR-0033 §1
/// cuts a card *into* the page, so depth here is subtractive everywhere permanent; the one surface
/// #149 calls temporary is therefore the one surface that goes the other way, by the same amount.
/// Dark delivers **1.121:1** between page and card, so the popup sits 1.12:1 *above* the page in
/// both themes — placed by the gap dark delivers rather than by each theme's own page-relative
/// ratio, which is [ADR-0036 §2](../../../docs/adr/0036-the-light-palette.md)'s method and the thing
/// that ADR exists to stop anyone mirroring.
///
/// **The two directions are not equally affordable, and it decides how far this can ever go.** In
/// dark the risen direction is fully occupied — `STONE_3` the ordinary control at 1.099, `STONE_4`
/// the separator at 1.222, `STONE_5` the primary at 1.293 — so this lands *between* rungs rather
/// than on one. In light it is empty and nearly exhausted: **1.305:1** in total between the page and
/// pure white, no role occupying any of it, and this rise spends 1.125. **Light can afford exactly
/// one risen surface**, which is the argument to quote at whoever next proposes a raised card.
const POPUP_RISEN: Color32 = rgb(0x22282b); // 1.124:1 above the dark page
const POPUP_RISEN_L: Color32 = rgb(0xeaeff0); // 1.125:1 above the light page

/// The shadow a floating surface casts (ADR-0037 §1).
///
/// **Stock's geometry, and a darkening that was chosen.** Nothing in #154 disputed the offset or the
/// blur, so they are stock's; only the alpha was ever in question, and it was judged in both themes
/// at once or not at all.
///
/// **The two alphas differ by 8× and buy the same thing.** Dark's 200 measures **1.159:1** against
/// its page, light's 25 measures **1.156:1** against its own. Stock's own defaults — 96 and 25 at
/// identical offset and blur — measure 1.083 and 1.156, an **1.88× disagreement in the very
/// quantity being set**, which is `weak_text_alpha`'s shape (module header) arriving from the
/// renderer's side. A shadow is a darkening, and a darkening is not one gesture across two grounds.
///
/// **Why the mechanism did not split, when the arithmetic invited it.** At stock's alpha a dark
/// shadow is *quieter than the card's own well* — 1.083:1 against ADR-0033's 1.121:1 — so dark could
/// have leaned on the rise alone while light, where the rise is nearly unaffordable, leaned on the
/// shadow alone. Judged blurred, which is ADR-0033 §3's own instrument, **one material won in both
/// themes**. So the asymmetry lives in these two numbers rather than in a rule, which is the smaller
/// place for it to live.
fn popup_shadow(alpha: u8) -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [6, 10],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    }
}

/// **Which theme the user asked for** (ADR-0036 §3). The decision, as distinct from
/// `egui::ThemePreference`, which is one renderer's way of carrying it — a native client honours
/// the same three options against its own platform setting.
///
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

    fn preference(self) -> ThemePreference {
        match self {
            Self::System => ThemePreference::System,
            Self::Light => ThemePreference::Light,
            Self::Dark => ThemePreference::Dark,
        }
    }
}

/// Install **both** palettes and apply the user's choice (ADR-0036 §3, superseding ADR-0030 §2).
/// Called from [`crate::CairnApp::new`], and again whenever the choice changes on Settings.
///
/// Each palette goes into **its own slot** — never the untargeted `set_visuals`, which writes to
/// whichever slot is active when it is called and would leave the other on stock. Filling both is
/// what ADR-0030 §2's refusal becomes now that light exists: the outcome it forbade was an unfilled
/// slot, and none is left unfilled here, so `System` is safe to honour.
pub fn install(ctx: &egui::Context, choice: ThemeChoice) {
    ctx.set_visuals_of(Theme::Dark, cairn_dark());
    ctx.set_visuals_of(Theme::Light, cairn_light());
    ctx.set_theme(choice.preference());
}

/// The palette itself, built from egui's dark theme so every field this does not name keeps its
/// default — a future egui release adding a field is then a value we inherit, not a compile error.
pub fn cairn_dark() -> Visuals {
    let mut v = Visuals::dark();

    v.panel_fill = STONE_2;
    // The popup **rises**; it is no longer the page's own colour (ADR-0037 §1). Assigning
    // `panel_fill` here is what drew every open combo box in exactly the page colour, in every
    // capture this repository held before #154.
    v.window_fill = POPUP_RISEN;
    v.window_stroke = Stroke::new(1.0, STONE_4); // the separator rung, never stock's off-ramp grey
    // The **field's** rung, not the card's (#163). `extreme_bg_color` is egui's deepest-background
    // slot and what a `TextEdit` draws itself on, so leaving it to the field is what it means; the
    // card names `STONE_0` through `theme::card_fill` now that the two are different colours. Every
    // text edit in the app therefore still gets its fill ambiently and no call site names a colour.
    v.extreme_bg_color = STONE_1;
    v.faint_bg_color = STONE_3;
    v.override_text_color = None; // body colour rides fg_stroke, per widget state

    v.widgets.noninteractive.bg_fill = STONE_2;
    v.widgets.noninteractive.weak_bg_fill = STONE_2;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STONE_4); // separators
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, STONE_11); // body text
    v.widgets.noninteractive.corner_radius = CornerRadius::same(2);

    // **The rung every un-wrapped widget inherits, so it is the ordinary one.** #134 rode the
    // *primary* here, which put the loudest role in the most-inherited slot — every raw `ui.button`
    // and every `ComboBox` in the app then drew itself as a primary.
    v.widgets.inactive.bg_fill = STONE_3;
    v.widgets.inactive.weak_bg_fill = STONE_3;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, QUIET);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, STONE_10);
    v.widgets.inactive.corner_radius = CornerRadius::same(2);

    v.widgets.hovered.bg_fill = STONE_6;
    v.widgets.hovered.weak_bg_fill = STONE_6;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, STONE_9);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, STONE_11);
    v.widgets.hovered.corner_radius = CornerRadius::same(3);
    v.widgets.hovered.expansion = 1.0;

    v.widgets.active.bg_fill = STONE_4;
    v.widgets.active.weak_bg_fill = STONE_4;
    v.widgets.active.bg_stroke = Stroke::new(1.0, STONE_11);
    v.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(2);

    v.widgets.open.bg_fill = STONE_4;
    v.widgets.open.weak_bg_fill = STONE_4;
    v.widgets.open.bg_stroke = Stroke::new(1.0, QUIET);
    v.widgets.open.fg_stroke = Stroke::new(1.0, STONE_10);
    v.widgets.open.corner_radius = CornerRadius::same(2);

    v.selection.bg_fill = LICHEN_DEEP;
    v.selection.stroke = Stroke::new(1.0, LICHEN_PALE);

    v.hyperlink_color = LICHEN;
    v.warn_fg_color = CLAY;
    v.error_fg_color = ROSE;

    // **One thing floats** (ADR-0037 §1). A shadow means *this surface is temporarily on top of the
    // page and will go away*, so only what the renderer already calls a popup, a menu or a window
    // casts one. `window_shadow` stays `NONE` because this application draws no egui window; the
    // day it does, that is a decision and not a default.
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = popup_shadow(200);

    v
}

/// The same stone in daylight (ADR-0036 §1, §2). Built from egui's **light** theme for the same
/// reason [`cairn_dark`] is built from its dark one — a future egui release adding a field gives a
/// value we inherit rather than a compile error.
///
/// Field for field this mirrors [`cairn_dark`], and it must: a slot filled there and left stock here
/// is the failure mode the whole theme is exposed to, and it is invisible until someone switches.
/// `both_palettes_name_the_same_slots` walks the two and fails if either names a field the other
/// leaves alone.
pub fn cairn_light() -> Visuals {
    let mut v = Visuals::light();

    v.panel_fill = STONE_L_PAGE;
    // As in dark, and by dark's gap rather than by a light-relative ratio (ADR-0037 §1).
    v.window_fill = POPUP_RISEN_L;
    v.window_stroke = Stroke::new(1.0, STONE_L_EDGE); // the separator rung
    // The field's own rung (#163), minted rather than reusing `STONE_L_EDGE` four units away — see
    // `STONE_L_FIELD` for why the distance was not the argument.
    v.extreme_bg_color = STONE_L_FIELD;
    v.faint_bg_color = STONE_L_CONTROL;
    v.override_text_color = None; // body colour rides fg_stroke, per widget state

    v.widgets.noninteractive.bg_fill = STONE_L_PAGE;
    v.widgets.noninteractive.weak_bg_fill = STONE_L_PAGE;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STONE_L_EDGE); // separators
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, STONE_L_BODY); // body text
    v.widgets.noninteractive.corner_radius = CornerRadius::same(2);

    v.widgets.inactive.bg_fill = STONE_L_CONTROL;
    v.widgets.inactive.weak_bg_fill = STONE_L_CONTROL;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, QUIET_L);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, STONE_L_ON_WIDGET);
    v.widgets.inactive.corner_radius = CornerRadius::same(2);

    v.widgets.hovered.bg_fill = STONE_L_HOVER;
    v.widgets.hovered.weak_bg_fill = STONE_L_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, STONE_L_HOVER_STROKE);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, STONE_L_BODY);
    v.widgets.hovered.corner_radius = CornerRadius::same(3);
    v.widgets.hovered.expansion = 1.0;

    v.widgets.active.bg_fill = STONE_L_EDGE;
    v.widgets.active.weak_bg_fill = STONE_L_EDGE;
    v.widgets.active.bg_stroke = Stroke::new(1.0, STONE_L_BODY);
    // Dark's active foreground is `WHITE` — the far end of the ramp, away from every surface. On a
    // light page the far end is the other one, so this is `BLACK` and not an inversion anyone chose.
    v.widgets.active.fg_stroke = Stroke::new(2.0, Color32::BLACK);
    v.widgets.active.corner_radius = CornerRadius::same(2);

    v.widgets.open.bg_fill = STONE_L_EDGE;
    v.widgets.open.weak_bg_fill = STONE_L_EDGE;
    v.widgets.open.bg_stroke = Stroke::new(1.0, QUIET_L);
    v.widgets.open.fg_stroke = Stroke::new(1.0, STONE_L_ON_WIDGET);
    v.widgets.open.corner_radius = CornerRadius::same(2);

    // **Selection inverts, and it has to.** Body text is drawn *over* the selection fill and must
    // clear the 7:1 floor there; in light the body is near-black, so the pale lichen becomes the
    // fill and the deep one the stroke. Same relationship as dark's, said on a light ground.
    v.selection.bg_fill = LICHEN_PALE;
    v.selection.stroke = Stroke::new(1.0, LICHEN_L);

    v.hyperlink_color = LICHEN_L;
    v.warn_fg_color = CLAY_L;
    v.error_fg_color = ROSE_L;

    // Named rather than left to `weak_text_alpha`, which lands in a different place on a light
    // ground and would make the badge quieter than dark's by accident (module header).
    v.weak_text_color = Some(STONE_L_WEAK);

    // As in dark, at an eighth of the alpha for the same measured weight (ADR-0037 §1).
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = popup_shadow(25);

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The relative luminance of an **opaque** colour, per WCAG 2.1.
    fn luminance(c: Color32) -> f64 {
        let channel = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    /// WCAG contrast ratio between two **opaque** colours.
    fn contrast(fg: Color32, bg: Color32) -> f64 {
        let (a, b) = (luminance(fg), luminance(bg));
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Composite a possibly-translucent `fg` (premultiplied, as egui stores it) over an opaque `bg`,
    /// so the *effective* drawn colour can be measured. `weak_text_color()` is body text at reduced
    /// alpha, so what the eye reads is the blend, not the source.
    fn over(fg: Color32, bg: Color32) -> Color32 {
        let a = fg.a() as f64 / 255.0;
        let blend = |f: u8, b: u8| (f as f64 + b as f64 * (1.0 - a)).round() as u8;
        Color32::from_rgb(
            blend(fg.r(), bg.r()),
            blend(fg.g(), bg.g()),
            blend(fg.b(), bg.b()),
        )
    }

    /// The visuals as the app reads them at rest — the non-interactive, panel-surface state every
    /// `text`/`body`/`badge` helper draws through.
    fn visuals() -> Visuals {
        cairn_dark()
    }

    /// Every **reading-text** pair the app draws clears the 7:1 floor (ADR-0030 §3). These are the
    /// pairs the crate reads via the ambient visuals: body text on the panel, body text on a card
    /// (the widget fill a card face draws over), and body text over the one reachable accent, the
    /// selection fill. Weak text is the one text pair below the floor and is held separately, by
    /// `weak_text_is_not_a_regression`, for the reason in the module header.
    #[test]
    fn text_pairs_clear_the_contrast_floor() {
        // **Both palettes.** A floor checked in one theme says nothing about the other, and light
        // is the theme with the tight pair (ADR-0036 §2).
        for (theme, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            let body = v.text_color();

            for (name, fg, bg) in [
                ("body-on-panel", body, v.panel_fill),
                ("body-on-card", body, card_fill(&v)),
                ("body-on-ordinary", body, control_fill(&v)),
                ("body-on-primary", body, primary_fill(&v)),
                ("body-on-selection", body, v.selection.bg_fill),
            ] {
                let ratio = contrast(fg, bg);
                assert!(
                    ratio >= 7.0,
                    "{theme}: {name} is {ratio:.2}:1, below the 7:1 contrast floor (ADR-0030 §3)"
                );
            }
        }
    }

    /// **The tightest reading pair either theme has**, pinned by figure rather than by inequality.
    ///
    /// Light's `primary` is 1.883:1 *below* its page where dark's is 1.293:1 *above* — the ink
    /// construction's price (ADR-0036 §2) — so body text on it lands at **7.06:1**, over the floor
    /// by 0.06. `text_pairs_clear_the_contrast_floor` would still pass at 7.001, which is passing by
    /// accident; this fails on any nudge to either colour, in either direction, so the margin has to
    /// be re-argued rather than quietly spent.
    #[test]
    fn the_light_primary_is_the_tightest_reading_pair() {
        let light = cairn_light();
        let ratio = contrast(light.text_color(), primary_fill(&light));
        assert!(
            (ratio - 7.06).abs() < 0.02,
            "light body-on-primary is {ratio:.3}:1; ADR-0036 §2 records 7.06:1 and it is the \
             tightest pair in either theme"
        );

        // And it really is the tightest — if some other pair drops below it, this test is no longer
        // guarding the right one and the module header is wrong.
        for (name, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            for (pair, bg) in [
                ("panel", v.panel_fill),
                ("card", card_fill(&v)),
                ("ordinary", control_fill(&v)),
                ("primary", primary_fill(&v)),
                ("selection", v.selection.bg_fill),
            ] {
                let other = contrast(v.text_color(), bg);
                assert!(
                    other >= ratio - 0.02,
                    "{name} body-on-{pair} is {other:.3}:1, tighter than the pair the module \
                     header calls the tightest ({ratio:.3}:1)"
                );
            }
        }
    }

    /// The measured exemplar the ADR quotes: body-on-panel is **13.34:1**, up from stock's 5.12:1.
    /// Pinned on its own so a change to the body colour or the panel fill that still cleared 7:1 but
    /// walked away from the headline figure would be caught.
    #[test]
    fn body_on_panel_matches_the_recorded_measurement() {
        let v = visuals();
        let ratio = contrast(v.text_color(), v.panel_fill);
        assert!(
            (ratio - 13.34).abs() < 0.05,
            "body-on-panel is {ratio:.2}:1; ADR-0030 §3 records 13.34:1"
        );
    }

    /// Weak text (the box badge, field labels) is a **pre-existing weakness**, not a regression: it
    /// is deliberately dim (ADR-0030 §4's quiet footnote) and sits below the 7:1 floor, but it must
    /// never drop below stock egui's weak text, which is what it replaces (module header). Stock's
    /// weak text is `gray(140)` on `gray(27)`, ~5.12:1; ours must clear that. Composited because
    /// `weak_text_color()` is translucent.
    #[test]
    fn weak_text_is_not_a_regression() {
        let v = visuals();
        let ours = contrast(over(v.weak_text_color(), v.panel_fill), v.panel_fill);

        let stock = Visuals::dark();
        let stock_weak = contrast(
            over(stock.weak_text_color(), stock.panel_fill),
            stock.panel_fill,
        );

        assert!(
            ours >= stock_weak,
            "weak-text-on-panel is {ours:.2}:1, below stock's {stock_weak:.2}:1 — a regression"
        );
    }

    /// [`install`] writes **each palette into its own slot**, and leaves neither on stock
    /// (ADR-0036 §3). The test reproduces the bug the targeted setter fixes: it launches "in light
    /// mode" *before* installing. The untargeted `set_visuals` writes to whichever slot is active,
    /// so it would fill one and leave the other stock — and which one would depend on what the OS
    /// happened to prefer at construction. So restoring `set_visuals` in `install` breaks this test.
    ///
    /// **Filling both is what ADR-0030 §2's refusal becomes.** §2 forbade an unfilled slot, and
    /// expressed that by making no slot reachable; now that light is drawn, the same refusal is
    /// honoured by leaving no slot unfilled, which is what makes `System` safe to offer.
    #[test]
    fn install_fills_both_slots_and_leaves_neither_stock() {
        let ctx = egui::Context::default();
        // Launch in light mode — the exact condition under which the untargeted setter misfires.
        ctx.set_theme(ThemePreference::Light);

        install(&ctx, ThemeChoice::System);

        assert_eq!(
            ctx.style_of(Theme::Dark).visuals.panel_fill,
            STONE_2,
            "the dark palette must land in the dark slot, never in whichever was active"
        );
        assert_eq!(
            ctx.style_of(Theme::Light).visuals.panel_fill,
            STONE_L_PAGE,
            "the light palette must land in the light slot"
        );
        assert_ne!(
            ctx.style_of(Theme::Light).visuals.panel_fill,
            Visuals::light().panel_fill,
            "no reachable slot may be left on stock egui — the outcome ADR-0030 §2 refused"
        );
        assert_ne!(
            ctx.style_of(Theme::Dark).visuals.panel_fill,
            Visuals::dark().panel_fill,
            "no reachable slot may be left on stock egui — the outcome ADR-0030 §2 refused"
        );
    }

    /// The three choices reach the three preferences, and anything unrecognised follows the
    /// platform rather than refusing (ADR-0036 §3).
    #[test]
    fn the_theme_choice_round_trips_and_degrades_to_system() {
        for choice in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
            assert_eq!(
                ThemeChoice::parse(Some(choice.as_str())),
                choice,
                "{choice:?} must survive a round trip through the store"
            );
            let ctx = egui::Context::default();
            install(&ctx, choice);
            assert_eq!(ctx.options(|o| o.theme_preference), choice.preference());
        }

        assert_eq!(ThemeChoice::parse(None), ThemeChoice::System, "unset");
        assert_eq!(
            ThemeChoice::parse(Some("sepia")),
            ThemeChoice::System,
            "a value from a build that knew more themes than this one must follow the platform, \
             not refuse to start"
        );
        assert_eq!(ThemeChoice::default(), ThemeChoice::System);
    }

    /// The hover stroke against its own fill must clear **3:1** (ADR-0030 §3). This is the pair the
    /// draft regressed to 2.49:1; the fix lifts it to `STONE_9`. Non-text, so the floor is 3:1, not 7:1 —
    /// but hover is exactly the state the rule covers, so it is not left failing.
    #[test]
    fn hover_stroke_clears_three_to_one() {
        for (theme, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            let ratio = contrast(v.widgets.hovered.bg_stroke.color, v.widgets.hovered.bg_fill);
            assert!(
                ratio >= 3.0,
                "{theme}: hover stroke is {ratio:.2}:1 against its fill, below 3:1 (ADR-0030 §3)"
            );
        }
    }

    /// **The light ramp is re-derived, not re-hued** (ADR-0036 §1). Every light fill is recomputed
    /// here from the *dark* constants — the same ratio for the same role against the same reference,
    /// placed away from the light page — and compared with what [`cairn_light`] names.
    ///
    /// This is the test that makes the constants outputs rather than choices. Nothing else stops a
    /// later reader nudging `STONE_L_CARD` a shade because it looked better on their monitor, which
    /// **An un-wrapped egui widget draws itself as an ordinary control** (#163, ADR-0034 §1).
    ///
    /// This is the test that was missing. `widgets.inactive.bg_fill` is what every widget this crate
    /// does not wrap reads — a raw `ui.button`, a `ComboBox`, anything egui draws on its own — and
    /// #134 gave that slot to the **primary**, following the ambient-role discipline. Fifteen call
    /// sites then drew the loudest weight in the palette, beside a card, and nothing failed: the
    /// source was right at every one of them, the ADR was right, and the two never met.
    ///
    /// So the invariant is stated where the leak was, in terms of what a *screen* gets rather than
    /// what a slot holds: ask for nothing and you get an ordinary control. The primary is the
    /// exception and has to be asked for by name.
    #[test]
    fn an_unwrapped_widget_inherits_the_ordinary_weight() {
        for (name, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            assert_eq!(
                v.widgets.inactive.bg_fill,
                control_fill(&v),
                "{name}: a widget nobody wrapped should be an ordinary control"
            );
            assert_ne!(
                v.widgets.inactive.bg_fill,
                primary_fill(&v),
                "{name}: the primary is back on the slot every widget inherits, so every \
                 un-wrapped control is a primary again"
            );
        }
    }

    /// **A card and a text field are not the same material, in either theme** (#163, amending
    /// ADR-0033 §2).
    ///
    /// §2 accepted them sharing `extreme_bg_color` on two grounds — an 8px corner against the
    /// widget's 2px, **and** that the two never appear on the same screen. The second was untrue from
    /// the day the card landed: the editor draws the fields in one column and the card faces in the
    /// other. #150 measured the separation at **1.000:1** — the same colour, in both themes — and
    /// nothing failed, because a shared value is not a defect any test was looking for.
    ///
    /// Pinned as a **floor plus the two figures**, not as an inequality alone. An inequality passes
    /// at 1.001:1, which is the state this fixes wearing a different number.
    #[test]
    fn a_card_and_a_text_field_are_different_materials() {
        for (name, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            let card = card_fill(&v);
            let field = v.extreme_bg_color;
            assert_ne!(
                card, field,
                "{name}: the card and the field are one colour again"
            );
            let separation = contrast(card, field);
            assert!(
                separation > 1.05,
                "{name}: card↔field is {separation:.3}:1, back within rounding of the 1.000:1 \
                 #150 measured"
            );
        }
        // The figures a thumb chose, so a nudge to either rung fails loudly rather than drifting.
        assert!((contrast(STONE_0, STONE_1) - 1.063).abs() < 0.005);
        assert!((contrast(STONE_L_CARD, STONE_L_FIELD) - 1.152).abs() < 0.005);
    }

    /// **The field sits between the page and the card, in both themes** — the structural half of the
    /// same decision, and the one that keeps ADR-0033 §3 true.
    ///
    /// §3 as ADR-0036 restated it says that on a screen with a card, every control is quieter than
    /// it. A text field is a control, and the editor is the screen. Moving the field *away* from the
    /// page instead — deeper than the card — would buy the same separation, read as a second well,
    /// and make the card no longer the deepest thing on the screen it is the subject of.
    #[test]
    fn the_field_rung_lies_between_the_page_and_the_card() {
        for (name, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            let page = v.panel_fill;
            let field = contrast(v.extreme_bg_color, page);
            let card = contrast(card_fill(&v), page);
            assert!(
                field < card,
                "{name}: the field is {field:.3}:1 from the page and the card {card:.3}:1 — the \
                 card has to stay the deepest surface on a screen that has one"
            );
        }
        // Dark's rung fills the ramp's own gap: `STONE_0` → `STONE_2` was its only double step, so
        // nothing is spent to seat a field between them. #143 had recorded the opposite as the cost.
        let below = contrast(STONE_0, STONE_1);
        let above = contrast(STONE_1, STONE_2);
        assert!(
            (below - above).abs() < 0.02,
            "STONE_1 should sit near the middle of the gap it fills: {below:.3}:1 below, \
             {above:.3}:1 above"
        );
    }

    /// is exactly how the design project's placeholders drifted warm and broke §3 without failing.
    #[test]
    fn the_light_ramp_is_re_derived_not_re_hued() {
        let d = cairn_dark();
        let page = STONE_L_PAGE;

        // The gaps the sitting chose: the ordinary control just off the page, the card one
        // card-to-ordinary gap below it, the primary one card-to-primary gap below the card.
        let gap_card_ordinary = contrast(card_fill(&d), control_fill(&d));
        let gap_card_primary = contrast(card_fill(&d), primary_fill(&d));

        let ordinary = contrast(STONE_L_CONTROL, page);
        let card = contrast(STONE_L_CARD, page);
        let primary = contrast(STONE_L_PRIMARY, page);

        assert!(
            (contrast(STONE_L_CARD, STONE_L_CONTROL) - gap_card_ordinary).abs() < 0.02,
            "light card↔ordinary is {:.3}:1; dark delivers {gap_card_ordinary:.3}:1 and the light \
             palette exists to preserve it",
            contrast(STONE_L_CARD, STONE_L_CONTROL)
        );
        assert!(
            (contrast(STONE_L_CARD, STONE_L_PRIMARY) - gap_card_primary).abs() < 0.02,
            "light card↔primary is {:.3}:1; dark delivers {gap_card_primary:.3}:1",
            contrast(STONE_L_CARD, STONE_L_PRIMARY)
        );
        assert!(
            ordinary < card && card < primary,
            "ADR-0033 §3's ordering must survive the change of ground: ordinary {ordinary:.3}, \
             card {card:.3}, primary {primary:.3}"
        );

        // Every light fill is **darker** than the page. This is the structural claim ADR-0036 §2
        // makes and the one a re-hue would break: on a light page there is no room above it, so a
        // value that came out lighter means someone mirrored the dark ramp instead of re-deriving.
        for (name, c) in [
            ("control", STONE_L_CONTROL),
            ("card", STONE_L_CARD),
            ("primary", STONE_L_PRIMARY),
            ("hover", STONE_L_HOVER),
            ("edge", STONE_L_EDGE),
        ] {
            assert!(
                luminance(c) < luminance(page),
                "light {name} is lighter than the page — the light palette places every fill below \
                 it, because above it there is nothing (ADR-0036 §2)"
            );
        }
    }

    /// **The two palettes name the same slots.** A field set in one and left stock in the other is
    /// invisible until someone switches theme, which is the failure mode a second palette
    /// introduces and the reason this exists (ADR-0036 §1).
    #[test]
    fn both_palettes_name_the_same_slots() {
        let (d, l) = (cairn_dark(), cairn_light());
        let (sd, sl) = (Visuals::dark(), Visuals::light());

        // For each slot: if dark moved it off stock, light must have moved it off stock too.
        let slots: [(&str, Color32, Color32, Color32, Color32); 8] = [
            (
                "panel_fill",
                d.panel_fill,
                sd.panel_fill,
                l.panel_fill,
                sl.panel_fill,
            ),
            (
                "window_fill",
                d.window_fill,
                sd.window_fill,
                l.window_fill,
                sl.window_fill,
            ),
            (
                "extreme_bg_color",
                d.extreme_bg_color,
                sd.extreme_bg_color,
                l.extreme_bg_color,
                sl.extreme_bg_color,
            ),
            (
                "faint_bg_color",
                d.faint_bg_color,
                sd.faint_bg_color,
                l.faint_bg_color,
                sl.faint_bg_color,
            ),
            (
                "hyperlink_color",
                d.hyperlink_color,
                sd.hyperlink_color,
                l.hyperlink_color,
                sl.hyperlink_color,
            ),
            (
                "warn_fg_color",
                d.warn_fg_color,
                sd.warn_fg_color,
                l.warn_fg_color,
                sl.warn_fg_color,
            ),
            (
                "error_fg_color",
                d.error_fg_color,
                sd.error_fg_color,
                l.error_fg_color,
                sl.error_fg_color,
            ),
            (
                "selection.bg_fill",
                d.selection.bg_fill,
                sd.selection.bg_fill,
                l.selection.bg_fill,
                sl.selection.bg_fill,
            ),
        ];
        for (name, dark, stock_dark, light, stock_light) in slots {
            if dark != stock_dark {
                assert_ne!(
                    light, stock_light,
                    "{name} is named in the dark palette and left on stock egui in the light one"
                );
            }
        }

        // Widget states, the same way — these are the ones easiest to forget, because a stock
        // hovered fill looks plausible right up until it is next to a Cairn one.
        for (name, dark, stock_dark, light, stock_light) in [
            (
                "noninteractive.bg_fill",
                d.widgets.noninteractive.bg_fill,
                sd.widgets.noninteractive.bg_fill,
                l.widgets.noninteractive.bg_fill,
                sl.widgets.noninteractive.bg_fill,
            ),
            (
                "inactive.bg_fill",
                d.widgets.inactive.bg_fill,
                sd.widgets.inactive.bg_fill,
                l.widgets.inactive.bg_fill,
                sl.widgets.inactive.bg_fill,
            ),
            (
                "hovered.bg_fill",
                d.widgets.hovered.bg_fill,
                sd.widgets.hovered.bg_fill,
                l.widgets.hovered.bg_fill,
                sl.widgets.hovered.bg_fill,
            ),
            (
                "active.bg_fill",
                d.widgets.active.bg_fill,
                sd.widgets.active.bg_fill,
                l.widgets.active.bg_fill,
                sl.widgets.active.bg_fill,
            ),
            (
                "open.bg_fill",
                d.widgets.open.bg_fill,
                sd.widgets.open.bg_fill,
                l.widgets.open.bg_fill,
                sl.widgets.open.bg_fill,
            ),
        ] {
            if dark != stock_dark {
                assert_ne!(
                    light, stock_light,
                    "{name} is named in the dark palette and left on stock egui in the light one"
                );
            }
        }

        // **Both cast on the popup and neither casts on a window** (ADR-0037 §1). This replaces the
        // earlier *both refuse shadow*, which was true until #154 chose the value and is exactly the
        // kind of claim that should fail loudly when it stops being true rather than be edited away.
        assert_eq!(d.window_shadow, egui::epaint::Shadow::NONE);
        assert_eq!(l.window_shadow, egui::epaint::Shadow::NONE);
        assert_ne!(
            d.popup_shadow,
            egui::epaint::Shadow::NONE,
            "the one surface that floats casts in dark"
        );
        assert_ne!(
            l.popup_shadow,
            egui::epaint::Shadow::NONE,
            "and in light — a slot filled in one theme only is this test's whole subject"
        );
        // Both keep the 2px widget corner.
        assert_eq!(
            l.widgets.noninteractive.corner_radius,
            d.widgets.noninteractive.corner_radius
        );
    }

    /// **The elevation numbers are a decision and ADR-0037 quotes them** (§1). The geometry is
    /// stock's and shared; the alphas are ours and differ by 8×.
    ///
    /// **The weights they buy — 1.159:1 dark, 1.156:1 light — cannot be asserted here**, and that is
    /// worth stating rather than leaving as an omission. A shadow's contribution at a given pixel is
    /// the *blur profile* evaluated there, which is epaint's and not ours, so the figures in the ADR
    /// were measured off shipped pixels in `docs/design/prototype-154/overlay-1280x800/`. What this
    /// test can hold is that nobody changes 200 or 25 without going back to those captures.
    #[test]
    fn the_two_shadow_alphas_are_the_measured_pair() {
        let d = cairn_dark().popup_shadow;
        let l = cairn_light().popup_shadow;

        assert_eq!(
            d.color.a(),
            200,
            "dark's alpha, back-solved to match light's weight"
        );
        assert_eq!(
            l.color.a(),
            25,
            "light's alpha — stock's, which measured correct"
        );
        assert_eq!(
            (d.offset, d.blur, d.spread),
            (l.offset, l.blur, l.spread),
            "one material in both themes: only the darkening differs (ADR-0037 §1)"
        );
        assert_eq!(
            (d.offset, d.blur, d.spread),
            ([6, 10], 8, 0),
            "stock's geometry, which #154 did not dispute"
        );
    }

    /// **The popup rises, and by dark's gap in both themes** (ADR-0037 §1, ADR-0036 §2's method).
    ///
    /// The defect this pins is the one that shipped: `window_fill` assigned `panel_fill`, so the
    /// application's only overlay was drawn in *exactly* the page colour, in both themes, in every
    /// capture this repository held before #154.
    #[test]
    fn the_popup_is_never_the_page_and_rises_by_the_same_gap() {
        for (name, v) in [("dark", cairn_dark()), ("light", cairn_light())] {
            assert_ne!(
                v.window_fill, v.panel_fill,
                "{name}: a popup drawn in the page's own colour is the #154 defect returning"
            );
            let rise = contrast(v.window_fill, v.panel_fill);
            assert!(
                (rise - 1.12).abs() < 0.01,
                "{name}: the rise mirrors ADR-0033's 1.121:1 well; got {rise:.3}"
            );
        }
    }

    /// **The badge weighs the same in both themes** (ADR-0030 §4, ADR-0036 §2). Dark gets that from
    /// egui's `weak_text_alpha`; light has to name it, because 60% of a near-black over a light
    /// ground lands much closer to the ground than 60% of a near-white over a dark one. Without the
    /// explicit value light's badge would be quieter than dark's by an accident of compositing.
    #[test]
    fn weak_text_carries_the_same_weight_in_both_themes() {
        let d = cairn_dark();
        let l = cairn_light();
        let dark_weak = contrast(over(d.weak_text_color(), d.panel_fill), d.panel_fill);
        let light_weak = contrast(over(l.weak_text_color(), l.panel_fill), l.panel_fill);

        assert!(
            (dark_weak - light_weak).abs() < 0.15,
            "weak text is {dark_weak:.2}:1 in dark and {light_weak:.2}:1 in light — the badge must \
             be the same quiet in both (ADR-0030 §4)"
        );

        // And light must not regress against stock light, the same bar dark is held to.
        let stock = Visuals::light();
        let stock_weak = contrast(
            over(stock.weak_text_color(), stock.panel_fill),
            stock.panel_fill,
        );
        assert!(
            light_weak >= stock_weak,
            "light weak text is {light_weak:.2}:1, below stock light's {stock_weak:.2}:1 — \
             a regression"
        );
    }
}
