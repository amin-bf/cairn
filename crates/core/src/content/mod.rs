//! See `CONTEXT.md` beside this file for the vocabulary, the binding ADR sections, and the rules
//! that break silently.
//!
//! This is the base context: `log`, `scheduling` and `replay` all depend on it, and it depends on
//! none of them. It carries the two things the rest of the domain names — a card's identity
//! (`CardRef`, ADR-0002 §6) and the kind definitions that say which cards a note generates
//! (ADR-0002 §1, ADR-0017 §1).
//!
//! Deliberately narrow, per [#78](https://github.com/amin-bf/leitner/issues/78): **one shipped
//! kind, `basic`, declaring slot 0 for Front→Back.** The full kind set and the slot namespace are a
//! separate ticket; what lands here is `basic` plus the two tests that make a slot's immutability
//! enforceable (ADR-0017 §4).

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
        if out_i != 16 {
            return None;
        }
        Some(NoteId(out))
    }

    /// The RFC 9562 canonical text form, lowercase. The inverse of [`NoteId::parse_canonical`].
    pub fn to_canonical(&self) -> String {
        let mut s = String::with_capacity(36);
        for (i, byte) in self.0.iter().enumerate() {
            if i == 4 || i == 6 || i == 8 || i == 10 {
                s.push('-');
            }
            s.push(char::from(HEX[usize::from(byte >> 4)]));
            s.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        s
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

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
}

/// The high bit partitions cloze blanks (`0x8000 | n`) from fixed-arity slots (`0x0000–0x7FFF`),
/// per ADR-0017 §3. `basic` is fixed-arity, so nothing here reaches above the bit; the constant is
/// recorded so a later slot-namespace ticket cannot silently allocate across the partition.
pub const CLOZE_SLOT_BIT: u16 = 0x8000;

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

/// Every kind definition this build ships. The two tests below guard the slot rule over exactly
/// this table (ADR-0017 §4); a second kind is added here, not woven in elsewhere.
pub const SHIPPED_KINDS: &[&KindDefinition] = &[&BASIC];

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
    fn cloze_ordinal_is_the_blank_number_above_the_high_bit() {
        // ADR-0017 §3: a raw log row for cloze blank 1 reads ordinal 32769, and the blank number is
        // `ordinal & 0x7FFF`. `basic` never reaches here, but the encoding must carry it faithfully.
        let ordinal = CLOZE_SLOT_BIT | 1;
        assert_eq!(ordinal, 32769);
        assert_eq!(ordinal & 0x7FFF, 1);
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

    // ADR-0017 §4: "the most destructive edit in the codebase becomes a test." The two tests below
    // are the only reason the slot rule is enforceable, and both need no database, no window and no
    // handset.

    #[test]
    fn slots_are_unique_across_every_shipped_definition() {
        // Slots are drawn from one namespace shared by every kind (ADR-0017 §1). Two kinds may share
        // a slot only when they mean the same card; within a single definition a repeat is always a
        // bug, so this checks per-definition uniqueness, which is what the golden list below relies
        // on to be well-formed.
        for kind in SHIPPED_KINDS {
            let mut seen = std::collections::HashSet::new();
            for card in kind.cards {
                assert!(
                    seen.insert(card.slot),
                    "kind {} declares slot {} twice",
                    kind.id,
                    card.slot
                );
            }
        }
    }

    #[test]
    fn slot_meanings_match_the_checked_in_golden_list() {
        // ADR-0017 §4: immutability against a checked-in golden `slot → (prompt, answer)` list.
        // Changing a slot number on an existing entry, or reusing one for a different question,
        // silently retypes accumulated review history onto the wrong card and cannot be repaired
        // from the log. Editing a shipped definition without updating this list is a red build.
        let golden: &[(&str, u16, &[&str], &[&str])] = &[("basic", 0, &["Front"], &["Back"])];

        let mut actual: Vec<(&str, u16, &[&str], &[&str])> = Vec::new();
        for kind in SHIPPED_KINDS {
            for card in kind.cards {
                actual.push((kind.id, card.slot, card.prompt, card.answer));
            }
        }
        assert_eq!(
            actual, golden,
            "a shipped slot's meaning changed — update the golden list only if the change is truly \
             additive, never to relabel an existing slot"
        );
    }
}
