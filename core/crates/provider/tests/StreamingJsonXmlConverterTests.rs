use super::{StreamingJsonXmlConverter, StreamingJsonXmlEvent};

/// Collects converter events without changing the emitted XML content.
fn render(parts: &[&str]) -> String {
    let mut parser = StreamingJsonXmlConverter::new();
    let mut events = Vec::new();
    for part in parts {
        events.extend(parser.feed(part));
    }
    events.extend(parser.flush());
    events
        .into_iter()
        .map(|event| match event {
            StreamingJsonXmlEvent::Tag(text) | StreamingJsonXmlEvent::Content(text) => text,
        })
        .collect()
}

/// Preserves supplementary characters at every possible JSON fragment boundary.
#[test]
fn surrogate_pairs_survive_every_fragment_boundary() {
    let json = r#"{"text":"hello\uD83D\uDE00\ud834\udd1e!","next":"\u4f60\u597d"}"#;
    let expected =
        "\n  <param name=\"text\">hello😀𝄞!</param>\n  <param name=\"next\">你好</param>";
    for split in 0..=json.len() {
        assert_eq!(render(&["{}", &json[..split], &json[split..]]), expected);
    }
    let fragments: Vec<_> = (0..json.len())
        .map(|index| &json[index..index + 1])
        .collect();
    assert_eq!(render(&fragments), expected);
}

/// Emits a supplementary character as soon as its low surrogate is complete.
#[test]
fn surrogate_pair_emits_before_the_parameter_finishes() {
    let mut parser = StreamingJsonXmlConverter::new();
    parser.feed(r#"{"text":"hello"#);
    assert!(parser.feed(r#"\uD83D"#).is_empty());
    assert!(parser.feed(r#"\uDE0"#).is_empty());
    let events = parser.feed("0");
    assert!(matches!(events.as_slice(), [StreamingJsonXmlEvent::Content(text)] if text == "😀"));
    assert!(parser.hasUnfinishedParam());
}

/// Keeps literal Unicode, ordinary escapes, BMP characters and XML escaping intact.
#[test]
fn unicode_decoding_preserves_other_string_content() {
    assert_eq!(
        render(&[r#"{"text":"😀\uD83D\uDE00\u4F60\u597D\u003C\u0026\u003E\n\"\\"}"#]),
        "\n  <param name=\"text\">😀😀你好&lt;&amp;&gt;\n&quot;\\</param>"
    );
}

/// Does not combine surrogate halves across literal text, escapes or parameters.
#[test]
fn surrogate_state_does_not_cross_string_boundaries() {
    assert_eq!(
        render(&[r#"{"first":"\uD83Dx\uDE00","second":"\uD83D\n\uDE00","third":"\uD83D","fourth":"\uDE00"}"#]),
        "\n  <param name=\"first\">x</param>\n  <param name=\"second\">\n</param>\n  <param name=\"third\"></param>\n  <param name=\"fourth\"></param>"
    );
}
