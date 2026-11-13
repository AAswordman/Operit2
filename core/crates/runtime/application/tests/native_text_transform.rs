use operit_runtime::plugins::toolpkg::NativeTextTransform::*;

/// Compiles shared programs once, retains assignment order, and rejects incomplete or invalid indexes.
#[test]
fn indexed_batch_preserves_order_and_rejects_invalid_assignments() {
    let batch = TextTransformBatch {
        programs: vec![TextTransformProgram::default(), TextTransformProgram { rules: vec![TextTransformRule {
            pattern: "secret".into(), replacement: "public".into(), flags: "g".into(),
            kinds: vec!["markdown".into()], trimStrings: vec![],
        }] }],
        messagePrograms: vec![1, 0, 1],
    };
    let programs = CompiledTextTransformProgram::compile_batch(batch.clone(), 3).unwrap();
    assert_eq!(programs.iter().map(|program| program.apply("markdown", "secret")).collect::<Vec<_>>(),
        ["public", "secret", "public"]);
    assert!(CompiledTextTransformProgram::compile_batch(batch.clone(), 2).is_err());
    assert!(CompiledTextTransformProgram::compile_batch(TextTransformBatch { messagePrograms: vec![2], ..batch }, 1).is_err());
}

/// Measures only production Rust compilation and matching on the same eighty-message benchmark payloads.
#[test]
#[ignore]
fn completed_window_matching_cost() {
    for (count, bytes) in [(1, 2048), (10, 2048), (1, 32768)] {
        let source = "### History\nsecret information with **bold** and a link.\n";
        let text = source.repeat(bytes / source.len() + 1)[..bytes].to_string();
        let spec = TextTransformProgram { rules: (0..count).map(|index| TextTransformRule {
            pattern: if index == 0 { "secret".into() } else { format!("stage{index}") },
            replacement: if index == count - 1 { if count == 1 { "public".into() } else { "public10".into() } }
                else { format!("stage{}", index + 1) },
            flags: "g".into(), kinds: vec!["markdown".into()], trimStrings: vec![],
        }).collect() };
        let started = std::time::Instant::now();
        let compiled = CompiledTextTransformProgram::compile(&spec).unwrap();
        let compile_micros = started.elapsed().as_micros();
        let mut samples = Vec::new();
        for _ in 0..7 {
            let started = std::time::Instant::now();
            for _ in 0..80 { std::hint::black_box(compiled.apply("markdown", &text)); }
            samples.push(started.elapsed().as_micros());
        }
        println!("RUST_MATCH rules={count} bytes={bytes} compileMicros={compile_micros} samplesMicros={samples:?}");
    }
}

/// Compares the incremental matcher with an independent regex engine over exhaustive small inputs.
#[test]
fn exhaustive_alternation_and_chunk_partitions_match_reference() {
    for pattern in [
        "ab|a", "a|ab", "a+b", "a.*b|a", "a.*?b", "(ab)+", "[^b]+", "a.{0,3}b", "a$|b", "(?m:^a)",
    ] {
        let compiled = program(pattern, "[$0]", "g");
        let reference = regex::Regex::new(pattern).unwrap();
        for length in 0..7u32 {
            for encoded in 0..3usize.pow(length) {
                let mut number = encoded;
                let text: String = (0..length)
                    .map(|_| {
                        let ch = ['a', 'b', '!'][number % 3];
                        number /= 3;
                        ch
                    })
                    .collect();
                let expected = reference.replace_all(&text, "[$0]").into_owned();
                let mut stream = compiled.stream("markdown");
                let mut actual = String::new();
                for ch in text.chars() {
                    actual.push_str(&stream.push(&ch.to_string()));
                }
                actual.push_str(&stream.finish());
                assert_eq!(actual, expected, "pattern={pattern} text={text:?}");
            }
        }
    }
}

