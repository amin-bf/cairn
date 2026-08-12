//! **The controls** — the throwaway prototype for
//! [#134](https://github.com/amin-bf/cairn/issues/134), the fourth and last slice of the Review
//! vertical on the design pass map ([#121](https://github.com/amin-bf/cairn/issues/121)).
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-134`, the
//! repo's standing convention (`AGENTS.md`, *Rules that are easy to break silently* 3). Reachable
//! from any clone without merging:
//!
//! ```sh
//! git show prototypes/issue-134:docs/design/prototype-134/README.md
//! git checkout prototypes/issue-134 -- crates/desktop/src/bin/controls-prototype.rs
//! ```
//!
//! # What is held constant
//!
//! Everything the three ADRs before this one fixed. Like `card-prototype`, this draws through the
//! application's own modules — `frame::column`, `typography`, `spacing::gap`, `theme::cairn_dark`,
//! and **`surface::card` itself**, the shipped card, not a copy of it. The page is `panel_fill`,
//! because [ADR-0033 §2](../../../../docs/adr/0033-the-card.md) decided it.
//!
//! Drawing the *real* card matters more here than in any prototype before it. ADR-0033 §3 is a
//! constraint on this ticket stated as a comparison — *"a card outweighs the controls beneath
//! it"* — so a prototype that drew its own approximation of a card would be comparing the controls
//! against something the application does not have.
//!
//! # The axes
//!
//! | var | values | question |
//! |---|---|---|
//! | `PROTO_SCREEN` | `revealed`, `picker`, `caughtup`, `pointer`, `checkpoint`, `live` | which cluster is under the camera |
//! | `PROTO_GRADES` | `stacked`, `row`, `row4`, `rowplus` | the arrangement of the grade controls |
//! | `PROTO_WEIGHT` | `solid`, `faint`, `quiet` | what a control is **made of** — ADR-0033 §3's question |
//! | `PROTO_PREVIEW` | `same`, `small`, `none` | what the interval preview is, and whether it is there |
//! | `PROTO_ENTRANCE` | `counts`, `primary`, `primarylink`, `plain` | how a sitting is started |
//! | `PROTO_EMPTY` | `sentence`, `centred`, `bare` | what a caught-up Review screen is |
//! | `PROTO_CONTROL` | any number | the control height; the app ships 36, #124's variant E used 48 |
//!
//! # `PROTO_WEIGHT` is the axis the ticket was handed
//!
//! [ADR-0033 §3](../../../../docs/adr/0033-the-card.md) binds this ticket without deciding it: the
//! controls must end up **quieter than the card**. It reached that by blurring a capture until
//! nothing is legible and asking what still stands out, and on the shipped screen the answer was
//! the grade buttons.
//!
//! The three values are the three honest answers. `solid` is what the application draws now — the
//! `inactive` widget fill, a slab. `quiet` is the treatment §3 photographed as `PROTO_CONTROLS`:
//! same size, same hit target, no fill, a 1px edge. `faint` is the middle nobody has drawn — the
//! `faint_bg_color` rung, a control that is still a surface but a much shallower one — and it is
//! here because *outline or slab* is a false pair and the ramp has a rung between them.
//!
//! # `PROTO_GRADES` carries a question #124 answered and one it did not
//!
//! #124 chose **Forgot held apart, the three passes in one segmented row** (`row`), and this
//! prototype does not reopen that. What it does draw are the two arrangements that test whether the
//! choice is *robust*: `row4` puts all four grades in one row, which is the arrangement the choice
//! rejects, and `rowplus` puts a hypothetical **fourth pass grade** in the row — the question the
//! ticket asks in as many words. Four segments inside the frame's 640 measure is 154px each at the
//! judging width and **118px each at 560**, against `row`'s 208 and 163.
//!
//! # The screens nobody has photographed
//!
//! `caughtup`, `pointer` and `checkpoint` are three of the four Review states no capture in this
//! repository holds, because the capture seed always leaves cards due, ten minutes never elapse
//! under a four-second settle, and no card has ever been failed enough times to make a leech. They
//! are drawn here directly rather than reached through a sitting, which is the only way to get a
//! picture of them at all without changing the seed.
//!
//! **`checkpoint` is drawn twice on purpose**, and the pair is the point:
//! [ADR-0006 §1](../../../../docs/adr/0006-the-review-session-experience.md) says the checkpoint
//! surfaces *"without hiding the card underneath — the reviewer can still grade what they're
//! looking at while deciding"*, and the application draws it as an `else if` branch that replaces
//! the card. `PROTO_CHECKPOINT=replaces` is what ships; `PROTO_CHECKPOINT=over` is what the ADR
//! says.

