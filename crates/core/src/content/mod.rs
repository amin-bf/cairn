//! See `CONTEXT.md` beside this file for the vocabulary, the binding ADR sections, and the rules
//! that break silently.
//!
//! This is the base context: `log`, `scheduling` and `replay` all depend on it, and it depends on
//! none of them. It carries the two things the rest of the domain names — a card's identity
//! (`CardRef`, ADR-0002 §6) and the kind definitions that say which cards a note generates
//! (ADR-0002 §1, ADR-0017 §1).
//!
//! The four shipped kinds and the slot namespace that gives their cards identity, per
//! [#81](https://github.com/amin-bf/leitner/issues/81): `basic`, `basic-reverse`, `vocab` and
//! `cloze`, drawing their slots from **one namespace shared by every kind** (ADR-0017 §1). `basic`
//! and `basic-reverse` share slot 0 for Front→Back deliberately, and cloze blanks are partitioned
//! above the high bit (`0x8000 | n`, ADR-0017 §3). The two tests that make a slot's immutability
//! enforceable — uniqueness across the shipped definitions and a golden `slot → (prompt, answer)`
//! list (ADR-0017 §4) — sit at the foot of this file.

/// The `position` order key: a fractional index with infill (ADR-0021 §3), the value that fixes a
/// note's place in authored order.
pub mod order;

/// A note's identity: sixteen bytes, minted once at creation as a UUIDv4 (ADR-0002 §6).
///
/// `leitner-core` never mints one — minting is a write-time act at the edge, and this crate takes
/// identity as a value (ADR-0009 §8). The bytes are stored in RFC 9562 order so that
/// [`CardRef::encode`] is a fixed, cross-device byte string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(pub [u8; 16]);

impl NoteId {
    /// Parse the RFC 9562 canonical text form — `8-4-4-4-12` lowercase or uppercase hex with four
    /// hyphens. Returns `None` for anything else, so a malformed `n` field cannot panic replay.
    ///
    /// This is the reverse of the `n` field in ADR-0004 §11's interchange line, which carries the
    /// note UUID as canonical text; the 18-byte [`CardRef::encode`] is rebuilt from it plus the
    /// ordinal.
    pub fn parse_canonical(text: &str) -> Option<Self> {
        uuid16_from_canonical(text).map(NoteId)
    }

    /// The RFC 9562 canonical text form, lowercase. The inverse of [`NoteId::parse_canonical`].
    pub fn to_canonical(&self) -> String {
        uuid16_to_canonical(&self.0)
    }
}

/// A deck's identity: sixteen bytes, minted once at creation as a UUIDv4 and **preserved through
/// export and import** (ADR-0005 §4) — what lets an import be recognised as an update to the same
/// deck rather than a new one, matched against your notes by note id so review history survives.
///
/// A deck is `{ id, name }` and nothing else (ADR-0005 §5): the name is a mutable, non-unique display
/// label, and no configuration ever lives here. Like [`NoteId`], `leitner-core` never mints one —
/// minting is a write-time act at the edge (ADR-0009 §8) — and the bytes are stored in RFC 9562 order
/// so the canonical text form is a fixed, cross-device string. A note's `deck` reference is this id's
/// [`DeckId::to_canonical`] text; a reference naming no held deck is **unfiled, never lost**
/// (ADR-0005 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckId(pub [u8; 16]);

impl DeckId {
    /// Parse the RFC 9562 canonical text form — the inverse of [`DeckId::to_canonical`]. Returns
    /// `None` for anything else, so a malformed stored `deck` reference cannot panic the note list.
    pub fn parse_canonical(text: &str) -> Option<Self> {
        uuid16_from_canonical(text).map(DeckId)
    }

