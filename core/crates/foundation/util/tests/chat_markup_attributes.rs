use operit_util::ChatMarkupRegex::{attr_value, ChatMarkupRegex};

/// Accepts mixed attribute syntax without consuming neighboring attributes or tag delimiters.
#[test]
fn attributes_accept_quoted_and_bare_values() {
    let markup = "<tool NAME = write_file status='success' call_id=call-1>body</tool>";
    assert_eq!(attr_value(markup, "name").as_deref(), Some("write_file"));
    assert_eq!(attr_value(markup, "status").as_deref(), Some("success"));
    assert_eq!(attr_value(markup, "call_id").as_deref(), Some("call-1"));
    assert_eq!(
        attr_value("<link path=文档/a.txt/>", "path").as_deref(),
        Some("文档/a.txt"),
    );
    assert_eq!(
        attr_value("<param name=path\tother=value>", "name").as_deref(),
        Some("path"),
    );
}

/// Keeps attribute-like text in values, nested tags, and bodies out of opening-tag fields.
#[test]
fn attributes_only_read_opening_tag_fields() {
    for markup in [
        "<tool><param name=path>a.txt</param></tool>",
        "<tool data-name=ignored>name=write_file</tool>",
        "<tool title=\"name='ignored'\">body</tool>",
        "<tool name=>body</tool>",
        "<tool name= status=success>body</tool>",
        "<param name=\"various_search</param>",
    ] {
        assert_eq!(attr_value(markup, "name"), None, "{markup}");
    }
    assert_eq!(
        attr_value("<tool title=\"name='ignored'\" name=write_file>", "name").as_deref(),
        Some("write_file"),
    );
}

/// Uses the same bare attributes for execution matches and hidden provider metadata.
#[test]
fn shared_markup_readers_accept_bare_attributes() {
    let calls = ChatMarkupRegex::tool_call_matches(
        "<tool_generated name=write_file><param name=path>a.txt</param></tool_generated>",
    );
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "write_file");
    assert_eq!(
        ChatMarkupRegex::extract_gemini_thought_signature(
            "<meta provider=gemini:thought_signature>signature</meta>",
        )
        .as_deref(),
        Some("signature"),
    );
}
