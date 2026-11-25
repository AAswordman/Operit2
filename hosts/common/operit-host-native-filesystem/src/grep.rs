use std::collections::BTreeMap;
use std::io;

use globset::GlobBuilder;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkContext, SinkMatch};
use ignore::WalkBuilder;
use operit_host_contract::{
    GrepCodeRequest, GrepCodeResult, GrepFileMatch, GrepLineMatch, HostError, HostResult,
};

/// Searches native files once, sharing a result budget across the entire directory walk.
#[allow(non_snake_case)]
pub fn grepCode(request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
    if request.pattern.trim().is_empty() {
        return Err(HostError::new("Pattern parameter is required"));
    }
    if request.filePattern.trim().is_empty() {
        return Err(HostError::new("file_pattern parameter is required"));
    }
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(request.caseInsensitive)
        .build(&request.pattern)
        .map_err(|error| HostError::new(format!("Invalid regex pattern: {error}")))?;
    let fileMatcher = GlobBuilder::new(&request.filePattern)
        .case_insensitive(request.caseInsensitive)
        .build()
        .map_err(|error| HostError::new(format!("Invalid file pattern: {error}")))?
        .compile_matcher();
    let mut result = GrepCodeResult {
        matches: Vec::new(),
        totalMatches: 0,
        filesSearched: 0,
    };
    if request.maxResults == 0 {
        return Ok(result);
    }
    let mut builder = SearcherBuilder::new();
    builder
        .line_number(true)
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .before_context(request.contextLines)
        .after_context(request.contextLines);
    for entry in WalkBuilder::new(&request.path).build() {
        let entry = entry.map_err(|error| HostError::new(format!("walk error: {error}")))?;
        let filePath = entry.path();
        if !filePath.is_file() || !fileMatcher.is_match(filePath) {
            continue;
        }
        let remaining = request.maxResults - result.totalMatches;
        let mut sink = GrepSink::new(remaining, request.contextLines);
        result.filesSearched += 1;
        builder
            .max_matches(Some(remaining as u64))
            .build()
            .search_path(&matcher, filePath, &mut sink)?;
        let lineMatches = sink.intoMatches();
        if !lineMatches.is_empty() {
            result.totalMatches += lineMatches.len();
            result.matches.push(GrepFileMatch {
                filePath: filePath.to_string_lossy().into_owned(),
                lineMatches,
            });
        }
        if result.totalMatches == request.maxResults {
            break;
        }
    }
    Ok(result)
}

/// Retains only matching lines and the context emitted by ripgrep's streaming searcher.
#[allow(non_snake_case)]
struct GrepSink {
    maximum: usize,
    contextLines: usize,
    lineMatches: Vec<GrepLineMatch>,
    context: BTreeMap<usize, String>,
    lastContextLine: Option<usize>,
}

#[allow(non_snake_case)]
impl GrepSink {
    /// Creates one file's bounded result collector without allocating for the entire file.
    fn new(maximum: usize, contextLines: usize) -> Self {
        Self {
            maximum,
            contextLines,
            lineMatches: Vec::new(),
            context: BTreeMap::new(),
            lastContextLine: None,
        }
    }

    /// Includes tail matches as context without increasing or extending the result budget.
    fn recordContext(&mut self, lineNumber: usize, content: &str) -> bool {
        if self.contextLines > 0 && self.lastContextLine.is_none_or(|last| lineNumber <= last) {
            self.context.insert(lineNumber, content.to_owned());
        }
        self.lastContextLine.is_none_or(|last| lineNumber < last)
    }

    /// Builds each match's exact context window from the lines already visited by the searcher.
    fn intoMatches(mut self) -> Vec<GrepLineMatch> {
        if self.contextLines > 0 {
            for hit in &mut self.lineMatches {
                let first = hit.lineNumber.saturating_sub(self.contextLines);
                let last = hit.lineNumber.saturating_add(self.contextLines);
                hit.matchContext = Some(
                    self.context
                        .range(first..=last)
                        .map(|(_, line)| line.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
        }
        self.lineMatches
    }
}

impl Sink for GrepSink {
    type Error = io::Error;

    /// Counts only admitted matches; ripgrep may also report matches in trailing context.
    fn matched(&mut self, _searcher: &Searcher, hit: &SinkMatch<'_>) -> io::Result<bool> {
        let number = hit.line_number().expect("grep line numbers are enabled") as usize;
        let content = lineText(hit.bytes());
        if self.lineMatches.len() < self.maximum {
            if self.lineMatches.len() + 1 == self.maximum {
                self.lastContextLine = Some(number.saturating_add(self.contextLines));
            }
            let keepSearching = self.recordContext(number, &content);
            self.lineMatches.push(GrepLineMatch {
                lineNumber: number,
                lineContent: content,
                matchContext: None,
            });
            return Ok(keepSearching);
        }
        Ok(self.recordContext(number, &content))
    }

    /// Stops after the final admitted match's context even when later lines also match.
    fn context(&mut self, _searcher: &Searcher, line: &SinkContext<'_>) -> io::Result<bool> {
        let number = line.line_number().expect("grep line numbers are enabled") as usize;
        Ok(self.recordContext(number, &lineText(line.bytes())))
    }
}

/// Removes one line terminator while preserving empty lines and literal carriage returns.
#[allow(non_snake_case)]
fn lineText(bytes: &[u8]) -> String {
    let bytes = match bytes.strip_suffix(b"\n") {
        Some(line) => line.strip_suffix(b"\r").unwrap_or(line),
        None => bytes,
    };
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests;
