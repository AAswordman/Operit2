use std::collections::BTreeSet;

const MATCH_THRESHOLD: f64 = 0.90;

pub struct FileBindingService;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructuredEditAction {
    REPLACE,
    DELETE,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

impl FileBindingService {
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
            let diffString = self.generateDiff(&originalContent.replace("\r\n", "\n"), &resultString);
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

    pub fn generateUnifiedDiff(&self, original: &str, modified: &str) -> String {
        let originalLines: Vec<&str> = if original.is_empty() {
            Vec::new()
        } else {
            original.lines().collect()
        };
        let modifiedLines: Vec<&str> = if modified.is_empty() {
            Vec::new()
        } else {
            modified.lines().collect()
        };

        if originalLines == modifiedLines {
            return "No changes detected (files are identical)".to_string();
        }

        let mut additions = 0;
        let mut deletions = 0;
        let max_len = originalLines.len().max(modifiedLines.len());
        let mut resultLines = Vec::new();
        resultLines.push(format!(
            "@@ -1,{} +1,{} @@",
            originalLines.len(),
            modifiedLines.len()
        ));
        for index in 0..max_len {
            match (originalLines.get(index), modifiedLines.get(index)) {
                (Some(left), Some(right)) if left == right => {
                    resultLines.push(format!(" {:<4}|{}", index + 1, left));
                }
                (Some(left), Some(right)) => {
                    deletions += 1;
                    additions += 1;
                    resultLines.push(format!("-{:<4}|{}", index + 1, left));
                    resultLines.push(format!("+{:<4}|{}", index + 1, right));
                }
                (Some(left), None) => {
                    deletions += 1;
                    resultLines.push(format!("-{:<4}|{}", index + 1, left));
                }
                (None, Some(right)) => {
                    additions += 1;
                    resultLines.push(format!("+{:<4}|{}", index + 1, right));
                }
                (None, None) => {}
            }
        }

        format!(
            "Changes: +{} -{} lines\n{}",
            additions,
            deletions,
            resultLines.join("\n")
        )
    }

    fn applyFuzzyPatch(&self, originalContent: &str, aiPatchCode: &str) -> (bool, String) {
        let operations = self.parseEditOperations(aiPatchCode);
        if operations.is_empty() {
            return (false, "No valid edit operations found in the patch code.".to_string());
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
            let (start, end) = self.findBestMatchRange(&originalLines, &op.oldContent);
            if start < 0 {
                return (
                    false,
                    "Could not find a match for an OLD block. The file may have changed too much."
                        .to_string(),
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
            let boundaryPreservingLines = self.tryApplyBoundaryPreservingEdit(&originalSegment, &op);
            for index in (start..=end).rev() {
                originalLines.remove(index);
            }
            if let Some(lines) = boundaryPreservingLines {
                for (offset, line) in lines.into_iter().enumerate() {
                    originalLines.insert(start + offset, line);
                }
            } else if op.action == EditAction::REPLACE {
                let newLines = op.newContent.lines().map(str::to_string).collect::<Vec<_>>();
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
                    for line in newLines.iter().skip(1).take(newLines.len().saturating_sub(2)) {
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
                while i < lines.len() && !lines[i].trim().starts_with(&format!("[END-{actionStr}]")) {
                    let currentLine = lines[i];
                    let trimmedLine = currentLine.trim();
                    if trimmedLine.starts_with("[OLD]") {
                        inBlock = Some("OLD");
                        let inline = currentLine.split_once("[OLD]").map(|(_, v)| v).unwrap_or("");
                        if !inline.is_empty() {
                            oldContent.push_str(inline);
                            oldContent.push('\n');
                        }
                    } else if trimmedLine.starts_with("[NEW]") {
                        inBlock = Some("NEW");
                        let inline = currentLine.split_once("[NEW]").map(|(_, v)| v).unwrap_or("");
                        if !inline.is_empty() {
                            newContent.push_str(inline);
                            newContent.push('\n');
                        }
                    } else if trimmedLine.starts_with("[/OLD]") || trimmedLine.starts_with("[/NEW]") {
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

    fn findBestMatchRange(&self, originalLines: &[String], oldContent: &str) -> (isize, isize) {
        let oldContentLines = oldContent.lines().collect::<Vec<_>>();
        let numOldLines = oldContentLines.len();
        if numOldLines == 0 || originalLines.is_empty() {
            return (-1, -1);
        }

        let normalizedOldContent = normalizeWhitespace(oldContent);
        let normalizedOldLength = normalizedOldContent.len();
        let baseNgrams = self.buildNgrams(&normalizedOldContent, 3);
        if baseNgrams.is_empty() {
            return (-1, -1);
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
                                        && (best.startLine == -1 || i as isize <= best.startLine))))));
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
            (best.startLine, best.endLine)
        } else {
            (-1, -1)
        }
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

fn normalizeWhitespace(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn trimTrailingNewline(value: &str) -> String {
    value.trim_end_matches(['\n', '\r']).to_string()
}