use cairn_app::eframe;
use cairn_app::{fonts, frame, spacing, surface, theme, typography};
use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle, text::LayoutJob};

// --- the rungs this prototype needs by name ------------------------------------------------------
//
// A prototype is the one place a colour literal outside `theme` is not the defect ADR-0030 §1
// describes, because nothing here ships. Only the rungs the candidates disagree about are named;
// everything else is reached through `theme`'s installed visuals.

/// `faint_bg_color` — the rung between a slab and an outline. No control in the app takes it today.
const STONE_3: Color32 = Color32::from_rgb(0x21, 0x26, 0x2a);
/// `widgets.inactive.bg_fill` — what a control is made of today.
const STONE_5: Color32 = Color32::from_rgb(0x2c, 0x32, 0x37);
/// Separators, and the edge a `quiet` control keeps.
const STONE_4: Color32 = Color32::from_rgb(0x28, 0x2e, 0x33);
/// `widgets.noninteractive.bg_stroke` at rest.
const QUIET_STROKE: Color32 = Color32::from_rgb(0x33, 0x3b, 0x40);
/// The **link** accent. Dormant by [ADR-0030 §5] — defined, and with no call site anywhere in the
/// application. It is reachable here for exactly one candidate, `PROTO_ENTRANCE=primarylink`, which
/// is #124's variant E drawn as it actually was: its *"or a shorter sitting: 5 10 20"* line set the
/// three numbers in this colour. Drawing it is how the cost of that choice becomes visible — §5
/// exists to stop a dormant accent acquiring a caller because the colour is there.
///
/// [ADR-0030 §5]: ../../../../docs/adr/0030-the-first-finish-pass-decisions.md
const LICHEN: Color32 = Color32::from_rgb(0x6f, 0x93, 0xa8);

/// The card the whole Review screen is about. `chien`/`dog` — the seed's first note, and the pair
/// every capture of this screen has used, so the controls are the only thing that has changed.
const PROMPT: &str = "chien";
const ANSWER: &str = "dog";

// --- the axes ------------------------------------------------------------------------------------

/// What a control is made of. **The ticket's central question**, per ADR-0033 §3.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Weight {
    /// Today: the `inactive` widget fill, a slab.
    Solid,
    /// The rung between — `faint_bg_color`, still a surface, a much shallower one.
    Faint,
    /// No fill, a 1px edge. Same size, same hit target — the treatment ADR-0033 §3 photographed.
    Quiet,
}

impl Weight {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "solid" => Self::Solid,
            "faint" => Self::Faint,
            "quiet" => Self::Quiet,
            other => panic!("unknown PROTO_WEIGHT {other:?} — one of solid, faint, quiet"),
        }
    }

    fn fill(self) -> Color32 {
        match self {
            Self::Solid => STONE_5,
            Self::Faint => STONE_3,
            Self::Quiet => Color32::TRANSPARENT,
        }
    }

    fn stroke(self) -> Stroke {
        match self {
            Self::Solid => Stroke::new(1.0, QUIET_STROKE),
            Self::Faint => Stroke::new(1.0, STONE_4),
            Self::Quiet => Stroke::new(1.0, STONE_4),
        }
    }
}

/// How the four grades are arranged.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Grades {
    /// Today: four full-width controls stacked, a 3-unit break after *Forgot*.
    Stacked,
    /// #124's choice: *Forgot* held apart, the three passes in one segmented row.
    Row,
    /// All four in one row — the arrangement the choice rejects, drawn so the rejection is visible.
    Row4,
    /// *Forgot* apart and **four** passes in the row: the ticket's *"does the row survive a fourth
    /// pass grade"*, made a picture rather than an arithmetic worry.
    RowPlus,
}

