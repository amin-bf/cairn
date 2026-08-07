//! A minimal JSON reader for the interchange form (ADR-0004 §11).
//!
//! Hand-written on purpose. `serde` is in the lockfile transitively through `fsrs` but is **not
//! ours to reach for** (ADR-0027 §3), and the guarantee ADR-0004 §11 actually wants is stronger than
//! any derive gives: a row is relayed **byte for byte and never re-encoded**. So this reads the form
//! and there is deliberately **no writer here** — nothing in `cairn-core` re-encodes a row.
//!
//! It parses the whole grammar (so an unknown field of any shape is skipped rather than breaking a
//! known row) but exposes only the scalar and number-array accessors the row builders need.

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Parse a complete JSON document, or `None` if it is malformed or has trailing content.
    pub fn parse(input: &str) -> Option<Json> {
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        parser.skip_ws();
        let value = parser.value()?;
        parser.skip_ws();
        if parser.pos == parser.bytes.len() {
            Some(value)
        } else {
            None
        }
    }

    /// Look up a key in an object; `None` for a non-object or a missing key.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Num(n) if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 => {
                Some(*n as i64)
            }
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Num(n) if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 => {
                Some(*n as u64)
            }
            _ => None,
        }
    }

    /// A JSON array of numbers as `f32`s. Weights are written with full round-trip precision
    /// (ADR-0004 §11), so parsing as `f64` and narrowing recovers the exact `f32`.
    pub fn as_f32_array(&self) -> Option<Vec<f32>> {
        match self {
            Json::Arr(items) => items.iter().map(|v| v.as_f64().map(|n| n as f32)).collect(),
            _ => None,
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Some(())
        } else {
            None
        }
    }

    fn value(&mut self) -> Option<Json> {
        self.skip_ws();
        match self.peek()? {
            b'"' => self.string().map(Json::Str),
            b'{' => self.object(),
            b'[' => self.array(),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn literal(&mut self, text: &str, value: Json) -> Option<Json> {
        let end = self.pos + text.len();
        if self.bytes.get(self.pos..end) == Some(text.as_bytes()) {
            self.pos = end;
            Some(value)
        } else {
            None
        }
    }

    fn object(&mut self) -> Option<Json> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Some(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek()? {
                b',' => {
                    self.pos += 1;
                }
                b'}' => {
                    self.pos += 1;
                    return Some(Json::Obj(entries));
                }
                _ => return None,
            }
        }
    }

    fn array(&mut self) -> Option<Json> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Some(Json::Arr(items));
        }
        loop {
            let value = self.value()?;
            items.push(value);
            self.skip_ws();
            match self.peek()? {
                b',' => {
                    self.pos += 1;
                }
                b']' => {
                    self.pos += 1;
                    return Some(Json::Arr(items));
                }
                _ => return None,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = self.peek()?;
            self.pos += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escape = self.peek()?;
                    self.pos += 1;
                    match escape {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let code = self.hex4()?;
                            // The interchange form is ASCII in practice; handle the BMP correctly
                            // and leave surrogate pairs to a future need rather than mis-decoding.
                            let ch = char::from_u32(u32::from(code))?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return None,
                    }
                }
                // A control character unescaped inside a string is invalid JSON.
                0x00..=0x1f => return None,
                // Any other byte (including UTF-8 continuation bytes) is copied verbatim.
                other => out.push(other),
            }
        }
    }

    fn hex4(&mut self) -> Option<u16> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let digit = self.peek()?;
            self.pos += 1;
            let nibble = match digit {
                b'0'..=b'9' => digit - b'0',
                b'a'..=b'f' => digit - b'a' + 10,
                b'A'..=b'F' => digit - b'A' + 10,
                _ => return None,
            };
            value = value << 4 | u16::from(nibble);
        }
        Some(value)
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => self.pos += 1,
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        let parsed: f64 = text.parse().ok()?;
        if parsed.is_finite() {
            Some(Json::Num(parsed))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_flat_object() {
        let json = Json::parse(r#"{"a":1,"b":"two","c":true,"d":null}"#).unwrap();
        assert_eq!(json.get("a").and_then(Json::as_u64), Some(1));
        assert_eq!(json.get("b").and_then(Json::as_str), Some("two"));
        assert_eq!(json.get("c"), Some(&Json::Bool(true)));
        assert_eq!(json.get("d"), Some(&Json::Null));
    }

    #[test]
    fn parses_nested_and_arrays() {
        let json = Json::parse(r#"{"v":[1,2.5,-3],"obj":{"x":[true,null]}}"#).unwrap();
        assert_eq!(
            json.get("v").and_then(Json::as_f32_array),
            Some(vec![1.0, 2.5, -3.0])
        );
        assert!(json.get("obj").and_then(|o| o.get("x")).is_some());
    }

    #[test]
    fn negative_and_fractional_numbers_classify_correctly() {
        let json = Json::parse(r#"{"i":-5,"f":1.5,"big":20514}"#).unwrap();
        assert_eq!(json.get("i").and_then(Json::as_i64), Some(-5));
        assert_eq!(
            json.get("i").and_then(Json::as_u64),
            None,
            "negative is not u64"
        );
        assert_eq!(
            json.get("f").and_then(Json::as_i64),
            None,
            "fractional is not i64"
        );
        assert_eq!(json.get("big").and_then(Json::as_i64), Some(20514));
    }

    #[test]
    fn handles_string_escapes() {
        let json = Json::parse(r#"{"s":"a\"b\\c\/d\nA"}"#).unwrap();
        assert_eq!(json.get("s").and_then(Json::as_str), Some("a\"b\\c/d\nA"));
    }

    #[test]
    fn preserves_utf8_content() {
        let json = Json::parse(r#"{"s":"héllo"}"#).unwrap();
        assert_eq!(json.get("s").and_then(Json::as_str), Some("héllo"));
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(Json::parse(""), None);
        assert_eq!(Json::parse("{"), None);
        assert_eq!(Json::parse(r#"{"a":}"#), None);
        assert_eq!(Json::parse(r#"{"a":1,}"#), None);
        assert_eq!(Json::parse(r#"{"a":1} trailing"#), None);
        assert_eq!(Json::parse("[1,2"), None);
        assert_eq!(Json::parse(r#""unterminated"#), None);
    }

    #[test]
    fn f32_array_recovers_exact_weights() {
        // A default-ish weight written with full precision must narrow back to the exact f32.
        let json = Json::parse(r#"{"v":[0.212,8.2956,0.1542]}"#).unwrap();
        let v = json.get("v").and_then(Json::as_f32_array).unwrap();
        assert_eq!(v, vec![0.212f32, 8.2956f32, 0.1542f32]);
    }
}