/// Keeps tool/status markup byte-identical while streaming text and reasoning independently.
#[test]
fn semantic_stream_preserves_original_markup_at_every_split() {
    let program = std::sync::Arc::new(
        CompiledTextTransformProgram::compile(&TextTransformProgram {
            rules: vec![TextTransformRule {
                pattern: "secret".into(),
                replacement: "public".into(),
                flags: "g".into(),
                kinds: vec!["markdown".into(), "thinking".into()],
                trimStrings: vec![],
            }],
        })
        .unwrap(),
    );
    for (source, expected) in [
        ("正文 secret<ThInKiNg data='secret'>secret</ThInKiNg>secret", "正文 public<ThInKiNg data='secret'>public</ThInKiNg>public"),
        ("secret<tool name='secret'><param name='q'>secret</param></tool>secret", "public<tool name='secret'><param name='q'>secret</param></tool>public"),
        ("<tool_result name='secret'>secret</tool_result><status type='secret'>secret</status>secret", "<tool_result name='secret'>secret</tool_result><status type='secret'>secret</status>public"),
        ("sec<think>ret</think>ret", "sec<think>ret</think>ret"),
        ("secret<tool name='secret'>secret", "public<tool name='secret'>secret"),
        ("secret<think>secret</thi", "public<think>public</thi"),
        ("secret<tool name='secret'", "public<tool name='secret'"),
        ("secret<status type='secret'/>secret", "public<status type='secret'/>public"),
        ("<private>secret</private>", "<private>public</private>"),
    ] {
        for split in source.char_indices().map(|(i, _)| i).chain(std::iter::once(source.len())) {
            let mut stream = NativeDisplayStream::new(program.clone());
            let mut actual = stream.push(&source[..split]).unwrap();
            actual.push_str(&stream.push(&source[split..]).unwrap());
            actual.push_str(&stream.finish().unwrap());
            assert_eq!(actual, expected, "source={source} split={split}");
        }
        let mut stream = NativeDisplayStream::new(program.clone());
        let mut actual = String::new();
        for ch in source.chars() { actual.push_str(&stream.push(&ch.to_string()).unwrap()); }
        actual.push_str(&stream.finish().unwrap());
        assert_eq!(actual, expected);
    }
    let mut stream = NativeDisplayStream::new(program);
    assert_eq!(
        stream.push("<tool name='secret'>").unwrap(),
        "<tool name='secret'>"
    );
    for _ in 0..1000 {
        assert_eq!(stream.push("secret payload ").unwrap(), "secret payload ");
    }
}

/// Restores unresolved captures and semantic boundaries, sharing compiled automata across revisions.
#[test]
fn savepoints_restore_pending_matches_and_pipeline_order() {
    let mut rules = TextTransformProgram {
        rules: vec![TextTransformRule {
            pattern: "secret".into(),
            replacement: "public".into(),
            flags: "g".into(),
            kinds: vec!["markdown".into()],
            trimStrings: vec![],
        }],
    };
    rules.rules.push(TextTransformRule {
        pattern: "public".into(),
        replacement: "shown".into(),
        ..rules.rules[0].clone()
    });
    let compiled = std::sync::Arc::new(CompiledTextTransformProgram::compile(&rules).unwrap());
    let mut stream = NativeDisplayStream::new(compiled);
    assert_eq!(stream.push("sec").unwrap(), "");
    let saved = stream.clone();
    let mut speculative = stream.push("ret!").unwrap();
    speculative.push_str(&stream.push(" next").unwrap());
    assert_eq!(speculative, "shown! next");
    stream = saved;
    assert_eq!(stream.push("ond!").unwrap(), "second!");
    assert_eq!(stream.finish().unwrap(), "");
}

/// Rejects unsupported programs before rendering begins instead of silently changing execution modes.
#[test]
fn invalid_or_nonprogressing_programs_fail_compilation() {
    for (pattern, flags) in [
        ("a*", "g"),
        ("(?=a)a", "g"),
        ("(a)\\1", "g"),
        ("a", "gg"),
        ("a", "y"),
    ] {
        let rules = TextTransformProgram {
            rules: vec![TextTransformRule {
                pattern: pattern.into(),
                replacement: "".into(),
                flags: flags.into(),
                kinds: vec!["markdown".into()],
                trimStrings: vec![],
            }],
        };
        assert!(
            CompiledTextTransformProgram::compile(&rules).is_err(),
            "{pattern} / {flags}"
        );
    }
    assert_eq!(
        program("(?<a>secret)", "$$:$<a>:$0:$99", "g").apply("markdown", "secret"),
        "$:secret:secret:"
    );
}