impl Grades {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "stacked" => Self::Stacked,
            "row" => Self::Row,
            "row4" => Self::Row4,
            "rowplus" => Self::RowPlus,
            other => panic!("unknown PROTO_GRADES {other:?} — one of stacked, row, row4, rowplus"),
        }
    }
}

/// What the interval preview is. The baseline sets the grade's name and its `1d` at the **same**
/// size and colour, which is what makes two grades that happen to share `1d` read as a fault
/// rather than as two different answers to the same card.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Preview {
    /// Today: same size, same colour, separated by a `·`.
    Same,
    /// #124's: the small tier, dimmed. The button says *Good* first and `2d` second.
    Small,
    /// Gone. ADR-0006 §4 records the preview as *wanted* information confirmed live, so this is
    /// not a candidate — it is the control that shows what the preview is worth.
    None,
}

impl Preview {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "same" => Self::Same,
            "small" => Self::Small,
            "none" => Self::None,
            other => panic!("unknown PROTO_PREVIEW {other:?} — one of same, small, none"),
        }
    }
}

/// How a sitting is entered.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entrance {
    /// Today: a sentence, then a wrapped row of `5` `10` `20` `All 5` — four equal buttons, no
    /// primary among them, and the choice made before anything has been seen.
    Counts,
    /// #124's variant E: one primary way in, the sizes as a quiet second line in **weak text**.
    Primary,
    /// Variant E **as it was actually drawn** — the second line's numbers in the dormant link
    /// accent. See [`LICHEN`].
    PrimaryLink,
    /// No size choice at all: one control, and the sitting runs the queue. The ticket's deeper
    /// question — *is the count picker the right entrance at all* — drawn rather than argued.
    Plain,
}

impl Entrance {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "counts" => Self::Counts,
            "primary" => Self::Primary,
            "primarylink" => Self::PrimaryLink,
            "plain" => Self::Plain,
            other => panic!(
                "unknown PROTO_ENTRANCE {other:?} — one of counts, primary, primarylink, plain"
            ),
        }
    }
}

/// What a caught-up Review screen is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Empty {
    /// Today: one body sentence under the heading, and the durable leech entrance below it.
    Sentence,
    /// #124's variant E: the statement given the screen and centred, the entrance kept below it.
    Centred,
    /// Centred **without** the entrance — what three of #124's five variants drew, with nothing
    /// failing. On a caught-up Review the entrance is the only control on the screen (ADR-0010 §6,
    /// §8), so this is a picture of the screen losing its last affordance.
    Bare,
    /// `Centred`, with the statement on the **display** tier.
    ///
    /// Round one drew it at `HEADING`, which is the size of the *"Review"* label three lines above
    /// it — so the screen's whole content and the screen's name are set identically. #124's variant
    /// E used `display * 0.6` = 24px, and ADR-0032 fixed four sizes with nothing between 20 and 40,
    /// so 24 is not available to the application. This is the other end of that choice.
    Display,
}

impl Empty {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "sentence" => Self::Sentence,
            "centred" => Self::Centred,
            "bare" => Self::Bare,
            "display" => Self::Display,
            other => {
                panic!("unknown PROTO_EMPTY {other:?} — one of sentence, centred, bare, display")
            }
        }
    }
}

/// Which screen is under the camera.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// A revealed card with its grades — where `PROTO_GRADES`, `PROTO_WEIGHT` and `PROTO_PREVIEW`
    /// are judged, and the one screen ADR-0033 §3's comparison is about.
    Revealed,
    /// The entrance.
    Picker,
    /// Caught up: nothing due.
    CaughtUp,
    /// The end-of-session pointer (ADR-0010 §6) — never captured on `main`.
    Pointer,
    /// The 10-minute checkpoint (ADR-0006 §1) — never captured on `main`.
    Checkpoint,
    /// A sitting run with a hand on the mouse, entrance to floor.
    Live,
}