    /// The RFC 9562 canonical text form, lowercase — the string a note's `deck` reference carries and
    /// the deck filter compares.
    pub fn to_canonical(&self) -> String {
        uuid16_to_canonical(&self.0)
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// The RFC 9562 canonical text form of sixteen bytes, lowercase — shared by [`NoteId`] and
/// [`DeckId`], whose ids are both UUIDv4s stored in RFC 9562 order (ADR-0002 §6, ADR-0005 §4).
pub(crate) fn uuid16_to_canonical(bytes: &[u8; 16]) -> String {
    let mut s = String::with_capacity(36);
    for (i, byte) in bytes.iter().enumerate() {
        if i == 4 || i == 6 || i == 8 || i == 10 {
            s.push('-');
        }
        s.push(char::from(HEX[usize::from(byte >> 4)]));
        s.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    s
}

/// Parse the RFC 9562 canonical `8-4-4-4-12` text form into sixteen bytes, case-insensitively;
/// `None` for anything else. The inverse of [`uuid16_to_canonical`], shared by [`NoteId`] and
/// [`DeckId`] so a malformed id token is rejected rather than panicking.
pub(crate) fn uuid16_from_canonical(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return None;
    }
    let mut out = [0u8; 16];
    let mut out_i = 0;
    let mut i = 0;
    while i < bytes.len() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            if bytes[i] != b'-' {
                return None;
            }
            i += 1;
            continue;
        }
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(*bytes.get(i + 1)?)?;
        out[out_i] = (hi << 4) | lo;
        out_i += 1;
        i += 2;
    }
    (out_i == 16).then_some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// A card's identity: the pair `(note, ordinal)`, where the ordinal is the **slot** the kind
/// definition declares (ADR-0002 §6, ADR-0017 §1) — never an index into the `cards` list.
///
/// A card is **derived, never minted** (ADR-0002 §6): two offline devices running the same rule
/// over the same content must reach the same `CardRef` without communicating, which is why identity
/// is a function of content rather than a stored id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardRef {
    pub note: NoteId,
    pub ordinal: u16,
}

impl CardRef {
    pub fn new(note: NoteId, ordinal: u16) -> Self {
        CardRef { note, ordinal }
    }

    /// The canonical 18-byte encoding (ADR-0002 §6): the note UUID's sixteen bytes in RFC 9562
    /// order, followed by the ordinal as a big-endian `u16`. No separators, no text form.
    ///
    /// Load-bearing beyond this context: `scheduling` seeds its interval fuzz from these bytes
    /// (ADR-0001 §7), so two devices compute the same due date. Any wire framing (ADR-0004 §11) must
    /// be a bijection with this.
    pub fn encode(&self) -> [u8; 18] {
        let mut out = [0u8; 18];
        out[..16].copy_from_slice(&self.note.0);
        out[16..].copy_from_slice(&self.ordinal.to_be_bytes());
        out
    }

    /// The inverse of [`CardRef::encode`]: rebuild a `CardRef` from its canonical 18 bytes, or `None`
    /// for any other length. The store keys suspension by this encoding (ADR-0010 §5), and reading the
    /// suspended set back needs to decode the blob it stored; anything not exactly eighteen bytes is
    /// not one of ours and is refused rather than guessed.
    pub fn decode(bytes: &[u8]) -> Option<CardRef> {
        let bytes: &[u8; 18] = bytes.try_into().ok()?;
        let mut note = [0u8; 16];
        note.copy_from_slice(&bytes[..16]);
        let ordinal = u16::from_be_bytes([bytes[16], bytes[17]]);
        Some(CardRef::new(NoteId(note), ordinal))
    }
}

/// The high bit partitions cloze blanks (`0x8000 | n`) from fixed-arity slots (`0x0000–0x7FFF`),
/// per ADR-0017 §3. Every shipped fixed-arity slot stays below it (the uniqueness test enforces
/// this), so the two numbering schemes cannot collide even across a note that changes kind.
pub const CLOZE_SLOT_BIT: u16 = 0x8000;

/// The slot a `cloze` blank numbered `n` occupies: `0x8000 | n` (ADR-0017 §3). The high bit keeps
/// authored, unbounded blank numbers ([`CLOZE`]) disjoint from the fixed-arity registry without the
/// registry ever having to see them — the one bit *is* the check. Its inverse is [`cloze_blank`].
pub const fn cloze_slot(blank: u16) -> u16 {
    CLOZE_SLOT_BIT | blank
}

/// The blank number a `cloze` slot names: `slot & 0x7FFF` (ADR-0017 §3). This mask is a **name,
/// never a sort key** (ADR-0018 §1): ordering cards by it asserts an adjacency between the two
/// namespaces the partition exists precisely to deny.
pub const fn cloze_blank(slot: u16) -> u16 {
    slot & !CLOZE_SLOT_BIT
}

/// The highest blank number a note may hold, `0x7FFF` (ADR-0017 §3): the cloze partition reserves
/// one bit, so an authored blank must stay in `1..=0x7FFF` for [`cloze_slot`] to be a bijection.
pub const MAX_BLANK: u16 = 0x7FFF;

/// The placeholder the masked blank leaves on a cloze card's prompt — what stands where the hidden
/// text is (ADR-0002 §5). What it *looks* like is the visual design pass's; that it is a gap and not
/// the answer is the behaviour.
pub const CLOZE_GAP: &str = "[…]";