/// Measures sustained chunk processing while asserting that ordinary history never accumulates.
#[test]
fn sustained_stream_retains_only_unresolved_text() {
    let compiled = program("secret", "public", "g");
    let chunk = "ordinary text. ".repeat(74);
    for count in [64, 1024, 4096] {
        let mut stream = compiled.stream("markdown");
        let started = std::time::Instant::now();
        let mut emitted = 0;
        for _ in 0..count {
            emitted += stream.push(std::hint::black_box(&chunk)).len();
            assert!(stream.buffered_bytes() <= 4);
        }
        emitted += stream.finish().len();
        assert_eq!(emitted, count * chunk.len());
        println!(
            "native stream: {emitted} bytes in {:?}, retained {} bytes",
            started.elapsed(),
            stream.buffered_bytes()
        );
    }
}

/// Builds a native program with the same text scope used by plugin registrations.
fn program(pattern: &str, replacement: &str, flags: &str) -> CompiledTextTransformProgram {
    CompiledTextTransformProgram::compile(&TextTransformProgram {
        rules: vec![TextTransformRule {
            pattern: pattern.into(),
            replacement: replacement.into(),
            flags: flags.into(),
            kinds: vec!["markdown".into()],
            trimStrings: Vec::new(),
        }],
    })
    .unwrap()
}

/// Exercises all Unicode-safe split points, including greedy matches that wait for future input.
#[test]
fn chunk_boundaries_agree_with_complete_regex_replacement() {
    for (pattern, replacement, flags, text) in [
        ("secret", "public", "g", "你好 secret secret!"),
        ("秘密", "公开", "g", "你好 秘密 秘密!"),
        ("abab", "X", "g", "aba!ababababa abab"),
        ("a\\.b", "[$0]$$", "g", "a.b a-b a.b"),
        ("a+", "[$0]", "g", "xxaaa aa!"),
        ("a+?", "[$0]", "g", "xxaaa aa!"),
        ("a.*z|a", "X", "g", "aaaz aaa!"),
        (
            "<private>.*?</private>",
            "",
            "gs",
            "ok<private>hidden\n秘密</private>end",
        ),
        (
            "(?<word>secret)-(\\d+)",
            "$<word>:$2",
            "gi",
            "Secret-12 secret-3!",
        ),
        ("secret", "X", "", "secret secret"),
        ("^secret", "X", "g", "secret secret"),
        ("secret$", "X", "g", "secret secret"),
        ("^secret", "X", "gm", "secret\nsecret!\nsecret"),
        (
            "(?-u:\\b)secret(?-u:\\b)",
            "X",
            "g",
            "a secret notsecret 秘secret!",
        ),
    ] {
        let compiled = program(pattern, replacement, flags);
        let mut reference = regex::RegexBuilder::new(pattern);
        reference
            .case_insensitive(flags.chars().any(|flag| flag == 'i'))
            .multi_line(flags.chars().any(|flag| flag == 'm'))
            .dot_matches_new_line(flags.chars().any(|flag| flag == 's'));
        let reference = reference.build().unwrap();
        let template = replacement.replace("$<word>", "${word}");
        let expected = if flags.chars().any(|flag| flag == 'g') {
            reference.replace_all(text, template.as_str()).into_owned()
        } else {
            reference.replace(text, template.as_str()).into_owned()
        };
        assert_eq!(compiled.apply("markdown", text), expected);
        for split in text
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(text.len()))
        {
            let mut stream = compiled.stream("markdown");
            let mut actual = stream.push(&text[..split]);
            actual.push_str(&stream.push(&text[split..]));
            actual.push_str(&stream.finish());
            assert_eq!(actual, expected, "pattern={pattern} split={split}");
        }
        let mut byte_chunks = compiled.stream("markdown");
        let mut actual = String::new();
        for character in text.chars() {
            actual.push_str(&byte_chunks.push(&character.to_string()));
        }
        actual.push_str(&byte_chunks.finish());
        assert_eq!(
            actual, expected,
            "pattern={pattern}, single-character chunks"
        );
    }
}

/// Streams unmatched ordinary text immediately and with constant retained matching memory.
#[test]
fn ordinary_text_does_not_wait_for_completion_or_accumulate_history() {
    let compiled = program("secret", "public", "g");
    let mut stream = compiled.stream("markdown");
    for _ in 0..1000 {
        assert_eq!(stream.push("普通正文. "), "普通正文. ");
        assert!(stream.buffered_bytes() <= 4);
    }
    assert_eq!(stream.push("sec"), "");
    let mut output = stream.push("ret!");
    output.push_str(&stream.finish());
    assert_eq!(output, "public!");
}
