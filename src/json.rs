//! Minimal JSON emission for machine-readable findings (§GOAL-002-tiny-footprint).
//! Only the value shapes `fissile` emits are modeled, so no `serde_json` (or any
//! reflective JSON dependency) is pulled into the binary.

/// A JSON value `fissile` can render. Object key order is preserved so output is
/// stable and diff-friendly.
pub enum Json {
    Null,
    UInt(u64),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    pub fn str(value: impl Into<String>) -> Json {
        Json::Str(value.into())
    }

    /// Render to a compact (newline-free) JSON string.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::UInt(value) => out.push_str(&value.to_string()),
            Json::Str(value) => write_string(value, out),
            Json::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(fields) => {
                out.push('{');
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_string(value: &str, out: &mut String) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_objects_and_arrays_in_order() {
        let value = Json::Array(vec![Json::Object(vec![
            ("path", Json::str("src/lib.rs")),
            ("actual", Json::UInt(612)),
            ("note", Json::Null),
        ])]);
        assert_eq!(
            value.render(),
            r#"[{"path":"src/lib.rs","actual":612,"note":null}]"#
        );
    }

    #[test]
    fn escapes_control_and_quote_characters() {
        let value = Json::str("a\"b\\c\n\t");
        assert_eq!(value.render(), r#""a\"b\\c\n\t""#);
    }
}
