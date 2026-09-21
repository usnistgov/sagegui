//! Small XML helpers shared by the mzIdentML and pepXML writers.
//!
//! Both writers build text by hand. One escaping rule covers attribute values
//! and element text, so a value can go in either place.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Wraps a string so `Display` writes it XML-escaped.
///
/// - `& < > " '` become entity references.
/// - Tab, line feed and carriage return become numeric references. That keeps
///   them intact inside an attribute value.
/// - Characters that XML 1.0 does not allow are dropped. Such a character
///   would make the whole file unreadable.
pub struct Esc<'a>(pub &'a str);

/// XML 1.0 `Char` production. Everything outside it is illegal in a document.
fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

impl fmt::Display for Esc<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Fast path: most values need no change.
        if self.0.chars().all(|c| {
            is_xml_char(c) && !matches!(c, '&' | '<' | '>' | '"' | '\'' | '\t' | '\n' | '\r')
        }) {
            return f.write_str(self.0);
        }
        for c in self.0.chars() {
            match c {
                '&' => f.write_str("&amp;")?,
                '<' => f.write_str("&lt;")?,
                '>' => f.write_str("&gt;")?,
                '"' => f.write_str("&quot;")?,
                '\'' => f.write_str("&apos;")?,
                '\t' => f.write_str("&#9;")?,
                '\n' => f.write_str("&#10;")?,
                '\r' => f.write_str("&#13;")?,
                c if is_xml_char(c) => fmt::Write::write_char(f, c)?,
                _ => {}
            }
        }
        Ok(())
    }
}

/// Escape to an owned string. Used where a value is built before it is written.
pub fn escaped(s: &str) -> String {
    Esc(s).to_string()
}

/// Format a float for XML. Rust's `Display` gives the shortest text that reads
/// back to the same value, and never uses an exponent for finite numbers.
/// Negative zero is written as `0`.
pub fn num(v: f64) -> String {
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{v}")
    }
}

/// Same as [`num`] for `f32`. Used for values Sage stores as `f32`, so the
/// text matches what a person typed (`57.0215`, not `57.02149963378906`).
pub fn num32(v: f32) -> String {
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{v}")
    }
}

/// Current UTC time as an ISO 8601 string, for example `2026-09-21T15:11:00`.
/// Both formats need a date. This avoids a date-time crate.
pub fn utc_now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso_from_unix(secs)
}

/// Unix seconds to `YYYY-MM-DDThh:mm:ss` (UTC). Civil-date algorithm from
/// Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms".
pub fn iso_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// A local path or URL as a URI. The XSD types `location` as `anyURI`, and a
/// raw Windows path or a path with a space is not one.
///
/// A value that already has a scheme (`file://`, `s3://`) keeps it. Anything
/// not allowed in a URI is percent-encoded, and an existing `%XX` is kept.
/// Backslashes become slashes.
pub fn to_uri(path: &str) -> String {
    let p = path.replace('\\', "/");
    let (prefix, rest) = if let Some(i) = p.find("://") {
        (p[..i + 3].to_string(), p[i + 3..].to_string())
    } else if p.starts_with('/') {
        ("file://".to_string(), p)
    } else {
        // `c:/dir/file` or a relative path.
        ("file:///".to_string(), p)
    };
    let mut out = prefix;
    for b in rest.bytes() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'/' | b'-' | b'_' | b'.' | b'~' | b':' | b'%' | b'@' | b'+' | b',' | b'='
            );
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_five_entities_and_whitespace_controls() {
        assert_eq!(escaped(r#"a&b<c>d"e'f"#), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
        assert_eq!(escaped("x\ty\nz\r"), "x&#9;y&#10;z&#13;");
        // A regex from the mzIdentML enzyme block, where `<` sits in element text.
        assert_eq!(escaped("(?<=[KR])(?!P)"), "(?&lt;=[KR])(?!P)");
    }

    #[test]
    fn strips_characters_xml_1_0_does_not_allow() {
        // NUL, vertical tab, and a lone control character are all illegal.
        assert_eq!(escaped("a\u{0}b\u{B}c\u{1F}d"), "abcd");
        // Non-ASCII text that is legal stays.
        assert_eq!(escaped("caf\u{E9} \u{1F600}"), "caf\u{E9} \u{1F600}");
        // U+FFFE and U+FFFF are not XML characters.
        assert_eq!(escaped("a\u{FFFE}b\u{FFFF}c"), "abc");
    }

    #[test]
    fn a_plain_string_is_unchanged() {
        assert_eq!(escaped("sp|P06727|APOA4_HUMAN"), "sp|P06727|APOA4_HUMAN");
    }

    #[test]
    fn numbers_have_no_exponent_and_no_negative_zero() {
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(500.0), "500");
        assert_eq!(num32(57.0215), "57.0215");
        assert_eq!(num32(-17.026548), "-17.026548");
        assert_eq!(num32(-0.0), "0");
    }

    #[test]
    fn unix_time_becomes_the_right_utc_date() {
        assert_eq!(iso_from_unix(0), "1970-01-01T00:00:00");
        assert_eq!(iso_from_unix(1_000_000_000), "2001-09-09T01:46:40");
        // A leap day.
        assert_eq!(iso_from_unix(1_709_208_000), "2024-02-29T12:00:00");
    }

    #[test]
    fn paths_become_valid_uris() {
        assert_eq!(
            to_uri("/Users/a b/db.fasta"),
            "file:///Users/a%20b/db.fasta"
        );
        assert_eq!(
            to_uri("c:\\Users\\x\\db.fasta"),
            "file:///c:/Users/x/db.fasta"
        );
        // An existing URL keeps its scheme and its escapes.
        assert_eq!(to_uri("file:///a%20b/c.fasta"), "file:///a%20b/c.fasta");
        assert_eq!(to_uri("s3://bucket/key.fasta"), "s3://bucket/key.fasta");
    }
}