impl Screen {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "revealed" => Self::Revealed,
            "picker" => Self::Picker,
            "caughtup" => Self::CaughtUp,
            "pointer" => Self::Pointer,
            "checkpoint" => Self::Checkpoint,
            "live" => Self::Live,
            other => panic!(
                "unknown PROTO_SCREEN {other:?} — one of revealed, picker, caughtup, pointer, \
                 checkpoint, live"
            ),
        }
    }
}

/// What the 10-minute checkpoint is, and where it sits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Checkpoint {
    /// What ships: an `else if` branch that draws the checkpoint **instead of** the card.
    Replaces,
    /// What [ADR-0006 §1] says: the checkpoint surfaces and the card underneath stays gradeable.
    /// Two full-width controls above the card, which is the literal reading — and the picture shows
    /// what it costs.
    ///
    /// [ADR-0006 §1]: ../../../../docs/adr/0006-the-review-session-experience.md
    Over,
    /// The same rule, drawn as an aside rather than as a decision: the sentence and two compact
    /// controls on **one line**, above an unmoved card. Same guarantee, a fifth of the mass.
    Compact,
}

impl Checkpoint {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "replaces" => Self::Replaces,
            "over" => Self::Over,
            "compact" => Self::Compact,
            other => panic!("unknown PROTO_CHECKPOINT {other:?} — one of replaces, over, compact"),
        }
    }

    fn keeps_the_card(self) -> bool {
        self != Self::Replaces
    }
}

#[derive(Clone, Copy)]
struct Options {
    screen: Screen,
    grades: Grades,
    weight: Weight,
    preview: Preview,
    entrance: Entrance,
    empty: Empty,
    control: f32,
    checkpoint: Checkpoint,
    /// Whether the **primary** control on a screen with no card keeps a fill.
    ///
    /// The axis round one produced rather than proposed. [ADR-0033 §3] is a *relationship* — the
    /// controls are quieter **than the card** — and round one applied it as a *material*, giving
    /// every control on every screen the same outline. On the revealed card that is exactly right.
    /// On the picker and the caught-up screen, which have no card at all, it leaves a page whose
    /// only mass is a faint rectangle that reads as disabled.
    ///
    /// [ADR-0033 §3]: ../../../../docs/adr/0033-the-card.md
    primary_filled: bool,
    /// Whether *Edit note* is drawn as a control or as a **tertiary** frameless action.
    ///
    /// Also from round one: at the quiet weight *Edit note* becomes indistinguishable from a grade,
    /// so the screen offers five identical rectangles of which four commit a grading and one does
    /// not. Solid hid this, because the shipped screen's grades are separated from it by a gap and
    /// nothing else.
    edit_tertiary: bool,
}

// --- the controls --------------------------------------------------------------------------------

/// One control, at the prototype's chosen weight and height.
///
/// Every state is set explicitly rather than left to `Visuals`, because `quiet` is only honest if
/// hovering it does not silently restore a fill — and a still cannot show that, so it would go
/// unnoticed until the `live` screen. The hover cue is the **stroke**, which is the only thing a
/// control with no fill has to change.
fn control(ui: &mut egui::Ui, o: Options, job: LayoutJob, width: f32) -> egui::Response {
    control_at(ui, o, o.weight, job, width)
}

/// A **primary** control: [`control`], except that `PROTO_PRIMARY=filled` overrides the weight.
/// Used only where there is no card on the screen for ADR-0033 §3's comparison to be about.
fn primary(ui: &mut egui::Ui, o: Options, job: LayoutJob, width: f32) -> egui::Response {
    let weight = if o.primary_filled {
        Weight::Solid
    } else {
        o.weight
    };
    control_at(ui, o, weight, job, width)
}

/// A **tertiary** action: the label alone, at the same height so the hit target is unchanged.
fn tertiary(ui: &mut egui::Ui, o: Options, job: LayoutJob, width: f32) -> egui::Response {
    ui.add_sized([width, o.control], egui::Button::new(job).frame(false))
}