/// One piece of a parsed cloze `Text`: literal run, or a closed `{{n::hidden}}` blank.
enum ClozeToken<'a> {
    Text(&'a str),
    Blank { number: u16, hidden: &'a str },
}

/// Split a cloze note's `Text` into literal runs and closed blanks, left to right (ADR-0002 §5). A
/// **half-typed `{{1::` stays literal** (ADR-0012 §3): an unclosed region is not a blank, because
/// live preview sees every keystroke and an inferred number is an invented identity. A `{{` that is
/// not a well-formed `{{<digits>::…}}` open — no digits, no `::`, or a number outside `1..=MAX_BLANK`
/// — is passed over as ordinary text.
fn cloze_tokens(text: &str) -> Vec<ClozeToken<'_>> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut literal_start = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{'
            && bytes[i + 1] == b'{'
            && let Some((number, hidden, end)) = parse_blank(text, i)
        {
            if literal_start < i {
                tokens.push(ClozeToken::Text(&text[literal_start..i]));
            }
            tokens.push(ClozeToken::Blank { number, hidden });
            i = end;
            literal_start = end;
            continue;
        }
        i += 1;
    }
    if literal_start < bytes.len() {
        tokens.push(ClozeToken::Text(&text[literal_start..]));
    }
    tokens
}

/// Parse one `{{<digits>::hidden}}` blank starting at `at` (where `text[at..]` begins `{{`), returning
/// its `(number, hidden text, byte index past the closing }})`, or `None` when it is not a
/// well-formed, in-range, **closed** blank — in which case the caller treats the `{{` as literal.
fn parse_blank(text: &str, at: usize) -> Option<(u16, &str, usize)> {
    let bytes = text.as_bytes();
    let num_start = at + 2;
    let mut j = num_start;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == num_start || j + 1 >= bytes.len() || bytes[j] != b':' || bytes[j + 1] != b':' {
        return None; // no digits, or no `::` separator
    }
    let hidden_start = j + 2;
    let mut k = hidden_start;
    let close = loop {
        if k + 1 >= bytes.len() {
            return None; // unclosed — a half-typed blank stays literal (ADR-0012 §3)
        }
        if bytes[k] == b'}' && bytes[k + 1] == b'}' {
            break k;
        }
        k += 1;
    };
    let number: u32 = text[num_start..j].parse().ok()?;
    if number == 0 || number > u32::from(MAX_BLANK) {
        return None; // 0 and over-cap numbers are not authorable blanks (ADR-0002 §5, ADR-0017 §3)
    }
    Some((number as u16, &text[hidden_start..close], close + 2))
}

/// The distinct blank numbers a `cloze` note's `Text` currently holds, ascending (ADR-0002 §5). A
/// number repeated in the text is **one** blank that hides in several places, so it appears once. A
/// half-typed `{{1::` contributes nothing ([`cloze_tokens`]). This is the current-content half of
/// dormancy: a blank in the log but not in this list is a **dormant** card (replay `CONTEXT.md`).
pub fn cloze_blanks(text: &str) -> Vec<u16> {
    let mut nums: Vec<u16> = cloze_tokens(text)
        .into_iter()
        .filter_map(|t| match t {
            ClozeToken::Blank { number, .. } => Some(number),
            ClozeToken::Text(_) => None,
        })
        .collect();
    nums.sort_unstable();
    nums.dedup();
    nums
}

/// The `CardRef`s a `cloze` note generates from its `Text` — one per distinct blank, at
/// [`cloze_slot`]`(n)` (ADR-0002 §5, ADR-0017 §3). This is the cloze arm of "current content" replay
/// projects onto, the counterpart to [`KindDefinition::generated_cards`] for the fixed-arity kinds
/// whose `cards` list this build declares; cloze's cards are content-derived, so its list is empty
/// and its slots are computed here instead.
pub fn cloze_cards(note: NoteId, text: &str) -> Vec<CardRef> {
    cloze_blanks(text)
        .into_iter()
        .map(|n| CardRef::new(note, cloze_slot(n)))
        .collect()
}

/// The number a new blank takes: **one above the highest currently in `text`, never the lowest free
/// one** (ADR-0012 §3). Filling a gap would hand the new blank a deleted card's identity and reattach
/// its reviews to different content — the exact damage auto-renumbering does, one edit later — so
/// gaps are left as gaps. The first blank is 1 (ADR-0002 §5).
///
/// This reads only the current text; the editor widens "ever used" to include a note's **dormant**
/// blanks — deleted blanks still carrying history — which are not in the text but must not be reused
/// either (see `leitner-app::cards`).
pub fn next_blank_number(text: &str) -> u16 {
    cloze_blanks(text).into_iter().max().map_or(1, |n| n + 1)
}

