use std::collections::BTreeSet;

use similar::{capture_diff_slices, Algorithm, DiffTag};

const MATCH_THRESHOLD: f64 = 0.90;

/// Applies AI-provided structured edits to bound file content.
pub struct FileBindingService;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Public edit action accepted from structured file-binding requests.
pub enum StructuredEditAction {
    REPLACE,
    DELETE,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Public structured edit operation for a file-binding request.
pub struct StructuredEditOperation {
    pub action: StructuredEditAction,
    pub oldContent: String,
    pub newContent: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EditAction {
    REPLACE,
    DELETE,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Internal normalized edit operation used by the fuzzy patch engine.
pub struct EditOperation {
    action: EditAction,
    oldContent: String,
    newContent: String,
}

#[derive(Clone, Debug, PartialEq)]
struct MatchSearchResult {
    bestScore: f64,
    startLine: isize,
    endLine: isize,
    sizeDiff: usize,
    lengthDiff: usize,
    windows: usize,
    lcsCalculations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum UnifiedDiffLine {
    Context {
        oldLine: usize,
        newLine: usize,
        text: String,
    },
    Delete {
        oldLine: usize,
        text: String,
    },
    Insert {
        newLine: usize,
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DiffOp {
    Equal(String),
    Delete(String),
    Insert(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UnifiedDiffHunk {
    oldStart: usize,
    oldCount: usize,
    newStart: usize,
    newCount: usize,
    lines: Vec<UnifiedDiffLine>,
}

impl FileBindingService {
    /// Processes AI-generated file-binding text and returns modified content plus a diff.
    pub fn processFileBinding(
        &self,
        originalContent: &str,
        aiGeneratedCode: &str,
    ) -> (String, String) {
        if !originalContent.is_empty() && !aiGeneratedCode.contains("[START-") {
            let errorMsg = "If you want to rewrite an entire existing file: please delete_file first then use apply_file with type=create (do not overwrite directly).If you want to modify a file: please use apply_file with type=replace/delete and provide old/new (or old).";
            return (originalContent.to_string(), errorMsg.to_string());
        }

        if aiGeneratedCode.contains("[START-") {
            let (success, resultString) = self.applyFuzzyPatch(originalContent, aiGeneratedCode);
            if success {
                let diffString =
                    self.generateDiff(&originalContent.replace("\r\n", "\n"), &resultString);
                return (resultString, diffString);
            }
            return (
                originalContent.to_string(),
                format!("Error: Could not apply patch. Reason: {resultString}"),
            );
        }

        let normalizedOriginalContent = originalContent.replace("\r\n", "\n");
        let normalizedAiGeneratedCode = aiGeneratedCode.replace("\r\n", "\n").trim().to_string();
        let diffString = self.generateDiff(&normalizedOriginalContent, &normalizedAiGeneratedCode);
        (normalizedAiGeneratedCode, diffString)
    }

    /// Applies structured replacement and deletion operations to bound file content.
    pub fn processFileBindingOperations(
        &self,
        originalContent: &str,
        operations: &[StructuredEditOperation],
    ) -> (String, String) {
        if operations.is_empty() {
            return (
                originalContent.to_string(),
                "Error: No valid edit operations provided".to_string(),
            );
        }

        let internalOps = operations
            .iter()
            .filter_map(|op| {
                let old = op.oldContent.clone();
                if old.trim().is_empty() {
                    return None;
                }
                match op.action {
                    StructuredEditAction::REPLACE => {
                        if op.newContent.trim().is_empty() {
                            None
                        } else {
                            Some(EditOperation {
                                action: EditAction::REPLACE,
                                oldContent: old,
                                newContent: op.newContent.clone(),
                            })
                        }
                    }
                    StructuredEditAction::DELETE => Some(EditOperation {
                        action: EditAction::DELETE,
                        oldContent: old,
                        newContent: String::new(),
                    }),
                }
            })
            .collect::<Vec<_>>();

        if internalOps.is_empty() {
            return (
                originalContent.to_string(),
                "Error: No valid edit operations provided".to_string(),
            );
        }

        let (success, resultString) = self.applyFuzzyOperations(originalContent, &internalOps);
        if success {
            let diffString =
                self.generateDiff(&originalContent.replace("\r\n", "\n"), &resultString);
            return (resultString, diffString);
        }
        (
            originalContent.to_string(),
            format!("Error: Could not apply patch. Reason: {resultString}"),
        )
    }

    fn generateDiff(&self, original: &str, modified: &str) -> String {
        self.generateUnifiedDiff(original, modified)
    }

    /// Builds a unified diff between the original and modified file content.
    pub fn generateUnifiedDiff(&self, original: &str, modified: &str) -> String {
        if original == modified {
            return "No changes detected (files are identical)".to_string();
        }
        let originalLines = kotlinLines(original);
        let modifiedLines = kotlinLines(modified);
        let diffOps = buildDiffOps(&originalLines, &modifiedLines);

        if diffOps.iter().all(|op| matches!(op, DiffOp::Equal(_))) {
            return "No changes detected (files are identical)".to_string();
        }

        let annotatedLines = annotateDiffOps(&diffOps);
        let hunks = buildUnifiedDiffHunks(&annotatedLines, 3);
        let mut resultLines = Vec::new();
        for hunk in hunks {
            resultLines.push(format!(
                "@@ -{},{} +{},{} @@",
                hunk.oldStart, hunk.oldCount, hunk.newStart, hunk.newCount
            ));
            for line in hunk.lines {
                match line {
                    UnifiedDiffLine::Context { text, .. } => {
                        resultLines.push(format!(" {}", text));
                    }
                    UnifiedDiffLine::Delete { text, .. } => {
                        resultLines.push(format!("-{}", text));
                    }
                    UnifiedDiffLine::Insert { text, .. } => {
                        resultLines.push(format!("+{}", text));
                    }
                }
            }
        }

        resultLines.join("\n")
    }

    fn applyFuzzyPatch(&self, originalContent: &str, aiPatchCode: &str) -> (bool, String) {
        let operations = self.parseEditOperations(aiPatchCode);
        if operations.is_empty() {
            return (
                false,
                "No valid edit operations found in the patch code.".to_string(),
            );
        }
        self.applyFuzzyOperations(originalContent, &operations)
    }

    fn applyFuzzyOperations(
        &self,
        originalContent: &str,
        operations: &[EditOperation],
    ) -> (bool, String) {
        let mut originalLines = originalContent
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut enrichedOps = Vec::<(EditOperation, usize, usize)>::new();

        for op in operations {
            let (start, end, bestScore) = self.findBestMatchRange(&originalLines, &op.oldContent);
            if start < 0 {
                // Fast path for short OLD fragments inside long lines: the line-window
                // n-gram score is diluted by the surrounding text, so an exact, unique
                // containment match is accepted even when it cannot clear the threshold.
                if let Some((s, e)) = self.findUniqueContainmentRange(&originalLines, &op.oldContent) {
                    enrichedOps.push((op.clone(), s as usize, e as usize));
                    continue;
                }
                return (
                    false,
                    format!(
                        "Could not find a match for an OLD block (best similarity {:.3}, required {:.2}). The file may have changed too much, or the OLD text is a short fragment of a much longer line; in that case provide the complete line as OLD.",
                        bestScore,
                        MATCH_THRESHOLD
                    ),
                );
            }
            if self.hasMultiplePerfectMatches(originalContent, &op.oldContent) {
                return (
                    false,
                    "Found multiple perfect matches for an OLD block in the target file. Please refine the patch so it only matches a single location.".to_string(),
                );
            }
            enrichedOps.push((op.clone(), start as usize, end as usize));
        }

        enrichedOps.sort_by(|left, right| right.1.cmp(&left.1));
        for (op, start, end) in enrichedOps {
            let originalSegment = originalLines[start..=end].to_vec();
            let boundaryPreservingLines =
                self.tryApplyBoundaryPreservingEdit(&originalSegment, &op);
            for index in (start..=end).rev() {
                originalLines.remove(index);
            }
            if let Some(lines) = boundaryPreservingLines {
                for (offset, line) in lines.into_iter().enumerate() {
                    originalLines.insert(start + offset, line);
                }
            } else if op.action == EditAction::REPLACE {
                let newLines = op
                    .newContent
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                for (offset, line) in newLines.into_iter().enumerate() {
                    originalLines.insert(start + offset, line);
                }
            }
        }

        (true, originalLines.join("\n"))
    }

    fn tryApplyBoundaryPreservingEdit(
        &self,
        originalSegment: &[String],
        op: &EditOperation,
    ) -> Option<Vec<String>> {
        if originalSegment.is_empty() {
            return None;
        }
        let oldLines = op.oldContent.lines().collect::<Vec<_>>();
        if oldLines.is_empty() {
            return None;
        }
        let startIndex = self.findUniqueOccurrence(&originalSegment[0], oldLines[0])?;
        let endIndex =
            self.findUniqueOccurrence(originalSegment.last()?, oldLines.last().copied()?)?;
        let prefix = &originalSegment[0][..startIndex];
        let suffix = &originalSegment.last()?[endIndex + oldLines.last()?.len()..];
        match op.action {
            EditAction::REPLACE => {
                let newLines = op.newContent.lines().collect::<Vec<_>>();
                if newLines.is_empty() {
                    return None;
                }
                if newLines.len() == 1 {
                    Some(vec![format!("{prefix}{}{suffix}", newLines[0])])
                } else {
                    let mut output = Vec::new();
                    output.push(format!("{prefix}{}", newLines[0]));
                    for line in newLines
                        .iter()
                        .skip(1)
                        .take(newLines.len().saturating_sub(2))
                    {
                        output.push((*line).to_string());
                    }
                    output.push(format!("{}{suffix}", newLines[newLines.len() - 1]));
                    Some(output)
                }
            }
            EditAction::DELETE => {
                let updatedLine = format!("{prefix}{suffix}");
                if updatedLine.is_empty() {
                    Some(Vec::new())
                } else {
                    Some(vec![updatedLine])
                }
            }
        }
    }

    fn findUniqueOccurrence(&self, line: &str, fragment: &str) -> Option<usize> {
        if fragment.is_empty() {
            return None;
        }
        let firstIndex = line.find(fragment)?;
        if line[firstIndex + fragment.len()..].find(fragment).is_some() {
            return None;
        }
        Some(firstIndex)
    }

    fn parseEditOperations(&self, patchCode: &str) -> Vec<EditOperation> {
        let mut operations = Vec::new();
        let lines = patchCode.lines().collect::<Vec<_>>();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.starts_with("[START-") {
                let actionStr = line
                    .trim_start_matches("[START-")
                    .split(']')
                    .next()
                    .unwrap_or("");
                let action = match actionStr {
                    "REPLACE" => EditAction::REPLACE,
                    "DELETE" => EditAction::DELETE,
                    _ => {
                        i += 1;
                        continue;
                    }
                };
                let mut oldContent = String::new();
                let mut newContent = String::new();
                let mut inBlock: Option<&str> = None;
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with(&format!("[END-{actionStr}]"))
                {
                    let currentLine = lines[i];
                    let trimmedLine = currentLine.trim();
                    if trimmedLine.starts_with("[OLD]") {
                        inBlock = Some("OLD");
                        let inline = currentLine
                            .split_once("[OLD]")
                            .map(|(_, v)| v)
                            .unwrap_or("");
                        if !inline.is_empty() {
                            oldContent.push_str(inline);
                            oldContent.push('\n');
                        }
                    } else if trimmedLine.starts_with("[NEW]") {
                        inBlock = Some("NEW");
                        let inline = currentLine
                            .split_once("[NEW]")
                            .map(|(_, v)| v)
                            .unwrap_or("");
                        if !inline.is_empty() {
                            newContent.push_str(inline);
                            newContent.push('\n');
                        }
                    } else if trimmedLine.starts_with("[/OLD]") || trimmedLine.starts_with("[/NEW]")
                    {
                        inBlock = None;
                    } else {
                        match inBlock {
                            Some("OLD") => {
                                oldContent.push_str(currentLine);
                                oldContent.push('\n');
                            }
                            Some("NEW") => {
                                newContent.push_str(currentLine);
                                newContent.push('\n');
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }
                let normalizedOld = trimTrailingNewline(&oldContent);
                let normalizedNew = trimTrailingNewline(&newContent);
                if normalizedOld.trim().is_empty() {
                    i += 1;
                    continue;
                }
                if action == EditAction::REPLACE && normalizedNew.trim().is_empty() {
                    i += 1;
                    continue;
                }
                operations.push(EditOperation {
                    action,
                    oldContent: normalizedOld,
                    newContent: normalizedNew,
                });
            }
            i += 1;
        }
        operations
    }

    fn findBestMatchRange(&self, originalLines: &[String], oldContent: &str) -> (isize, isize, f64) {
        let oldContentLines = oldContent.lines().collect::<Vec<_>>();
        let numOldLines = oldContentLines.len();
        if numOldLines == 0 || originalLines.is_empty() {
            return (-1, -1, 0.0);
        }

        let normalizedOldContent = normalizeWhitespace(oldContent);
        let normalizedOldLength = normalizedOldContent.len();
        let baseNgrams = self.buildNgrams(&normalizedOldContent, 3);
        if baseNgrams.is_empty() {
            return (-1, -1, 0.0);
        }

        let delta = (numOldLines as f64 * 0.2) as isize + 2;
        let min_size = (numOldLines as isize - delta).max(1) as usize;
        let max_size = numOldLines + delta as usize;
        let mut best = MatchSearchResult {
            bestScore: 0.0,
            startLine: -1,
            endLine: -1,
            sizeDiff: usize::MAX,
            lengthDiff: usize::MAX,
            windows: 0,
            lcsCalculations: 0,
        };

        for i in 0..originalLines.len() {
            for size in min_size..=max_size {
                let endLine = i + size;
                if endLine > originalLines.len() {
                    break;
                }
                let normalizedWindow = normalizeWhitespace(&originalLines[i..endLine].join("\n"));
                let sizeDiff = size.abs_diff(numOldLines);
                let lengthDiff = normalizedWindow.len().abs_diff(normalizedOldLength);
                let score = self.ngramSimilarity(&baseNgrams, &normalizedWindow, 3);
                let isBetter = score > best.bestScore
                    || (score == best.bestScore
                        && (sizeDiff < best.sizeDiff
                            || (sizeDiff == best.sizeDiff
                                && (lengthDiff < best.lengthDiff
                                    || (lengthDiff == best.lengthDiff
                                        && (best.startLine == -1
                                            || i as isize <= best.startLine))))));
                if isBetter {
                    best.bestScore = score;
                    best.startLine = i as isize;
                    best.endLine = endLine as isize - 1;
                    best.sizeDiff = sizeDiff;
                    best.lengthDiff = lengthDiff;
                }
            }
        }

        if best.bestScore > MATCH_THRESHOLD {
            (best.startLine, best.endLine, best.bestScore)
        } else {
            (-1, -1, best.bestScore)
        }
    }


    /// Locates an OLD block that appears exactly once when the file is viewed as
    /// consecutive line windows of the same line count, compared after whitespace
    /// normalization. Used as a fallback for fragments inside long lines.
    fn findUniqueContainmentRange(&self, originalLines: &[String], oldContent: &str) -> Option<(isize, isize)> {
        let normalizedOld = normalizeWhitespace(oldContent);
        if normalizedOld.is_empty() {
            return None;
        }
        let numOldLines = oldContent.lines().count();
        if numOldLines == 0 || originalLines.is_empty() {
            return None;
        }
        let mut found: Option<(usize, usize)> = None;
        let mut i = 0usize;
        while i + numOldLines <= originalLines.len() {
            let window = normalizeWhitespace(&originalLines[i..i + numOldLines].join("\n"));
            if window.contains(&normalizedOld) {
                if found.is_some() {
                    return None; // ambiguous, let the caller report no match
                }
                found = Some((i, i + numOldLines - 1));
            }
            i += 1;
        }
        found.map(|(start, end)| (start as isize, end as isize))
    }

    fn buildNgrams(&self, s: &str, n: usize) -> BTreeSet<String> {
        if s.chars().count() < n {
            return BTreeSet::new();
        }
        let chars = s.chars().collect::<Vec<_>>();
        chars
            .windows(n)
            .map(|window| window.iter().collect::<String>())
            .collect()
    }

    fn ngramSimilarity(&self, baseNgrams: &BTreeSet<String>, s2: &str, n: usize) -> f64 {
        if baseNgrams.is_empty() || s2.is_empty() || s2.chars().count() < n {
            return 0.0;
        }
        let ngrams2 = self.buildNgrams(s2, n);
        if ngrams2.is_empty() {
            return 0.0;
        }
        let intersection = baseNgrams.intersection(&ngrams2).count();
        let union = baseNgrams.len() + ngrams2.len() - intersection;
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    fn hasMultiplePerfectMatches(&self, originalContent: &str, oldContent: &str) -> bool {
        let normalizedOld = normalizeWhitespace(oldContent);
        if normalizedOld.is_empty() {
            return false;
        }
        let normalizedOriginal = normalizeWhitespace(originalContent);
        let mut count = 0;
        let mut search_start = 0;
        while let Some(index) = normalizedOriginal[search_start..].find(&normalizedOld) {
            count += 1;
            if count > 1 {
                return true;
            }
            search_start += index + normalizedOld.len();
        }
        false
    }
}

#[allow(non_snake_case)]
fn kotlinLines(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
            .collect()
    }
}

#[allow(non_snake_case)]
/// Computes exact line edits with Myers' linear-space divide-and-conquer algorithm.
fn buildDiffOps(originalLines: &[String], modifiedLines: &[String]) -> Vec<DiffOp> {
    let mut result = Vec::new();
    for op in capture_diff_slices(Algorithm::Myers, originalLines, modifiedLines) {
        if op.tag() == DiffTag::Equal {
            result.extend(
                originalLines[op.old_range()].iter().cloned().map(DiffOp::Equal),
            );
        } else {
            result.extend(originalLines[op.old_range()].iter().cloned().map(DiffOp::Delete));
            result.extend(modifiedLines[op.new_range()].iter().cloned().map(DiffOp::Insert));
        }
    }
    result
}

#[allow(non_snake_case)]
fn annotateDiffOps(diffOps: &[DiffOp]) -> Vec<UnifiedDiffLine> {
    let mut lines = Vec::new();
    let mut oldLine = 1usize;
    let mut newLine = 1usize;

    for op in diffOps {
        match op {
            DiffOp::Equal(text) => {
                lines.push(UnifiedDiffLine::Context {
                    oldLine,
                    newLine,
                    text: text.clone(),
                });
                oldLine += 1;
                newLine += 1;
            }
            DiffOp::Delete(text) => {
                lines.push(UnifiedDiffLine::Delete {
                    oldLine,
                    text: text.clone(),
                });
                oldLine += 1;
            }
            DiffOp::Insert(text) => {
                lines.push(UnifiedDiffLine::Insert {
                    newLine,
                    text: text.clone(),
                });
                newLine += 1;
            }
        }
    }

    lines
}

#[allow(non_snake_case)]
fn buildUnifiedDiffHunks(lines: &[UnifiedDiffLine], context: usize) -> Vec<UnifiedDiffHunk> {
    let changeIndexes = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            if matches!(line, UnifiedDiffLine::Context { .. }) {
                None
            } else {
                Some(index)
            }
        })
        .collect::<Vec<_>>();

    let mut ranges = Vec::<(usize, usize)>::new();
    for changeIndex in changeIndexes {
        let start = changeIndex.saturating_sub(context);
        let end = (changeIndex + context).min(lines.len().saturating_sub(1));
        if let Some((_, previousEnd)) = ranges.last_mut() {
            if start <= *previousEnd + 1 {
                *previousEnd = (*previousEnd).max(end);
                continue;
            }
        }
        ranges.push((start, end));
    }

    ranges
        .into_iter()
        .map(|(start, end)| buildUnifiedDiffHunk(lines[start..=end].to_vec()))
        .collect()
}

#[allow(non_snake_case)]
fn buildUnifiedDiffHunk(lines: Vec<UnifiedDiffLine>) -> UnifiedDiffHunk {
    let oldStart = lines
        .iter()
        .find_map(|line| match line {
            UnifiedDiffLine::Context { oldLine, .. } | UnifiedDiffLine::Delete { oldLine, .. } => {
                Some(*oldLine)
            }
            UnifiedDiffLine::Insert { .. } => None,
        })
        .unwrap_or(1);
    let newStart = lines
        .iter()
        .find_map(|line| match line {
            UnifiedDiffLine::Context { newLine, .. } => Some(*newLine),
            UnifiedDiffLine::Insert { newLine, .. } => Some(*newLine),
            UnifiedDiffLine::Delete { .. } => None,
        })
        .unwrap_or(1);
    let oldCount = lines
        .iter()
        .filter(|line| {
            matches!(
                line,
                UnifiedDiffLine::Context { .. } | UnifiedDiffLine::Delete { .. }
            )
        })
        .count();
    let newCount = lines
        .iter()
        .filter(|line| {
            matches!(
                line,
                UnifiedDiffLine::Context { .. } | UnifiedDiffLine::Insert { .. }
            )
        })
        .count();

    UnifiedDiffHunk {
        oldStart,
        oldCount,
        newStart,
        newCount,
        lines,
    }
}

fn normalizeWhitespace(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn trimTrailingNewline(value: &str) -> String {
    value.trim_end_matches(['\n', '\r']).to_string()
}

fn tokenize_line_for_diff(line: &str) -> Vec<(&str, usize, usize)> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let mut end = start + ch.len_utf8();
        let is_word = ch.is_alphanumeric() || ch == '_';
        let is_space = ch.is_whitespace();
        while let Some(&(next_start, next_ch)) = chars.peek() {
            let next_is_word = next_ch.is_alphanumeric() || next_ch == '_';
            let next_is_space = next_ch.is_whitespace();
            if (is_word && next_is_word) || (is_space && next_is_space) {
                chars.next();
                end = next_start + next_ch.len_utf8();
            } else {
                break;
            }
        }
        tokens.push((&line[start..end], start, end));
    }
    tokens
}

/// Computes UTF-8 byte spans using the same linear-space diff as whole-file edits.
pub fn compute_intra_line_spans(
    old_line: &str,
    new_line: &str,
) -> Option<(Vec<(usize, usize)>, Vec<(usize, usize)>)> {
    if old_line == new_line {
        return None;
    }
    let old_tokens = tokenize_line_for_diff(old_line);
    let new_tokens = tokenize_line_for_diff(new_line);
    let max_len = old_tokens.len().max(new_tokens.len());
    if max_len == 0 {
        return None;
    }

    let old_text: Vec<_> = old_tokens.iter().map(|token| token.0).collect();
    let new_text: Vec<_> = new_tokens.iter().map(|token| token.0).collect();
    let operations = capture_diff_slices(Algorithm::Myers, &old_text, &new_text);
    let unchanged: usize = operations
        .iter()
        .filter(|op| op.tag() == DiffTag::Equal)
        .map(|op| op.old_range().len())
        .sum();
    if unchanged * 5 < max_len {
        return None;
    }

    let mut old_indices = Vec::new();
    let mut new_indices = Vec::new();
    for op in operations {
        if op.tag() != DiffTag::Equal {
            old_indices.extend(op.old_range());
            new_indices.extend(op.new_range());
        }
    }

    /// Coalesces adjacent changed tokens without splitting a UTF-8 character.
    fn merge_spans(tokens: &[(&str, usize, usize)], indices: &[usize]) -> Vec<(usize, usize)> {
        let mut spans = Vec::new();
        for &index in indices {
            let (_, start, end) = tokens[index];
            if let Some((_, last_end)) = spans.last_mut() {
                if start <= *last_end {
                    *last_end = (*last_end).max(end);
                    continue;
                }
            }
            spans.push((start, end));
        }
        spans
    }

    Some((
        merge_spans(&old_tokens, &old_indices),
        merge_spans(&new_tokens, &new_indices),
    ))
}

#[cfg(test)]
mod bounded_diff_tests {
    use super::*;

    /// Exercises a file size whose former 25,001 by 25,001 table required about five GiB.
    #[test]
    fn large_file_edit_keeps_an_exact_small_unified_hunk() {
        let old = (0..25000).map(|index| format!("line {index}\n")).collect::<String>();
        let new = old.replace("line 12500\n", "changed 中文\n");
        let diff = FileBindingService.generateUnifiedDiff(&old, &new);
        assert_eq!(diff, "@@ -12498,7 +12498,7 @@\n line 12497\n line 12498\n line 12499\n-line 12500\n+changed 中文\n line 12501\n line 12502\n line 12503");
        assert_eq!(FileBindingService.generateUnifiedDiff(&old, &old), "No changes detected (files are identical)");
    }

    /// Long state lines also avoid a token-count product allocation and preserve byte-accurate highlighting.
    #[test]
    fn large_single_line_edit_preserves_unicode_spans_and_similarity_threshold() {
        let prefix = "field, ".repeat(10000);
        let old = format!("{prefix}旧值, tail");
        let new = format!("{prefix}新值, tail");
        let spans = compute_intra_line_spans(&old, &new).unwrap();
        assert_eq!(spans, (vec![(prefix.len(), prefix.len() + "旧值".len())], vec![(prefix.len(), prefix.len() + "新值".len())]));
        assert_eq!(compute_intra_line_spans("aaa", "bbb"), None);
        assert_eq!(compute_intra_line_spans("same", "same"), None);
    }

    /// Reconstructs both inputs across repeated lines, empty inputs, insertions and deletions.
    /// A unique fragment inside a long line edits that line while preserving its
    /// surrounding text, even though the window n-gram score cannot clear the threshold.
    #[test]
    fn unique_fragment_in_long_line_is_applied() {
        let svc = FileBindingService;
        let long = format!("{}{}", "prefix field, ".repeat(200), "TAIL");
        let ops = vec![StructuredEditOperation {
            action: StructuredEditAction::REPLACE,
            oldContent: "field, TAIL".to_string(),
            newContent: "field, CHANGED".to_string(),
        }];
        let (result, message) = svc.processFileBindingOperations(&long, &ops);
        assert!(!message.starts_with("Error"), "{message}");
        assert!(result.starts_with("prefix field, "), "{result}");
        assert!(result.ends_with("CHANGED"), "{result}");
        assert!(!result.contains("TAIL"), "{result}");
    }

    /// A fragment that appears in more than one place is still rejected, and the
    /// failure message now carries the best similarity score.
    #[test]
    fn ambiguous_fragment_is_rejected_with_score_diagnostic() {
        let svc = FileBindingService;
        let content = "alpha beta gamma\nalpha beta gamma\n";
        let ops = vec![StructuredEditOperation {
            action: StructuredEditAction::REPLACE,
            oldContent: "lpha be".to_string(),
            newContent: "X".to_string(),
        }];
        let (_, message) = svc.processFileBindingOperations(content, &ops);
        assert!(message.starts_with("Error"), "{message}");
        assert!(message.contains("similarity"), "{message}");
    }

    /// Multi-line OLD blocks keep working through the regular window matcher.
    #[test]
    fn multiline_block_still_matches_normally() {
        let svc = FileBindingService;
        let content = "line one\nline two\nline three\n";
        let ops = vec![StructuredEditOperation {
            action: StructuredEditAction::REPLACE,
            oldContent: "line one\nline two".to_string(),
            newContent: "replaced".to_string(),
        }];
        let (result, message) = svc.processFileBindingOperations(content, &ops);
        assert!(!message.starts_with("Error"), "{message}");
        assert_eq!(result, "replaced\nline three");
    }

    #[test]
    fn line_edits_reconstruct_both_sources_with_duplicate_lines_and_line_endings() {
        for old in ["", "a", "a\nb\na\n", "a\r\nb\r\n", "\n\n"] {
            for new in ["", "b", "b\na\nb\n", "a\r\nc\r\n", "\n"] {
                let original = kotlinLines(old);
                let modified = kotlinLines(new);
                let ops = buildDiffOps(&original, &modified);
                let before: Vec<_> = ops.iter().filter_map(|op| match op {
                    DiffOp::Equal(text) | DiffOp::Delete(text) => Some(text.clone()),
                    DiffOp::Insert(_) => None,
                }).collect();
                let after: Vec<_> = ops.iter().filter_map(|op| match op {
                    DiffOp::Equal(text) | DiffOp::Insert(text) => Some(text.clone()),
                    DiffOp::Delete(_) => None,
                }).collect();
                assert_eq!(before, original);
                assert_eq!(after, modified);
            }
        }
    }
}