fn control_at(
    ui: &mut egui::Ui,
    o: Options,
    weight: Weight,
    job: LayoutJob,
    width: f32,
) -> egui::Response {
    let button = egui::Button::new(job)
        .corner_radius(CornerRadius::same(2))
        .fill(weight.fill())
        .stroke(weight.stroke());
    let response = ui.add_sized([width, o.control], button);
    if response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            CornerRadius::same(2),
            Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    response
}

/// A control's label, and the interval preview beside it when there is one.
fn grade_job(ui: &egui::Ui, o: Options, name: &str, days: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        name,
        0.0,
        egui::TextFormat {
            font_id: TextStyle::Button.resolve(ui.style()),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    // An empty `days` is how every control that is *not* a grade reaches this function — `Edit
    // note`, `Start`, the leech entrance. They carry no interval, so the preview axis does not
    // reach them, and without this line `Preview::Same` draws them a bare separator.
    if days.is_empty() {
        return job;
    }
    match o.preview {
        Preview::None => {}
        Preview::Same => job.append(
            &format!("   ·   {days}"),
            0.0,
            egui::TextFormat {
                font_id: TextStyle::Button.resolve(ui.style()),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        ),
        Preview::Small => job.append(
            days,
            spacing::gap(1),
            egui::TextFormat {
                font_id: FontId::proportional(typography::SMALL),
                color: ui.visuals().weak_text_color(),
                ..Default::default()
            },
        ),
    }
    job
}

/// The grades a candidate draws: the real four, plus the hypothetical fifth `rowplus` exists to
/// test. The intervals are the ones the shipped preview produces for a new card on its first pass,
/// which is why two of them read `1d` — the collision that makes `Preview::Same` a problem.
fn grade_set(g: Grades) -> (&'static [(&'static str, &'static str)], bool) {
    const FOUR: [(&str, &str); 4] = [
        ("Forgot", "1d"),
        ("Barely", "1d"),
        ("Good", "2d"),
        ("Easy", "4d"),
    ];
    const FIVE: [(&str, &str); 5] = [
        ("Forgot", "1d"),
        ("Barely", "1d"),
        ("Good", "2d"),
        ("Easy", "4d"),
        ("Trivial", "7d"),
    ];
    match g {
        Grades::RowPlus => (&FIVE, true),
        Grades::Row => (&FOUR, true),
        Grades::Row4 => (&FOUR, false),
        Grades::Stacked => (&FOUR, false),
    }
}

/// The grade cluster. Returns the grade pressed, if any.
fn grades(ui: &mut egui::Ui, o: Options) -> Option<&'static str> {
    let (set, forgot_apart) = grade_set(o.grades);
    let mut pressed = None;

    if o.grades == Grades::Stacked {
        for (i, (name, days)) in set.iter().enumerate() {
            let job = grade_job(ui, o, name, days);
            if control(ui, o, job, ui.available_width()).clicked() {
                pressed = Some(*name);
            }
            // Three units hold the passes apart from *Forgot*; one holds them apart from each
            // other. The grouping the shipped screen already expresses.
            if i + 1 < set.len() {
                ui.add_space(spacing::gap(if i == 0 { 3 } else { 1 }));
            }
        }
        return pressed;
    }

    let row: &[(&str, &str)] = if forgot_apart {
        let job = grade_job(ui, o, set[0].0, set[0].1);
        if control(ui, o, job, ui.available_width()).clicked() {
            pressed = Some(set[0].0);
        }
        ui.add_space(spacing::gap(2));
        &set[1..]
    } else {
        set
    };

    // `n` controls and `n - 1` gaps. A trailing gap after the last one pushes the row *past* the
    // column, egui grows `max_rect` to fit it, and every control drawn afterwards is then one gap
    // wider than the row above — which reads as a misalignment bug rather than as a design. #124
    // found this the hard way; it is repeated here because the arithmetic is easy to get wrong once
    // per prototype.
    let gap = spacing::gap(1);
    let each = (ui.available_width() - gap * (row.len() as f32 - 1.0)) / row.len() as f32;
    spacing::row(ui, 1, |ui| {
        for (name, days) in row {
            let job = grade_job(ui, o, name, days);
            if control(ui, o, job, each).clicked() {
                pressed = Some(*name);
            }
        }
    });
    pressed
}

/// A body sentence.
fn body(ui: &mut egui::Ui, s: &str) {
    ui.label(
        egui::RichText::new(s)
            .font(TextStyle::Body.resolve(ui.style()))
            .color(ui.visuals().text_color()),
    );
}

/// A small, weak line.
fn small(ui: &mut egui::Ui, s: &str) {
    ui.label(
        egui::RichText::new(s)
            .font(FontId::proportional(typography::SMALL))
            .color(ui.visuals().weak_text_color()),
    );
}

fn heading(ui: &mut egui::Ui, s: &str) {
    ui.label(
        egui::RichText::new(s)
            .font(TextStyle::Heading.resolve(ui.style()))
            .color(ui.visuals().text_color()),
    );
}

/// The entrance's quiet second line: the sizes, offered without being the way in.
fn size_line(ui: &mut egui::Ui, o: Options) -> bool {
    let mut started = false;
    let colour = if o.entrance == Entrance::PrimaryLink {
        LICHEN
    } else {
        ui.visuals().weak_text_color()
    };
    spacing::row(ui, 2, |ui| {
        small(ui, "or a shorter sitting:");
        for option in ["5", "10", "20"] {
            let job = LayoutJob::single_section(
                option.to_owned(),
                egui::TextFormat {
                    font_id: FontId::proportional(typography::SMALL),
                    color: colour,
                    ..Default::default()
                },
            );
            if ui.add(egui::Button::new(job).frame(false)).clicked() {
                started = true;
            }
        }
    });
    started
}

/// The entrance. `available` is what the queue holds — six in the seed, which is
/// `DEFAULT_NEW_CARD_RATE + 1`.
fn entrance(ui: &mut egui::Ui, o: Options, available: usize) -> bool {
    let mut started = false;
    match o.entrance {
        Entrance::Counts => {
            // Four equal controls and no primary among them — which is the arrangement, not an
            // omission: today the picker has no way in that is *the* way in.
            spacing::row_wrapped(ui, 1, |ui| {
                for option in [5usize, 10, 20] {
                    if option <= available {
                        let job = grade_job(ui, o, &option.to_string(), "");
                        if control(ui, o, job, 60.0).clicked() {
                            started = true;
                        }
                    }
                }
                let job = grade_job(ui, o, &format!("All {available}"), "");
                if control(ui, o, job, 96.0).clicked() {
                    started = true;
                }
            });
        }
        Entrance::Primary | Entrance::PrimaryLink => {
            let job = grade_job(ui, o, &format!("Start — all {available}"), "");
            if primary(ui, o, job, ui.available_width()).clicked() {
                started = true;
            }
            ui.add_space(spacing::gap(2));
            started |= size_line(ui, o);
        }
        Entrance::Plain => {
            let job = grade_job(ui, o, "Start reviewing", "");
            if primary(ui, o, job, ui.available_width()).clicked() {
                started = true;
            }
        }
    }
    started
}

/// The caught-up screen, and the one control it has.
fn caught_up(ui: &mut egui::Ui, o: Options) {
    match o.empty {
        Empty::Sentence => {
            body(ui, "All caught up — nothing is due right now.");
            ui.add_space(spacing::gap(3));
            let job = grade_job(ui, o, "Leeches (2) · suspended (1)", "");
            primary(ui, o, job, ui.available_width());
        }
        Empty::Centred | Empty::Bare | Empty::Display => {
            let tier = if o.empty == Empty::Display {
                typography::DISPLAY
            } else {
                typography::HEADING
            };
            ui.add_space(spacing::gap(8));
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("All caught up.")
                        .font(FontId::proportional(tier))
                        .color(ui.visuals().text_color()),
                );
                ui.add_space(spacing::gap(2));
                small(ui, "Nothing is due right now.");
            });
            if o.empty != Empty::Bare {
                ui.add_space(spacing::gap(5));
                let job = grade_job(ui, o, "Leeches (2) · suspended (1)", "");
                primary(ui, o, job, ui.available_width());
            }
        }
    }
}

