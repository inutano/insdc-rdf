/// Escape a string for use as a Turtle string literal.
pub fn escape_turtle_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Escape a string for use as an N-Triples string literal.
pub fn escape_ntriples_string(s: &str) -> String {
    // N-Triples uses the same escaping rules as Turtle for string literals.
    escape_turtle_string(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_quotes() {
        assert_eq!(escape_turtle_string("say \"hi\""), "say \\\"hi\\\"");
    }

    #[test]
    fn test_escape_newline() {
        assert_eq!(escape_turtle_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_backslash() {
        assert_eq!(escape_turtle_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_escape_tab() {
        assert_eq!(escape_turtle_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_escape_cr() {
        assert_eq!(escape_turtle_string("cr\rhere"), "cr\\rhere");
    }
}
