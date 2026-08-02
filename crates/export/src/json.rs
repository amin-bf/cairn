//! A minimal JSON *writer* — the counterpart the interchange reader in `leitner-core::log::json`
//! deliberately does not have.
//!
//! `leitner-core` never re-encodes a log row, so it ships only a reader (ADR-0004 §11). The deck
//! container is the opposite job: it *emits* JSON, and it must emit **byte-for-byte deterministic**
//! bytes (ADR-0008 §12), so keys are written in the exact order the caller passes and strings are
//! escaped by one rule. Hand-written for the same reason as the reader — `serde` is not this
//! workspace's to reach for (ADR-0027 §3), and determinism is a property the derive does not
//! promise. Numbers are `u64`/`u32` only; the container carries no floats.

/// Append a JSON string literal — `"…"` with the escapes JSON requires — to `out`.
pub fn string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            // Other C0 controls have no short escape and must be `\u00XX`.
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// One `"key":<value>` member into an object being built, prefixing a comma after the first.
///
/// `value` is an already-rendered JSON fragment. Keys are written in call order, so the caller
/// sorts to fix a canonical order — which is the whole determinism story for an object.
pub fn member(out: &mut String, first: &mut bool, key: &str, value: &str) {
    if *first {
        *first = false;
    } else {
        out.push(',');
    }
    string(out, key);
    out.push(':');
    out.push_str(value);
}

/// A bare JSON string value as a fragment, for [`member`]'s `value`.
pub fn string_value(s: &str) -> String {
    let mut out = String::new();
    string(&mut out, s);
    out
}

/// A JSON object assembled member by member. Members are written in the exact order they are added,
/// so a caller fixes a canonical order by adding keys sorted — the whole determinism story for an
/// object (ADR-0008 §12). Chain the builders and [`Object::finish`] to close the brace.
pub struct Object {
    out: String,
    first: bool,
}

impl Default for Object {
    fn default() -> Object {
        Object::new()
    }
}

impl Object {
    pub fn new() -> Object {
        Object {
            out: "{".to_owned(),
            first: true,
        }
    }

    /// Add `"key":value`, where `value` is an already-rendered JSON fragment (nested object, array,
    /// number or bool).
    pub fn raw(mut self, key: &str, value: &str) -> Object {
        member(&mut self.out, &mut self.first, key, value);
        self
    }

    /// Add `"key":"value"`, escaping `value` as a JSON string.
    pub fn string(self, key: &str, value: &str) -> Object {
        self.raw(key, &string_value(value))
    }

    pub fn finish(mut self) -> String {
        self.out.push('}');
        self.out
    }
}

/// A JSON array assembled element by element, in the order they are added — the caller arranges a
/// canonical order the same way [`Object`] does. Chain the pushers and [`Array::finish`].
pub struct Array {
    out: String,
    first: bool,
}

impl Default for Array {
    fn default() -> Array {
        Array::new()
    }
}

impl Array {
    pub fn new() -> Array {
        Array {
            out: "[".to_owned(),
            first: true,
        }
    }

    /// Push an already-rendered JSON fragment.
    pub fn raw(mut self, value: &str) -> Array {
        if self.first {
            self.first = false;
        } else {
            self.out.push(',');
        }
        self.out.push_str(value);
        self
    }

    /// Push a JSON string element, escaping `value`.
    pub fn string(self, value: &str) -> Array {
        self.raw(&string_value(value))
    }

    pub fn finish(mut self) -> String {
        self.out.push(']');
        self.out
    }
}