/// The end-of-session pointer (ADR-0010 §6): a plain statement of cost and an offer to look. Two
/// full-width controls today, which gives *Show me* and *Not now* the same weight — and one of them
/// is a dismissal.
fn pointer(ui: &mut egui::Ui, o: Options) {
    body(ui, "2 cards are costing you a lot. Take a look?");
    ui.add_space(spacing::gap(3));
    let job = grade_job(ui, o, "Show me", "");
    control(ui, o, job, ui.available_width());
    ui.add_space(spacing::gap(1));
    let job = grade_job(ui, o, "Not now", "");
    control(ui, o, job, ui.available_width());
}

/// The 10-minute checkpoint (ADR-0006 §1).
fn checkpoint(ui: &mut egui::Ui, o: Options) {
    if o.checkpoint == Checkpoint::Compact {
        // A courtesy check-in, drawn as one. ADR-0006 §1 calls the timer *"a courtesy check-in, not
        // an enforcement mechanism"* — and a stack of full-width controls is how an application
        // draws an enforcement.
        spacing::row(ui, 2, |ui| {
            small(ui, "10 minutes so far.");
            let job = grade_job(ui, o, "Finish here", "");
            tertiary(ui, o, job, 100.0);
            let job = grade_job(ui, o, "Keep going", "");
            tertiary(ui, o, job, 100.0);
        });
        return;
    }
    body(ui, "You've been reviewing for 10 minutes.");
    ui.add_space(spacing::gap(2));
    let job = grade_job(ui, o, "Finish here", "");
    control(ui, o, job, ui.available_width());
    ui.add_space(spacing::gap(1));
    let job = grade_job(ui, o, "Keep going", "");
    control(ui, o, job, ui.available_width());
}