/// Render one cloze card — the blank numbered `blank` hidden on the prompt and revealed on the answer,
/// every *other* blank shown revealed on both sides (ADR-0002 §5). The masked blank leaves
/// [`CLOZE_GAP`]; a blank repeated in the text is hidden in every place it occurs. Literal text is
/// carried through untouched. Returns `(prompt, answer)`; both are untrusted content and must reach a
/// screen through the `bidi` helper.
pub fn render_cloze(text: &str, blank: u16) -> (String, String) {
    let mut prompt = String::new();
    let mut answer = String::new();
    for token in cloze_tokens(text) {
        match token {
            ClozeToken::Text(t) => {
                prompt.push_str(t);
                answer.push_str(t);
            }
            ClozeToken::Blank { number, hidden } => {
                answer.push_str(hidden);
                if number == blank {
                    prompt.push_str(CLOZE_GAP);
                } else {
                    prompt.push_str(hidden);
                }
            }
        }
    }
    (prompt, answer)
}

/// A field's role on a note (ADR-0002 §3): `Asked` fields may be a card's prompt or answer;
/// `ShownWith` fields render beside the anchor field named, on whichever side it lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRole {
    Asked,
    ShownWith(&'static str),
}

/// One field on a kind (ADR-0002 §3, §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDef {
    pub name: &'static str,
    pub role: FieldRole,
}

/// One card a kind generates (ADR-0002 §4, ADR-0017 §1): the **slot** it occupies plus the fields
/// forming its prompt and answer. The slot — not this entry's position in the `cards` list — is the
/// card's ordinal, and **list order carries nothing** (ADR-0017 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardTemplate {
    pub slot: u16,
    pub prompt: &'static [&'static str],
    pub answer: &'static [&'static str],
}

/// The read-only data describing a kind's fields and the cards it generates (ADR-0002 §4). Shipped
/// with the application; never authored by a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindDefinition {
    pub id: &'static str,
    pub fields: &'static [FieldDef],
    pub cards: &'static [CardTemplate],
}

impl KindDefinition {
    /// The `CardRef`s a note of this kind generates. For a fixed-arity kind this is one card per
    /// `cards` entry, at the declared slot. This is the "current content" replay projects onto
    /// (ADR-0002 §7): a row whose `CardRef` is not in this set for its note is retained and simply
    /// not projected.
    pub fn generated_cards(&self, note: NoteId) -> Vec<CardRef> {
        self.cards
            .iter()
            .map(|c| CardRef::new(note, c.slot))
            .collect()
    }

    /// The fields rendered on each side of a card — `(prompt, answer)` — with every `shown-with`
    /// field placed beside the anchor it follows (ADR-0002 §3). A `shown-with(F)` field renders on
    /// **whichever side `F` lands on**, which is what carries a pronunciation to the answer when the
    /// term is being *produced* and to the prompt when it is being *recognised*, without either
    /// direction being special-cased.
    pub fn render_sides(&self, card: &CardTemplate) -> (Vec<&'static str>, Vec<&'static str>) {
        (self.render_side(card.prompt), self.render_side(card.answer))
    }

