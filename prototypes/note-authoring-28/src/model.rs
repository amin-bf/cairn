//! PROTOTYPE model — throwaway. Answers #28 only. See PROTOTYPE.md.
//!
//! ADR-0002's card model, reduced to the parts an authoring screen has to show: kind definitions
//! as data (§4), the `asked` / `shown-with` roles (§3), cloze blanks numbered by the author (§5),
//! and card identity as `(note, ordinal)` (§6). Nothing here is production code — the real thing
//! lives in `leitner-core`.
//!
//! The one thing this module takes seriously is that **a card's ordinal is half its identity**, so
//! a draft that stops generating an ordinal is a draft that puts that card's review history to
//! sleep (§7). Every variant needs that fact; only the presentation differs.

use std::collections::BTreeMap;

pub type Values = BTreeMap<String, String>;

/// ADR-0002 §3. A field either takes part in a card's prompt/answer, or it rides along with one
/// that does and is never asked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Asked,
    ShownWith(&'static str),
}

pub struct FieldDef {
    pub name: &'static str,
    pub role: Role,
    /// Editor affordance only — how tall to draw the input. Not part of ADR-0002.
    pub multiline: bool,
}

pub struct CardDef {
    pub prompt: &'static [&'static str],
    pub answer: &'static [&'static str],
}

/// ADR-0002 §4: layout is data, stored once per kind, read-only, and carried in exports.
pub struct KindDef {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub fields: &'static [FieldDef],
    /// Empty for `cloze`, whose card set comes from the content instead (§5).
    pub cards: &'static [CardDef],
}

impl KindDef {
    pub fn is_cloze(&self) -> bool {
        self.id == "cloze"
    }

    pub fn field(&self, name: &str) -> Option<&'static FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Fields that ride along with `anchor`, in declaration order (§3).
    pub fn shown_with(&self, anchor: &str) -> Vec<&'static FieldDef> {
        self.fields
            .iter()
            .filter(|f| matches!(f.role, Role::ShownWith(a) if a == anchor))
            .collect()
    }
}

pub static KINDS: &[KindDef] = &[
    KindDef {
        id: "basic",
        label: "Basic",
        blurb: "One card: front asks, back answers.",
        fields: &[
            FieldDef { name: "Front", role: Role::Asked, multiline: true },
            FieldDef { name: "Back", role: Role::Asked, multiline: true },
        ],
        cards: &[CardDef { prompt: &["Front"], answer: &["Back"] }],
    },
    KindDef {
        id: "basic-reverse",
        label: "Basic + reverse",
        blurb: "Two cards, one per direction.",
        fields: &[
            FieldDef { name: "Front", role: Role::Asked, multiline: true },
            FieldDef { name: "Back", role: Role::Asked, multiline: true },
        ],
        cards: &[
            CardDef { prompt: &["Front"], answer: &["Back"] },
            CardDef { prompt: &["Back"], answer: &["Front"] },
        ],
    },
    KindDef {
        id: "vocab",
        label: "Vocabulary",
        blurb: "Two cards. Pronunciation and example follow the term, and are never asked.",
        fields: &[
            FieldDef { name: "Term", role: Role::Asked, multiline: false },
            FieldDef { name: "Meaning", role: Role::Asked, multiline: false },
            FieldDef { name: "Pronunciation", role: Role::ShownWith("Term"), multiline: false },
            FieldDef { name: "Example", role: Role::ShownWith("Term"), multiline: true },
        ],
        cards: &[
            CardDef { prompt: &["Term"], answer: &["Meaning"] },
            CardDef { prompt: &["Meaning"], answer: &["Term"] },
        ],
    },
    KindDef {
        id: "cloze",
        label: "Cloze",
        blurb: "One card per numbered blank. You choose the numbers; nothing renumbers them.",
        fields: &[FieldDef { name: "Text", role: Role::Asked, multiline: true }],
        cards: &[],
    },
];

pub fn kind(id: &str) -> &'static KindDef {
    KINDS.iter().find(|k| k.id == id).unwrap_or(&KINDS[0])
}

// ---------------------------------------------------------------------------------------------
// Cloze blanks — ADR-0002 §5
// ---------------------------------------------------------------------------------------------

/// One piece of a cloze `Text` field: either literal text, or a `{{n::hidden}}` blank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Blank { n: u16, inner: String },
}