/// *Edit note* — a control, or a tertiary action that stops reading as a fifth grade.
fn edit_note(ui: &mut egui::Ui, o: Options) {
    if o.edit_tertiary {
        ui.vertical_centered(|ui| {
            let job = grade_job(ui, o, "Edit note", "");
            tertiary(ui, o, job, 120.0);
        });
    } else {
        let job = grade_job(ui, o, "Edit note", "");
        control(ui, o, job, ui.available_width());
    }
}

/// The card, drawn by the application's own `surface::card`.
fn card(ui: &mut egui::Ui, revealed: bool) -> egui::Response {
    surface::card(
        ui,
        PROMPT,
        revealed.then_some(ANSWER),
        revealed.then_some("new"),
        surface::REVIEW_HEIGHT,
    )
}

// --- the screens ---------------------------------------------------------------------------------

/// A running sitting, driven by clicking. The one thing a still cannot show is whether the screen
/// **moves** underneath you when the grades appear — a stacked column and a segmented row free
/// different amounts of vertical space, and the card's position at the reveal is the thing to
/// watch.
#[derive(Default)]
struct Live {
    started: bool,
    revealed: bool,
    graded: usize,
    checkpoint: bool,
}

fn draw(ui: &mut egui::Ui, o: Options, live: &mut Live) {
    heading(ui, "Review");
    ui.add_space(spacing::gap(2));

    match o.screen {
        Screen::Picker => {
            body(
                ui,
                "A fresh deck. These cards are new — start whenever you like.",
            );
            ui.add_space(spacing::gap(3));
            entrance(ui, o, 6);
        }
        Screen::CaughtUp => caught_up(ui, o),
        Screen::Pointer => pointer(ui, o),
        Screen::Checkpoint => {
            checkpoint(ui, o);
            // What ships draws nothing else — the checkpoint replaces the card entirely, which is
            // the defect. The other two keep it gradeable, per ADR-0006 §1.
            if o.checkpoint.keeps_the_card() {
                ui.add_space(spacing::gap(3));
                body(ui, "3 of 10");
                ui.add_space(spacing::gap(2));
                card(ui, true);
                ui.add_space(spacing::gap(3));
                grades(ui, o);
            }
        }
        Screen::Revealed => {
            body(ui, "3 of 10");
            ui.add_space(spacing::gap(2));
            card(ui, true);
            ui.add_space(spacing::gap(3));
            grades(ui, o);
            ui.add_space(spacing::gap(3));
            edit_note(ui, o);
        }
        Screen::Live => live_screen(ui, o, live),
    }
}

