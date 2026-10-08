use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

/// Characters that must be percent-encoded in an IRI fragment.
const IRI_FRAGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b']')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'^')
    .add(b'`')
    .add(b'\\');

/// Percent-encodes `s` for use as (part of) an IRI fragment. Non-ASCII is always encoded.
pub fn encode_fragment(s: &str) -> String {
    utf8_percent_encode(s, IRI_FRAGMENT_ENCODE_SET).to_string()
}

/// True for an absolute http(s) IRI that can be written between `<` and `>` in N-Triples as is.
pub fn is_http_iri(iri: &str) -> bool {
    (iri.starts_with("http://") || iri.starts_with("https://"))
        && !iri.chars().any(|c| {
            c.is_control()
                || c.is_whitespace()
                || matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_fragment_spaces_and_hash() {
        assert_eq!(encode_fragment("sample name#1"), "sample%20name%231");
    }

    #[test]
    fn test_encode_fragment_keeps_slash_colon_underscore() {
        assert_eq!(encode_fragment("bsllmner/a:b_c-d.e"), "bsllmner/a:b_c-d.e");
    }

    #[test]
    fn test_encode_fragment_non_ascii_and_percent() {
        assert_eq!(encode_fragment("β%"), "%CE%B2%25");
    }

    #[test]
    fn test_is_http_iri() {
        assert!(is_http_iri(
            "http://purl.obolibrary.org/obo/Cellosaurus#CVCL_0027"
        ));
        assert!(is_http_iri("https://example.org/x"));
        for bad in [
            "",
            "not an iri",
            "ftp://x",
            "http://x y",
            "http://x>y",
            "http://x\"y",
            "http://x\ny",
            "http://x\\y",
        ] {
            assert!(!is_http_iri(bad), "{:?}", bad);
        }
    }
}