/// Parses `{{n::text}}`. Anything malformed stays literal text — the editor must never silently
/// "fix" a half-typed blank, since the number is load-bearing identity.
pub fn parse_cloze(s: &str) -> Vec<Piece> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut lit = String::new();
    let mut i = 0usize;
    while i < b.len() {
        if b[i..].starts_with(b"{{") {
            if let Some(rel_end) = s[i + 2..].find("}}") {
                let body = &s[i + 2..i + 2 + rel_end];
                if let Some((num, inner)) = body.split_once("::") {
                    if let Ok(n) = num.trim().parse::<u16>() {
                        if n > 0 {
                            if !lit.is_empty() {
                                out.push(Piece::Text(std::mem::take(&mut lit)));
                            }
                            out.push(Piece::Blank { n, inner: inner.to_string() });
                            i += 2 + rel_end + 2;
                            continue;
                        }
                    }
                }
            }
        }
        // Push one whole character, never one byte — slicing mid-codepoint panics on Persian.
        let ch_len = s[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        lit.push_str(&s[i..i + ch_len]);
        i += ch_len;
    }
    if !lit.is_empty() {
        out.push(Piece::Text(lit));
    }
    out
}

/// The distinct blank numbers in a cloze text, ascending. Gaps are normal and are preserved
/// exactly as authored (§5) — this never renumbers, and never fills a hole.
pub fn blank_numbers(text: &str) -> Vec<u16> {
    let mut ns: Vec<u16> = parse_cloze(text)
        .into_iter()
        .filter_map(|p| match p {
            Piece::Blank { n, .. } => Some(n),
            _ => None,
        })
        .collect();
    ns.sort_unstable();
    ns.dedup();
    ns
}

/// How many times each number occurs — the same number may appear more than once, hiding every
/// occurrence on one card (§5).
pub fn blank_occurrences(text: &str, n: u16) -> usize {
    parse_cloze(text).iter().filter(|p| matches!(p, Piece::Blank { n: m, .. } if *m == n)).count()
}

/// The text a blank hides, for listing the set at a glance. First occurrence wins.
pub fn blank_inner(text: &str, n: u16) -> String {
    parse_cloze(text)
        .into_iter()
        .find_map(|p| match p {
            Piece::Blank { n: m, inner } if m == n => Some(inner),
            _ => None,
        })
        .unwrap_or_default()
}

/// The number a *new* blank gets: one above the highest ever used in this text.
///
/// Deliberately **not** "the lowest unused number". Filling a gap left by a deleted blank would
/// hand the new blank the identity of the old one, and every review of the deleted card would
/// silently reattach to different content (§5, §7) — the exact damage auto-renumbering does, just
/// arriving one edit later.
pub fn next_blank_number(text: &str) -> u16 {
    blank_numbers(text).last().copied().unwrap_or(0) + 1
}

// ---------------------------------------------------------------------------------------------
// Card generation — ADR-0002 §1, §3, §6
// ---------------------------------------------------------------------------------------------

/// One rendered line of a card side: a field's value, plus whether it is the asked field or a
/// `shown-with` passenger (§3).
#[derive(Clone, Debug)]
pub struct SideLine {
    pub field: String,
    pub text: String,
    pub passenger: bool,
}

/// A card as the note currently generates it. `ordinal` is half its identity (§6).
#[derive(Clone, Debug)]
pub struct GenCard {
    pub ordinal: u16,
    /// Human label — "Term → Meaning" for fixed-arity kinds, "blank 2" for cloze.
    pub label: String,
    pub prompt: Vec<SideLine>,
    pub answer: Vec<SideLine>,
    /// Cloze only: the whole text, so the variant can render it with blank `ordinal` hidden.
    pub cloze_text: Option<String>,
}