fn live_screen(ui: &mut egui::Ui, o: Options, live: &mut Live) {
    if !live.started {
        body(
            ui,
            "A fresh deck. These cards are new — start whenever you like.",
        );
        ui.add_space(spacing::gap(3));
        if entrance(ui, o, 6) {
            live.started = true;
        }
        return;
    }

    if live.graded >= 6 {
        caught_up(ui, o);
        ui.add_space(spacing::gap(3));
        if ui
            .add(egui::Button::new("Run it again").frame(false))
            .clicked()
        {
            *live = Live::default();
        }
        return;
    }

    // The checkpoint is reachable in `live` by clicking it on rather than by waiting ten minutes —
    // a capture run settles for four seconds, and nobody judges a design by sitting through a
    // timer. What is being judged is the *arrangement* at the moment it appears.
    if live.checkpoint {
        checkpoint(ui, o);
        if !o.checkpoint.keeps_the_card() {
            return;
        }
        ui.add_space(spacing::gap(3));
    }

    body(ui, &format!("{} of 6", live.graded));
    ui.add_space(spacing::gap(2));
    if card(ui, live.revealed).clicked() {
        live.revealed = true;
    }
    if live.revealed {
        ui.add_space(spacing::gap(3));
        // Any grade advances the sitting. The prototype does not schedule, so *which* grade was
        // pressed is not information it can act on — what is being judged is the arrangement and
        // whether the screen moves underneath the hand between one card and the next.
        let pressed = grades(ui, o).is_some();
        ui.add_space(spacing::gap(3));
        edit_note(ui, o);
        if pressed {
            live.graded += 1;
            live.revealed = false;
        }
    }
    ui.add_space(spacing::gap(3));
    if ui
        .add(egui::Button::new("(toggle checkpoint)").frame(false))
        .clicked()
    {
        live.checkpoint = !live.checkpoint;
    }
}

// --- the shell -----------------------------------------------------------------------------------

struct Prototype {
    options: Options,
    live: Live,
    fonts_installed: bool,
}

impl eframe::App for Prototype {
    /// The page is `panel_fill` — ADR-0033 §2, and the same override the application now has.
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The font set applies at the start of the *next* pass, so this frame draws nothing
        // (ADR-0012 §8).
        if !self.fonts_installed {
            fonts::install(ui.ctx());
            self.fonts_installed = true;
            ui.ctx().request_repaint();
            return;
        }
        ui.add_space(spacing::gap(1));
        let options = self.options;
        frame::column(ui, |ui| draw(ui, options, &mut self.live));
    }
}

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn main() -> eframe::Result<()> {
    let options = Options {
        screen: Screen::parse(&env("PROTO_SCREEN", "revealed")),
        grades: Grades::parse(&env("PROTO_GRADES", "row")),
        weight: Weight::parse(&env("PROTO_WEIGHT", "quiet")),
        preview: Preview::parse(&env("PROTO_PREVIEW", "small")),
        entrance: Entrance::parse(&env("PROTO_ENTRANCE", "primary")),
        empty: Empty::parse(&env("PROTO_EMPTY", "centred")),
        control: env("PROTO_CONTROL", "36")
            .trim()
            .parse()
            .expect("PROTO_CONTROL must be a number of pixels"),
        checkpoint: Checkpoint::parse(&env("PROTO_CHECKPOINT", "replaces")),
        primary_filled: env("PROTO_PRIMARY", "quiet").trim() == "filled",
        edit_tertiary: env("PROTO_EDIT", "control").trim() == "tertiary",
    };

    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([560.0, 860.0])
            .with_title("Cairn"),
        ..Default::default()
    };
    eframe::run_native(
        "Cairn",
        native,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            typography::install(&cc.egui_ctx);
            spacing::install(&cc.egui_ctx);
            Ok(Box::new(Prototype {
                options,
                live: Live::default(),
                fonts_installed: false,
            }))
        }),
    )
}