    /// One side's fields: each anchor, immediately followed by the `shown-with` fields attached to
    /// it, in field-definition order. Attachment does not chain (ADR-0002 §3), so a single pass
    /// suffices.
    fn render_side(&self, anchors: &[&'static str]) -> Vec<&'static str> {
        let mut out = Vec::new();
        for &anchor in anchors {
            out.push(anchor);
            for field in self.fields {
                if field.role == FieldRole::ShownWith(anchor) {
                    out.push(field.name);
                }
            }
        }
        out
    }
}

/// `basic`: two asked fields, one card at slot 0 for Front→Back (ADR-0002 §2, ADR-0017 §1).
pub const BASIC: KindDefinition = KindDefinition {
    id: "basic",
    fields: &[
        FieldDef {
            name: "Front",
            role: FieldRole::Asked,
        },
        FieldDef {
            name: "Back",
            role: FieldRole::Asked,
        },
    ],
    cards: &[CardTemplate {
        slot: 0,
        prompt: &["Front"],
        answer: &["Back"],
    }],
};

/// `basic-reverse`: the same two asked fields as `basic`, and **slot 0 is deliberately the same
/// Front→Back card** (ADR-0017 §2), so a note gaining its reverse direction reattaches its history
/// rather than orphaning it. Slot 1 adds Back→Front.
pub const BASIC_REVERSE: KindDefinition = KindDefinition {
    id: "basic-reverse",
    fields: &[
        FieldDef {
            name: "Front",
            role: FieldRole::Asked,
        },
        FieldDef {
            name: "Back",
            role: FieldRole::Asked,
        },
    ],
    cards: &[
        CardTemplate {
            slot: 0,
            prompt: &["Front"],
            answer: &["Back"],
        },
        CardTemplate {
            slot: 1,
            prompt: &["Back"],
            answer: &["Front"],
        },
    ],
};

/// `vocab`: two asked fields (Term, Meaning) and two `shown-with` fields that follow Term wherever
/// it lands (ADR-0002 §3, §4). Slots 2 and 3 are the two directions; the pronunciation and example
/// are never asked, so they render beside the term on whichever side the term is on.
pub const VOCAB: KindDefinition = KindDefinition {
    id: "vocab",
    fields: &[
        FieldDef {
            name: "Term",
            role: FieldRole::Asked,
        },
        FieldDef {
            name: "Meaning",
            role: FieldRole::Asked,
        },
        FieldDef {
            name: "Pronunciation",
            role: FieldRole::ShownWith("Term"),
        },
        FieldDef {
            name: "Example",
            role: FieldRole::ShownWith("Term"),
        },
    ],
    cards: &[
        CardTemplate {
            slot: 2,
            prompt: &["Term"],
            answer: &["Meaning"],
        },
        CardTemplate {
            slot: 3,
            prompt: &["Meaning"],
            answer: &["Term"],
        },
    ],
};

/// `cloze`: one asked `Text` field. Its cards are **not fixed** — one per numbered blank in the
/// note's text, at slot [`cloze_slot`]`(n)` (ADR-0002 §5, ADR-0017 §3) — so its `cards` list is
/// empty and its slots are computed rather than declared. Generating them needs the note's content,
/// which the fixed-arity [`KindDefinition::generated_cards`] path never sees; [`cloze_cards`] is that
/// content-driven projection, with [`render_cloze`] and [`next_blank_number`] the rest of the cloze
/// vocabulary the card pane draws on.
pub const CLOZE: KindDefinition = KindDefinition {
    id: "cloze",
    fields: &[FieldDef {
        name: "Text",
        role: FieldRole::Asked,
    }],
    cards: &[],
};

/// Every kind definition this build ships (ADR-0002 §2). The set is **closed and code-defined** — a
/// user never authors a kind. The two tests below guard the slot rule over exactly this table
/// (ADR-0017 §4); a fifth kind is added here, not woven in elsewhere.
pub const SHIPPED_KINDS: &[&KindDefinition] = &[&BASIC, &BASIC_REVERSE, &VOCAB, &CLOZE];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardref_encodes_to_eighteen_bytes_uuid_then_ordinal() {
        // ADR-0002 §6: sixteen UUID bytes in RFC 9562 order, then the ordinal big-endian.
        let note = NoteId([
            0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44,
            0x00, 0x00,
        ]);
        let card = CardRef::new(note, 0x0102);
        let encoded = card.encode();
        assert_eq!(&encoded[..16], &note.0);
        assert_eq!(&encoded[16..], &[0x01, 0x02]);
    }

    #[test]
    fn cardref_decode_is_the_inverse_of_encode_and_refuses_bad_lengths() {
        // ADR-0010 §5: the store keys suspension by the 18-byte encoding and decodes it back to
        // enumerate the suspended set. Round-trips exactly; a blob of any other length is not ours.
        for ordinal in [0u16, 1, 0x0102, 0x7FFF, cloze_slot(3)] {
            let card = CardRef::new(NoteId([0xab; 16]), ordinal);
            assert_eq!(CardRef::decode(&card.encode()), Some(card));
        }
        assert_eq!(
            CardRef::decode(&[0u8; 16]),
            None,
            "sixteen bytes is a note id, not a card"
        );
        assert_eq!(CardRef::decode(&[0u8; 19]), None);
        assert_eq!(CardRef::decode(&[]), None);
    }

    #[test]
    fn cloze_blank_maps_to_a_slot_above_the_high_bit_and_back() {
        // ADR-0017 §3: a raw log row for cloze blank 1 reads ordinal 32769, and the blank number is
        // recovered by masking. The two functions are inverse over the whole blank range.
        assert_eq!(cloze_slot(1), 32769);
        assert_eq!(cloze_blank(cloze_slot(1)), 1);
        // Every fixed-arity slot is disjoint from every cloze slot by the one bit.
        for blank in [1u16, 2, 7, 0x7FFF] {
            assert_eq!(cloze_slot(blank) & CLOZE_SLOT_BIT, CLOZE_SLOT_BIT);
            assert_eq!(cloze_blank(cloze_slot(blank)), blank);
        }
    }

    #[test]
    fn note_uuid_text_round_trips_through_the_canonical_form() {
        let text = "550e8400-e29b-41d4-a716-446655440000";
        let id = NoteId::parse_canonical(text).expect("valid canonical uuid");
        assert_eq!(id.to_canonical(), text);
        // Uppercase parses to the same bytes (RFC 9562 is case-insensitive on input).
        assert_eq!(
            NoteId::parse_canonical(&text.to_uppercase()),
            Some(id),
            "uppercase must parse to the same id"
        );
    }

    #[test]
    fn malformed_uuid_text_is_rejected_rather_than_panicking() {
        assert_eq!(NoteId::parse_canonical(""), None);
        assert_eq!(NoteId::parse_canonical("not-a-uuid"), None);
        // Right length, wrong hyphen placement.
        assert_eq!(
            NoteId::parse_canonical("550e8400e29b-41d4-a716-4466554400001"),
            None
        );
        // A non-hex digit where hex is required.
        assert_eq!(
            NoteId::parse_canonical("g50e8400-e29b-41d4-a716-446655440000"),
            None
        );
    }

    #[test]
    fn deck_id_round_trips_through_the_canonical_form_like_a_note_id() {
        // ADR-0005 §4: a deck id is a UUIDv4 preserved through export and import, carried as canonical
        // text (a note's `deck` reference and the deck filter both compare that text).
        let text = "550e8400-e29b-41d4-a716-446655440000";
        let id = DeckId::parse_canonical(text).expect("valid canonical uuid");
        assert_eq!(id.to_canonical(), text);
        // Case-insensitive on input, lowercase on output — the two ids share one canonical form.
        assert_eq!(DeckId::parse_canonical(&text.to_uppercase()), Some(id));
        assert_eq!(DeckId::parse_canonical("not-a-deck"), None);
        // A deck id and a note id with the same bytes share the same canonical text — the two types
        // differ in meaning, not in encoding.
        assert_eq!(id.to_canonical(), NoteId(id.0).to_canonical());
    }

    #[test]
    fn basic_declares_slot_zero_for_front_to_back() {
        // ADR-0002 §2, ADR-0017 §1: the one shipped card is slot 0, Front→Back.
        assert_eq!(BASIC.id, "basic");
        assert_eq!(BASIC.cards.len(), 1);
        let card = BASIC.cards[0];
        assert_eq!(card.slot, 0);
        assert_eq!(card.prompt, &["Front"]);
        assert_eq!(card.answer, &["Back"]);
    }

    #[test]
    fn basic_generates_one_card_at_slot_zero() {
        let note = NoteId([7; 16]);
        assert_eq!(BASIC.generated_cards(note), vec![CardRef::new(note, 0)]);
    }

    /// Read a card by its **slot**, never by its position in the `cards` list — the whole point of
    /// ADR-0017 §1 is that list order carries nothing.
    fn card_at(kind: &KindDefinition, slot: u16) -> &CardTemplate {
        kind.cards
            .iter()
            .find(|c| c.slot == slot)
            .unwrap_or_else(|| panic!("kind {} declares no slot {slot}", kind.id))
    }

    #[test]
    fn the_four_shipped_kinds_are_exactly_these() {
        // ADR-0002 §2: the set is closed and code-defined. This pins the identifiers so a rename or
        // a dropped kind is a red build, not a silent behaviour change on stored notes.
        let ids: Vec<&str> = SHIPPED_KINDS.iter().map(|k| k.id).collect();
        assert_eq!(ids, vec!["basic", "basic-reverse", "vocab", "cloze"]);
    }

    #[test]
    fn basic_and_basic_reverse_share_slot_zero_for_front_to_back() {
        // ADR-0017 §2: the same card, deliberately, so gaining the reverse direction reattaches
        // history rather than orphaning it. Same slot AND same question is what makes that safe.
        let basic0 = card_at(&BASIC, 0);
        let reverse0 = card_at(&BASIC_REVERSE, 0);
        assert_eq!(
            (basic0.prompt, basic0.answer),
            (reverse0.prompt, reverse0.answer)
        );
        assert_eq!(basic0.prompt, &["Front"]);
        assert_eq!(basic0.answer, &["Back"]);
        // The reverse direction is the *new* card, at its own slot.
        let reverse1 = card_at(&BASIC_REVERSE, 1);
        assert_eq!(reverse1.prompt, &["Back"]);
        assert_eq!(reverse1.answer, &["Front"]);
    }

    #[test]
    fn basic_reverse_generates_both_directions_as_distinct_cards() {
        let note = NoteId([9; 16]);
        assert_eq!(
            BASIC_REVERSE.generated_cards(note),
            vec![CardRef::new(note, 0), CardRef::new(note, 1)]
        );
    }

    #[test]
    fn shown_with_follows_its_anchor_to_whichever_side_it_lands() {
        // ADR-0002 §3: Pronunciation and Example are shown-with(Term). Recognising the term
        // (Term→Meaning) puts them on the prompt; producing it (Meaning→Term) puts them on the
        // answer — the same fields, no direction special-cased.
        let (prompt, answer) = VOCAB.render_sides(card_at(&VOCAB, 2));
        assert_eq!(prompt, vec!["Term", "Pronunciation", "Example"]);
        assert_eq!(answer, vec!["Meaning"]);

        let (prompt, answer) = VOCAB.render_sides(card_at(&VOCAB, 3));
        assert_eq!(prompt, vec!["Meaning"]);
        assert_eq!(answer, vec!["Term", "Pronunciation", "Example"]);
    }

    #[test]
    fn a_kind_without_shown_with_renders_only_its_asked_fields() {
        let (prompt, answer) = BASIC.render_sides(card_at(&BASIC, 0));
        assert_eq!(prompt, vec!["Front"]);
        assert_eq!(answer, vec!["Back"]);
    }

    #[test]
    fn cloze_blanks_are_the_distinct_closed_numbers_in_reading_order() {
        // ADR-0002 §5: each distinct number is one card; a number repeated in the text hides in two
        // places but is still one blank. The result is ascending and deduplicated.
        assert_eq!(cloze_blanks("{{1::le}} chat {{2::mange}}"), vec![1, 2]);
        assert_eq!(
            cloze_blanks("{{2::a}} then {{2::b}} then {{1::c}}"),
            vec![1, 2],
            "a repeated number is one blank"
        );
        assert!(cloze_blanks("no blanks here").is_empty());
    }

    #[test]
    fn a_half_typed_blank_stays_literal() {
        // ADR-0012 §3: an unclosed `{{1::` is not a blank — an inferred number is an invented
        // identity, so live preview must see nothing until the region closes.
        assert!(cloze_blanks("the {{1:: cat sat").is_empty());
        assert!(
            cloze_blanks("{{1::").is_empty(),
            "the exact keystroke the ADR names"
        );
        // A closed blank followed by a half-typed one keeps only the closed one.
        assert_eq!(cloze_blanks("{{1::a}} and {{2::"), vec![1]);
        // Not-a-blank opens are passed over as text: no digits, or no `::`.
        assert!(cloze_blanks("{{::x}} {{cat}}").is_empty());
    }

    #[test]
    fn an_out_of_range_blank_number_is_not_a_blank() {
        // ADR-0017 §3: a blank must fit `1..=0x7FFF` for the slot map to be a bijection; 0 and any
        // over-cap number are treated as literal rather than corrupting the partition.
        assert!(cloze_blanks("{{0::x}}").is_empty());
        assert_eq!(cloze_blanks("{{32767::x}}"), vec![MAX_BLANK]);
        assert!(
            cloze_blanks("{{32768::x}}").is_empty(),
            "one past the cap is not authorable"
        );
    }

    #[test]
    fn cloze_generates_one_card_per_distinct_blank_above_the_high_bit() {
        let note = NoteId([5; 16]);
        assert_eq!(
            cloze_cards(note, "{{1::a}} {{3::b}}"),
            vec![
                CardRef::new(note, cloze_slot(1)),
                CardRef::new(note, cloze_slot(3)),
            ]
        );
        assert!(cloze_cards(note, "nothing").is_empty());
    }

    #[test]
    fn a_new_blank_is_one_above_the_highest_never_the_lowest_free() {
        // ADR-0012 §3: gaps are shown as normal and never filled. Blanks 1, 2, 4 present → the next
        // is 5, not the free 3, so a new blank can never reclaim a deleted card's identity.
        assert_eq!(next_blank_number(""), 1, "the first blank is 1");
        assert_eq!(next_blank_number("{{1::a}}"), 2);
        assert_eq!(
            next_blank_number("{{1::a}} {{2::b}} {{4::d}}"),
            5,
            "the gap at 3 is left as a gap"
        );
    }

    #[test]
    fn render_cloze_masks_the_card_blank_and_reveals_the_rest() {
        // ADR-0002 §5: the card's blank is hidden on the prompt and shown on the answer; other blanks
        // stay revealed on both sides. Literal text is carried through untouched.
        let text = "{{1::Le}} chat {{2::mange}} la souris";
        let (prompt, answer) = render_cloze(text, 1);
        assert_eq!(prompt, "[…] chat mange la souris");
        assert_eq!(answer, "Le chat mange la souris");

        let (prompt, answer) = render_cloze(text, 2);
        assert_eq!(prompt, "Le chat […] la souris");
        assert_eq!(answer, "Le chat mange la souris");

        // A number repeated in the text is masked in every place it occurs.
        let (prompt, _) = render_cloze("{{1::a}} x {{1::a}}", 1);
        assert_eq!(prompt, "[…] x […]");
    }

    #[test]
    fn cloze_declares_no_fixed_cards_and_numbers_blanks_above_the_bit() {
        // Cloze cards come from content, not a fixed list (ADR-0002 §5); the fixed-arity generator
        // therefore yields nothing, and the numbering rule lives in `cloze_slot`.
        assert!(CLOZE.cards.is_empty());
        assert_eq!(CLOZE.generated_cards(NoteId([3; 16])), vec![]);
        assert_eq!(cloze_slot(1), CLOZE_SLOT_BIT | 1);
    }

    // ADR-0017 §4: "the most destructive edit in the codebase becomes a test." The two tests below
    // are the only reason the slot rule is enforceable, and both need no database, no window and no
    // handset.

    #[test]
    fn a_slot_means_one_question_across_every_shipped_definition() {
        // Slots are drawn from ONE namespace shared by every kind (ADR-0017 §1). Two things follow,
        // and both are checked here because both make the golden list and ADR-0018 §3's cross-kind
        // dormant-name lookup well-formed:
        //   * within a single definition a repeated slot is always a bug (per-definition
        //     uniqueness), and
        //   * two kinds may declare the same slot ONLY when they mean the same card — so a slot
        //     names one question collection-wide, which is exactly what lets `basic` and
        //     `basic-reverse` share slot 0 while catching a slot reused for a different question.
        // Fixed-arity slots must also stay below the cloze high bit (ADR-0017 §3), or the two
        // numbering schemes could collide.
        let mut meaning: std::collections::HashMap<u16, (&[&str], &[&str])> =
            std::collections::HashMap::new();
        for kind in SHIPPED_KINDS {
            let mut seen = std::collections::HashSet::new();
            for card in kind.cards {
                assert!(
                    seen.insert(card.slot),
                    "kind {} declares slot {} twice",
                    kind.id,
                    card.slot
                );
                assert_eq!(
                    card.slot & CLOZE_SLOT_BIT,
                    0,
                    "kind {} declares fixed-arity slot {} with the cloze high bit set",
                    kind.id,
                    card.slot
                );
                if let Some(prev) = meaning.insert(card.slot, (card.prompt, card.answer)) {
                    assert_eq!(
                        prev,
                        (card.prompt, card.answer),
                        "slot {} means two different questions across kinds",
                        card.slot
                    );
                }
            }
        }
    }

    #[test]
    fn slot_meanings_match_the_checked_in_golden_list() {
        // ADR-0017 §4: immutability against a checked-in golden `slot → (prompt, answer)` list.
        // Changing a slot number on an existing entry, or reusing one for a different question,
        // silently retypes accumulated review history onto the wrong card and cannot be repaired
        // from the log. Editing a shipped definition without updating this list is a red build.
        // Cloze contributes no rows: its cards are content-derived, not declared (ADR-0002 §5).
        let golden: &[(&str, u16, &[&str], &[&str])] = &[
            ("basic", 0, &["Front"], &["Back"]),
            ("basic-reverse", 0, &["Front"], &["Back"]),
            ("basic-reverse", 1, &["Back"], &["Front"]),
            ("vocab", 2, &["Term"], &["Meaning"]),
            ("vocab", 3, &["Meaning"], &["Term"]),
        ];

        let mut actual: Vec<(&str, u16, &[&str], &[&str])> = Vec::new();
        for kind in SHIPPED_KINDS {
            for card in kind.cards {
                actual.push((kind.id, card.slot, card.prompt, card.answer));
            }
        }
        // List order carries nothing (ADR-0017 §4), so compare on a stable (kind, slot) sort — a
        // reordered `cards` list is harmless and must not break this test.
        actual.sort_by_key(|(id, slot, ..)| (*id, *slot));
        let mut golden: Vec<(&str, u16, &[&str], &[&str])> = golden.to_vec();
        golden.sort_by_key(|(id, slot, ..)| (*id, *slot));
        assert_eq!(
            actual, golden,
            "a shipped slot's meaning changed — update the golden list only if the change is truly \
             additive, never to relabel an existing slot"
        );
    }
}