fn side(k: &KindDef, values: &Values, names: &[&'static str]) -> Vec<SideLine> {
    let mut out = Vec::new();
    for name in names {
        out.push(SideLine {
            field: (*name).to_string(),
            text: values.get(*name).cloned().unwrap_or_default(),
            passenger: false,
        });
        // §3's rendering rule: passengers follow their anchor to whichever side it landed on.
        for p in k.shown_with(name) {
            out.push(SideLine {
                field: p.name.to_string(),
                text: values.get(p.name).cloned().unwrap_or_default(),
                passenger: true,
            });
        }
    }
    out
}

pub fn generate(kind_id: &str, values: &Values) -> Vec<GenCard> {
    let k = kind(kind_id);
    if k.is_cloze() {
        let text = values.get("Text").cloned().unwrap_or_default();
        return blank_numbers(&text)
            .into_iter()
            .map(|n| GenCard {
                ordinal: n,
                label: format!("blank {n}"),
                prompt: Vec::new(),
                answer: Vec::new(),
                cloze_text: Some(text.clone()),
            })
            .collect();
    }
    k.cards
        .iter()
        .enumerate()
        .map(|(i, c)| GenCard {
            ordinal: i as u16,
            label: format!("{} → {}", c.prompt.join(" + "), c.answer.join(" + ")),
            prompt: side(k, values, c.prompt),
            answer: side(k, values, c.answer),
            cloze_text: None,
        })
        .collect()
}

pub fn ordinals(kind_id: &str, values: &Values) -> Vec<u16> {
    generate(kind_id, values).into_iter().map(|c| c.ordinal).collect()
}

// ---------------------------------------------------------------------------------------------
// Review history — the stakes behind a destructive edit
// ---------------------------------------------------------------------------------------------

/// Reviews already logged against `(this note, ordinal)`. In the real app this is a replay result,
/// not stored state; here it is a hard-coded table, because the point is what the *editor* says.
#[derive(Clone, Copy, Debug)]
pub struct History {
    pub ordinal: u16,
    pub reviews: u32,
    pub box_num: u8,
}

/// A card with history that the current draft no longer generates — ADR-0002 §7's dormant card.
/// Its events stay in the log, project onto nothing, and reattach by themselves if the content
/// comes back.
#[derive(Clone, Copy, Debug)]
pub struct Dormant {
    pub ordinal: u16,
    pub reviews: u32,
    pub box_num: u8,
}

pub fn dormant(history: &[History], live: &[u16]) -> Vec<Dormant> {
    history
        .iter()
        .filter(|h| h.reviews > 0 && !live.contains(&h.ordinal))
        .map(|h| Dormant { ordinal: h.ordinal, reviews: h.reviews, box_num: h.box_num })
        .collect()
}

pub fn history_for(history: &[History], ordinal: u16) -> Option<History> {
    history.iter().copied().find(|h| h.ordinal == ordinal)
}

// ---------------------------------------------------------------------------------------------
// Scenarios — the data conditions each variant is judged against
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scenario {
    NewNote,
    Vocab,
    Cloze,
    Persian,
    KindChange,
}

impl Scenario {
    pub const ALL: [Scenario; 5] =
        [Scenario::NewNote, Scenario::Vocab, Scenario::Cloze, Scenario::Persian, Scenario::KindChange];

    pub fn label(self) -> &'static str {
        match self {
            Scenario::NewNote => "New note",
            Scenario::Vocab => "Vocab",
            Scenario::Cloze => "Cloze + history",
            Scenario::Persian => "Persian (RTL)",
            Scenario::KindChange => "Kind change",
        }
    }

    /// What this scenario exists to put in front of the judge.
    pub fn note(self) -> &'static str {
        match self {
            Scenario::NewNote => "Empty note, kind not yet chosen. Watch: is the kind decision in the way, or out of the way?",
            Scenario::Vocab => "Pronunciation and Example are shown-with(Term) — never asked. Does the editor make that visible?",
            Scenario::Cloze => "Blanks 1, 2, 4 — gap is normal, and 2 occurs twice. Blank 2 carries 40 reviews: delete it and see what the editor says.",
            Scenario::Persian => "RTL content in the editor. Caret and selection are imprecise here by construction (AGENTS.md rule 2).",
            Scenario::KindChange => "basic-reverse with history on both cards. Switch to Basic and card 1 goes dormant.",
        }
    }

    pub fn kind_id(self) -> &'static str {
        match self {
            Scenario::NewNote => "basic",
            Scenario::Vocab => "vocab",
            Scenario::Cloze => "cloze",
            Scenario::Persian => "vocab",
            Scenario::KindChange => "basic-reverse",
        }
    }

    pub fn values(self) -> Values {
        let mut v = Values::new();
        let mut set = |k: &str, s: &str| {
            v.insert(k.to_string(), s.to_string());
        };
        match self {
            Scenario::NewNote => {}
            Scenario::Vocab => {
                set("Term", "der Hund");
                set("Meaning", "the dog");
                set("Pronunciation", "deːɐ̯ hʊnt");
                set("Example", "Der **Hund** bellt im Garten.");
            }
            Scenario::Cloze => {
                set(
                    "Text",
                    "The {{1::mitochondria}} is the powerhouse of the {{2::cell}}.\n\nIt burns sugar to make {{4::ATP}}, which the {{2::cell}} then spends.",
                );
            }
            Scenario::Persian => {
                set("Term", "سگ");
                set("Meaning", "dog");
                set("Pronunciation", "sag");
                set("Example", "سگ در خانه است.");
            }
            Scenario::KindChange => {
                set("Front", "Ottawa");
                set("Back", "capital of **Canada**");
            }
        }
        v
    }

    pub fn tags(self) -> &'static str {
        match self {
            Scenario::Vocab => "german, animals, chapter-3",
            Scenario::Cloze => "biology",
            Scenario::Persian => "persian, animals",
            Scenario::KindChange => "geography",
            Scenario::NewNote => "",
        }
    }

    pub fn history(self) -> &'static [History] {
        match self {
            Scenario::NewNote => &[],
            Scenario::Vocab => &[
                History { ordinal: 0, reviews: 12, box_num: 3 },
                History { ordinal: 1, reviews: 5, box_num: 2 },
            ],
            Scenario::Cloze => &[
                History { ordinal: 1, reviews: 8, box_num: 2 },
                History { ordinal: 2, reviews: 40, box_num: 4 },
                History { ordinal: 4, reviews: 2, box_num: 1 },
            ],
            Scenario::Persian => &[History { ordinal: 0, reviews: 3, box_num: 1 }],
            Scenario::KindChange => &[
                History { ordinal: 0, reviews: 22, box_num: 4 },
                History { ordinal: 1, reviews: 17, box_num: 3 },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_numbered_blank() {
        assert_eq!(
            parse_cloze("a {{1::b}} c"),
            vec![
                Piece::Text("a ".into()),
                Piece::Blank { n: 1, inner: "b".into() },
                Piece::Text(" c".into())
            ]
        );
    }

    #[test]
    fn a_half_typed_blank_stays_literal() {
        // The editor must not guess at `{{1::` — an inferred number is an invented identity.
        assert_eq!(parse_cloze("{{1::x"), vec![Piece::Text("{{1::x".into())]);
        assert_eq!(parse_cloze("{{::x}}"), vec![Piece::Text("{{::x}}".into())]);
    }

    #[test]
    fn gaps_and_repeats_survive_exactly_as_authored() {
        let t = "{{1::a}} {{4::b}} {{1::c}}";
        assert_eq!(blank_numbers(t), vec![1, 4]);
        assert_eq!(blank_occurrences(t, 1), 2);
    }

    #[test]
    fn a_new_blank_never_fills_a_gap() {
        // Reusing 2 after it was deleted would hand the new blank the dead card's identity.
        assert_eq!(next_blank_number("{{1::a}} {{4::b}}"), 5);
    }

    #[test]
    fn multibyte_text_is_not_sliced_mid_codepoint() {
        assert_eq!(parse_cloze("سگ {{1::خانه}}").len(), 2);
    }

    #[test]
    fn a_passenger_field_follows_its_anchor_to_either_side() {
        let v = Scenario::Vocab.values();
        let cards = generate("vocab", &v);
        // Card 0 asks Term, so the pronunciation is on the prompt.
        assert!(cards[0].prompt.iter().any(|l| l.field == "Pronunciation"));
        assert!(!cards[0].answer.iter().any(|l| l.field == "Pronunciation"));
        // Card 1 answers with Term, so it moves to the answer — with no special-casing.
        assert!(cards[1].answer.iter().any(|l| l.field == "Pronunciation"));
        assert!(!cards[1].prompt.iter().any(|l| l.field == "Pronunciation"));
    }

    #[test]
    fn deleting_a_blank_puts_its_history_to_sleep() {
        let hist = Scenario::Cloze.history();
        let live = ordinals("cloze", &Scenario::Cloze.values());
        assert!(dormant(hist, &live).is_empty());

        let mut edited = Scenario::Cloze.values();
        edited.insert("Text".into(), "The {{1::mitochondria}} is the powerhouse.".into());
        let d = dormant(hist, &ordinals("cloze", &edited));
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].ordinal, 2);
        assert_eq!(d[0].reviews, 40);
    }

    #[test]
    fn changing_kind_can_retire_a_card_too() {
        let v = Scenario::KindChange.values();
        let d = dormant(Scenario::KindChange.history(), &ordinals("basic", &v));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].ordinal, 1);
        assert_eq!(d[0].reviews, 17);
    }
}
