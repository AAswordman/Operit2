use super::*;
use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    /// Creates a directory owned exclusively by this search test.
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "operit-grep-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    /// Builds a search whose file filter includes every fixture file.
    fn request(&self, maximum: usize, context: usize) -> GrepCodeRequest {
        GrepCodeRequest {
            path: self.0.to_string_lossy().into_owned(),
            pattern: "needle".into(),
            filePattern: "*".into(),
            caseInsensitive: false,
            contextLines: context,
            maxResults: maximum,
        }
    }
}

impl Drop for Fixture {
    /// Removes only the directory created by this fixture.
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

/// A request limit spans files and stops traversal as soon as it is satisfied.
#[test]
fn result_budget_is_global_and_stops_before_searching_more_files() {
    let fixture = Fixture::new();
    for index in 0..3 {
        fs::write(fixture.0.join(format!("{index}.txt")), "needle\nneedle\n").unwrap();
    }
    let result = grepCode(fixture.request(3, 0)).unwrap();
    assert_eq!(result.totalMatches, 3);
    assert_eq!(result.filesSearched, 2);
    assert_eq!(
        result
            .matches
            .iter()
            .map(|file| file.lineMatches.len())
            .sum::<usize>(),
        3
    );
    assert!(result
        .matches
        .iter()
        .flat_map(|file| &file.lineMatches)
        .all(|hit| hit.matchContext.is_none()));
}

/// Zero results do not open or traverse the search root.
#[test]
fn zero_budget_does_not_walk() {
    let fixture = Fixture::new();
    let mut request = fixture.request(0, 3);
    request.path = fixture.0.join("missing").to_string_lossy().into_owned();
    let result = grepCode(request).unwrap();
    assert_eq!(result.totalMatches, 0);
    assert_eq!(result.filesSearched, 0);
}

/// Adjacent matches retain overlapping windows, CRLF and empty context lines.
#[test]
fn contexts_are_exact_for_overlapping_matches_and_file_edges() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("source.txt"),
        b"needle first\r\n\r\nneedle second\r\ntail\r\nneedle last",
    )
    .unwrap();
    let result = grepCode(fixture.request(10, 1)).unwrap();
    let hits = &result.matches[0].lineMatches;
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].matchContext.as_deref(), Some("needle first\n"));
    assert_eq!(
        hits[1].matchContext.as_deref(),
        Some("\nneedle second\ntail")
    );
    assert_eq!(hits[2].matchContext.as_deref(), Some("tail\nneedle last"));
    assert_eq!(
        hits.iter().map(|hit| hit.lineNumber).collect::<Vec<_>>(),
        [1, 3, 5]
    );
}

/// Matches encountered in the final context must not add results or extend the tail.
#[test]
fn matching_tail_context_does_not_exceed_the_budget() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("source.txt"),
        "before\nneedle 1\nneedle 2\nneedle 3\nneedle 4\n",
    )
    .unwrap();
    let result = grepCode(fixture.request(1, 2)).unwrap();
    assert_eq!(result.totalMatches, 1);
    let hit = &result.matches[0].lineMatches[0];
    assert_eq!(
        hit.matchContext.as_deref(),
        Some("before\nneedle 1\nneedle 2\nneedle 3")
    );
}

/// Non-UTF-8 text follows ripgrep's lossy display semantics instead of losing all matches.
#[test]
fn text_with_invalid_utf8_remains_searchable_and_binary_files_are_skipped() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("text.txt"),
        b"\xff before\nneedle\n\xff after\n",
    )
    .unwrap();
    fs::write(fixture.0.join("binary.dat"), b"\x00needle\n").unwrap();
    let result = grepCode(fixture.request(10, 1)).unwrap();
    assert_eq!(result.totalMatches, 1);
    assert_eq!(
        result.matches[0].lineMatches[0].matchContext.as_deref(),
        Some("\u{fffd} before\nneedle\n\u{fffd} after")
    );
}

/// Case-insensitive filtering and single-file searches use the same shared search engine.
#[test]
fn filters_and_single_file_roots_preserve_their_contract() {
    let fixture = Fixture::new();
    let path = fixture.0.join("SOURCE.TXT");
    fs::write(&path, "NEEDLE\n").unwrap();
    fs::write(fixture.0.join("skip.log"), "needle\n").unwrap();
    let mut request = fixture.request(10, 0);
    request.filePattern = "*.txt".into();
    request.caseInsensitive = true;
    let result = grepCode(request.clone()).unwrap();
    assert_eq!(result.totalMatches, 1);
    assert_eq!(result.filesSearched, 1);
    request.path = path.to_string_lossy().into_owned();
    assert_eq!(grepCode(request).unwrap().totalMatches, 1);
}

struct ChunkedReader {
    input: Cursor<Vec<u8>>,
}

impl Read for ChunkedReader {
    /// Delivers small chunks so accidental reads past the requested tail are observable.
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let length = bytes.len().min(16);
        self.input.read(&mut bytes[..length])
    }
}

/// A bounded streaming search must not read the rest of a large matching input.
#[test]
fn matching_context_stops_reading_a_large_tail() {
    for pattern in ["needle", r"needle|\n"] {
        let matcher = RegexMatcherBuilder::new().build(pattern).unwrap();
        let mut reader = ChunkedReader {
            input: Cursor::new(b"needle\n".repeat(100_000)),
        };
        let mut sink = GrepSink::new(1, 2);
        SearcherBuilder::new()
            .line_number(true)
            .before_context(2)
            .after_context(2)
            .max_matches(Some(1))
            .build()
            .search_reader(&matcher, &mut reader, &mut sink)
            .unwrap();
        let hits = sink.intoMatches();
        assert_eq!(hits.len(), 1, "{pattern}");
        assert_eq!(
            hits[0].matchContext.as_deref(),
            Some("needle\nneedle\nneedle")
        );
        assert!(
            reader.input.position() < 100,
            "The search must stop after its small buffered tail: {pattern}"
        );
    }
}
