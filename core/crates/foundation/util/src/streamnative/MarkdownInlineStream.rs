//! Incremental inline syntax. Only undecided delimiters, a link, or an autolink
//! are buffered; previously emitted text is never scanned again.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineKind {
    Text,
    Bold,
    Italic,
    Strike,
    Code,
    Latex,
    Link,
    FootnoteReference,
}

impl InlineKind {
    pub fn processor_type(self) -> super::NativeMarkdownSplitter::MarkdownProcessorType {
        use super::NativeMarkdownSplitter::MarkdownProcessorType as Type;
        match self {
            Self::Text => Type::PlainText,
            Self::Bold => Type::Bold,
            Self::Italic => Type::Italic,
            Self::Strike => Type::Strikethrough,
            Self::Code => Type::InlineCode,
            Self::Latex => Type::InlineLatex,
            Self::Link => Type::Link,
            Self::FootnoteReference => Type::FootnoteReference,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InlineToken {
    Open(InlineKind, Option<String>),
    Close,
    Text(InlineKind, String),
}

#[derive(Debug)]
enum Mode {
    Text,
    Escape,
    Delimiter {
        ch: char,
        count: usize,
        before: Option<char>,
    },
    Code {
        ticks: usize,
        pending: usize,
    },
    Latex {
        paren: bool,
        escape: bool,
    },
    Label {
        raw: String,
        depth: usize,
        escaped: bool,
    },
    AfterLabel(String),
    Destination {
        label: String,
        url: String,
        depth: usize,
        escaped: bool,
    },
    UrlPrefix(String),
    Url(String),
}

pub struct MarkdownInlineStream {
    mode: Mode,
    styles: Vec<(char, usize, InlineKind)>,
    previous: Option<char>,
    depth: usize,
    #[cfg(test)]
    pub processed_chars: usize,
}

impl Default for MarkdownInlineStream {
    fn default() -> Self {
        Self::new(0)
    }
}

impl MarkdownInlineStream {
    fn new(depth: usize) -> Self {
        Self {
            mode: Mode::Text,
            styles: Vec::new(),
            previous: None,
            depth,
            #[cfg(test)]
            processed_chars: 0,
        }
    }

    pub fn push(&mut self, content: &str) -> Vec<InlineToken> {
        let mut out = Vec::new();
        for ch in content.chars() {
            #[cfg(test)]
            {
                self.processed_chars += 1;
            }
            self.step(ch, &mut out);
            self.previous = Some(ch);
        }
        out
    }

    fn text(out: &mut Vec<InlineToken>, kind: InlineKind, ch: char) {
        if let Some(InlineToken::Text(last_kind, text)) = out.last_mut() {
            if *last_kind == kind {
                text.push(ch);
                return;
            }
        }
        out.push(InlineToken::Text(kind, ch.to_string()));
    }

    fn literal(out: &mut Vec<InlineToken>, text: &str) {
        for ch in text.chars() {
            Self::text(out, InlineKind::Text, ch);
        }
    }

    fn step(&mut self, ch: char, out: &mut Vec<InlineToken>) {
        let mode = std::mem::replace(&mut self.mode, Mode::Text);
        match mode {
            Mode::Text => self.normal(ch, out),
            Mode::Escape => {
                if ch == '(' {
                    self.mode = Mode::Latex {
                        paren: true,
                        escape: false,
                    };
                } else if ch.is_ascii_punctuation() {
                    Self::text(out, InlineKind::Text, ch);
                } else {
                    Self::text(out, InlineKind::Text, '\\');
                    self.normal(ch, out);
                }
            }
            Mode::Delimiter {
                ch: marker,
                count,
                before,
            } => {
                if ch == marker {
                    self.mode = Mode::Delimiter {
                        ch: marker,
                        count: count + 1,
                        before,
                    };
                } else if marker == '`' {
                    self.mode = Mode::Code {
                        ticks: count,
                        pending: 0,
                    };
                    self.step(ch, out);
                } else {
                    self.delimiter(marker, count, before, Some(ch), out);
                    self.normal(ch, out);
                }
            }
            Mode::Code { ticks, pending } => {
                if ch == '`' {
                    self.mode = Mode::Code {
                        ticks,
                        pending: pending + 1,
                    };
                } else if pending == ticks {
                    self.normal(ch, out);
                } else {
                    for _ in 0..pending {
                        Self::text(out, InlineKind::Code, '`');
                    }
                    Self::text(out, InlineKind::Code, ch);
                    self.mode = Mode::Code { ticks, pending: 0 };
                }
            }
            Mode::Latex { paren, escape } => {
                if paren && escape && ch == ')' {
                    return;
                }
                if !paren && ch == '$' {
                    return;
                }
                if escape {
                    Self::text(out, InlineKind::Latex, '\\');
                }
                if paren && ch == '\\' {
                    self.mode = Mode::Latex {
                        paren,
                        escape: true,
                    };
                } else {
                    Self::text(out, InlineKind::Latex, ch);
                    self.mode = Mode::Latex {
                        paren,
                        escape: false,
                    };
                }
            }
            Mode::Label {
                mut raw,
                mut depth,
                escaped,
            } => {
                if !escaped && ch == ']' && depth == 0 {
                    if let Some(label) = raw.strip_prefix('^').filter(|label| {
                        !label.is_empty()
                            && !label
                                .chars()
                                .any(|ch| ch.is_whitespace() || matches!(ch, '[' | ']' | '\\'))
                    }) {
                        out.push(InlineToken::Open(InlineKind::FootnoteReference, None));
                        Self::literal(out, label);
                        out.push(InlineToken::Close);
                    } else {
                        self.mode = Mode::AfterLabel(raw);
                    }
                } else {
                    if !escaped && ch == '[' {
                        depth += 1;
                    }
                    if !escaped && ch == ']' {
                        depth -= 1;
                    }
                    raw.push(ch);
                    self.mode = Mode::Label {
                        raw,
                        depth,
                        escaped: !escaped && ch == '\\',
                    };
                }
            }
            Mode::AfterLabel(label) => {
                if ch == '(' {
                    self.mode = Mode::Destination {
                        label,
                        url: String::new(),
                        depth: 0,
                        escaped: false,
                    };
                } else {
                    Self::literal(out, "[");
                    Self::literal(out, &label);
                    Self::literal(out, "]");
                    self.normal(ch, out);
                }
            }
            Mode::Destination {
                label,
                mut url,
                mut depth,
                escaped,
            } => {
                if !escaped && ch == ')' && depth == 0 {
                    let destination = link_destination(&url);
                    out.push(InlineToken::Open(InlineKind::Link, Some(destination)));
                    let mut label_parser = Self::new(self.depth + 1);
                    out.extend(label_parser.push(&label));
                    out.extend(label_parser.finish());
                    out.push(InlineToken::Close);
                } else {
                    if !escaped && ch == '(' {
                        depth += 1;
                    }
                    if !escaped && ch == ')' {
                        depth = depth.saturating_sub(1);
                    }
                    url.push(ch);
                    self.mode = Mode::Destination {
                        label,
                        url,
                        depth,
                        escaped: !escaped && ch == '\\',
                    };
                }
            }
            Mode::UrlPrefix(mut prefix) => {
                prefix.push(ch);
                let lower = prefix.to_ascii_lowercase();
                if lower == "http://" || lower == "https://" {
                    self.mode = Mode::Url(prefix);
                } else if "http://".starts_with(&lower) || "https://".starts_with(&lower) {
                    self.mode = Mode::UrlPrefix(prefix);
                } else {
                    let last = prefix.pop().unwrap();
                    Self::literal(out, &prefix);
                    self.normal(last, out);
                }
            }
            Mode::Url(mut url) => {
                if ch.is_whitespace() || matches!(ch, '<' | '>' | '[' | ']' | '*' | '`' | '~') {
                    Self::autolink(out, url);
                    self.normal(ch, out);
                } else {
                    url.push(ch);
                    self.mode = Mode::Url(url);
                }
            }
        }
    }

    fn normal(&mut self, ch: char, out: &mut Vec<InlineToken>) {
        match ch {
            '\\' => self.mode = Mode::Escape,
            '*' | '_' | '~' | '`' => {
                self.mode = Mode::Delimiter {
                    ch,
                    count: 1,
                    before: self.previous,
                }
            }
            '$' => {
                self.mode = Mode::Latex {
                    paren: false,
                    escape: false,
                }
            }
            '[' if self.depth == 0 => {
                self.mode = Mode::Label {
                    raw: String::new(),
                    depth: 0,
                    escaped: false,
                }
            }
            'h' | 'H'
                if self.depth == 0 && self.previous.is_none_or(|ch| !ch.is_alphanumeric()) =>
            {
                self.mode = Mode::UrlPrefix(ch.to_string())
            }
            _ => Self::text(out, InlineKind::Text, ch),
        }
    }

    fn delimiter(
        &mut self,
        ch: char,
        mut count: usize,
        before: Option<char>,
        after: Option<char>,
        out: &mut Vec<InlineToken>,
    ) {
        let can_close = before.is_some_and(|ch| !ch.is_whitespace());
        let can_open = after.is_some_and(|ch| !ch.is_whitespace());
        if ch == '_'
            && before.is_some_and(char::is_alphanumeric)
            && after.is_some_and(char::is_alphanumeric)
        {
            for _ in 0..count {
                Self::text(out, InlineKind::Text, ch);
            }
            return;
        }
        if can_close {
            while let Some(&(marker, width, _)) = self.styles.last() {
                if marker != ch || width > count || (width == 1 && count == 2) {
                    break;
                }
                self.styles.pop();
                out.push(InlineToken::Close);
                count -= width;
            }
        }
        if can_open && self.styles.len() < 12 {
            let widths: &[usize] = match (ch, count) {
                ('*' | '_', 1) => &[1],
                ('*' | '_', 2) => &[2],
                ('*' | '_', 3) => &[2, 1],
                ('~', 2) => &[2],
                _ => &[],
            };
            if !widths.is_empty() && self.styles.len() + widths.len() <= 12 {
                for &width in widths {
                    let kind = if ch == '~' {
                        InlineKind::Strike
                    } else if width == 2 {
                        InlineKind::Bold
                    } else {
                        InlineKind::Italic
                    };
                    self.styles.push((ch, width, kind));
                    out.push(InlineToken::Open(kind, None));
                }
                return;
            }
        }
        for _ in 0..count {
            Self::text(out, InlineKind::Text, ch);
        }
    }

    fn autolink(out: &mut Vec<InlineToken>, mut url: String) {
        let mut trailing = Vec::new();
        let mut balance = url.chars().fold(0i32, |balance, ch| {
            balance
                + if ch == '(' {
                    1
                } else if ch == ')' {
                    -1
                } else {
                    0
                }
        });
        while let Some(ch) = url.chars().next_back() {
            if ".,;:!?，。、；：！？".contains(ch) || (ch == ')' && balance < 0) {
                url.pop();
                trailing.push(ch);
                if ch == ')' {
                    balance += 1;
                }
            } else {
                break;
            }
        }
        out.push(InlineToken::Open(InlineKind::Link, Some(url.clone())));
        Self::literal(out, &url);
        out.push(InlineToken::Close);
        for ch in trailing.into_iter().rev() {
            Self::text(out, InlineKind::Text, ch);
        }
    }

    /// Flushes only undecided input. Called at a block boundary or end of stream.
    pub fn finish(&mut self) -> Vec<InlineToken> {
        let mut out = Vec::new();
        match std::mem::replace(&mut self.mode, Mode::Text) {
            Mode::Text => {}
            Mode::Escape => Self::literal(&mut out, "\\"),
            Mode::Delimiter { ch, count, before } => {
                if ch == '`' {
                    for _ in 0..count {
                        Self::text(&mut out, InlineKind::Text, ch);
                    }
                } else {
                    self.delimiter(ch, count, before, None, &mut out);
                }
            }
            Mode::Code { ticks, pending } => {
                if pending != ticks {
                    for _ in 0..pending {
                        Self::text(&mut out, InlineKind::Code, '`');
                    }
                }
            }
            Mode::Latex { escape: true, .. } => Self::text(&mut out, InlineKind::Latex, '\\'),
            Mode::Latex { .. } => {}
            Mode::Label { raw, .. } => {
                Self::literal(&mut out, "[");
                Self::literal(&mut out, &raw);
            }
            Mode::AfterLabel(label) => {
                Self::literal(&mut out, "[");
                Self::literal(&mut out, &label);
                Self::literal(&mut out, "]");
            }
            Mode::Destination { label, url, .. } => {
                Self::literal(&mut out, "[");
                Self::literal(&mut out, &label);
                Self::literal(&mut out, "](");
                Self::literal(&mut out, &url);
            }
            Mode::UrlPrefix(prefix) => Self::literal(&mut out, &prefix),
            Mode::Url(url) => Self::autolink(&mut out, url),
        }
        for _ in self.styles.drain(..).rev() {
            out.push(InlineToken::Close);
        }
        out
    }
}

fn link_destination(raw: &str) -> String {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix('<') {
        return rest
            .split_once('>')
            .map_or(rest, |(url, _)| url)
            .to_string();
    }
    let mut end = raw.len();
    let mut escaped = false;
    for (index, ch) in raw.char_indices() {
        if !escaped && ch.is_whitespace() {
            end = index;
            break;
        }
        escaped = !escaped && ch == '\\';
    }
    raw[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_syntax_retains_semantic_styles_and_literal_code() {
        let nodes = super::super::NativeMarkdownSplitter::NativeMarkdownSplitter::native_markdown_split_by_inline(
            "**粗体 *斜体* `**代码**` [链接](https://example.com/a_(b))**");
        assert_eq!(nodes.len(), 1);
        let bold = &nodes[0];
        assert_eq!(bold.r#type, InlineKind::Bold.processor_type());
        assert_eq!(bold.content, "粗体 斜体 **代码** 链接");
        assert!(bold
            .children
            .iter()
            .any(|child| child.r#type == InlineKind::Italic.processor_type()));
        let code = bold
            .children
            .iter()
            .find(|child| child.r#type == InlineKind::Code.processor_type())
            .unwrap();
        assert_eq!(code.content, "**代码**");
        let link = bold
            .children
            .iter()
            .find(|child| child.r#type == InlineKind::Link.processor_type())
            .unwrap();
        assert_eq!(link.link_url.as_deref(), Some("https://example.com/a_(b)"));
    }

    #[test]
    fn underscores_inside_words_and_escaped_delimiters_remain_literal() {
        use super::super::NativeMarkdownSplitter::NativeMarkdownSplitter as Parser;
        let nodes =
            Parser::native_markdown_split_by_inline(r"\*literal\* file_name a_b_c [bad](url");
        assert_eq!(
            nodes
                .iter()
                .map(|node| node.content.as_str())
                .collect::<String>(),
            "*literal* file_name a_b_c [bad](url"
        );
        assert!(nodes
            .iter()
            .all(|node| node.r#type == InlineKind::Text.processor_type()));
        let nodes = Parser::native_markdown_split_by_inline("_foo_bar_");
        assert_eq!(nodes[0].r#type, InlineKind::Italic.processor_type());
        assert_eq!(nodes[0].content, "foo_bar");
    }

    #[test]
    fn strong_emphasis_can_be_nested_inside_italic_without_spaces() {
        let nodes = super::super::NativeMarkdownSplitter::NativeMarkdownSplitter::native_markdown_split_by_inline("*a**b**c*");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].r#type, InlineKind::Italic.processor_type());
        assert_eq!(nodes[0].content, "abc");
        assert_eq!(
            nodes[0].children[1].r#type,
            InlineKind::Bold.processor_type()
        );
        assert_eq!(nodes[0].children[1].content, "b");
    }

    #[test]
    fn streaming_work_is_linear_in_new_input_and_does_not_retain_plain_history() {
        let chunk = "normal text **bold** and `code` ";
        for chunks in [100, 1000, 10000] {
            let mut parser = MarkdownInlineStream::default();
            for _ in 0..chunks {
                let _ = parser.push(chunk);
            }
            let _ = parser.finish();
            assert_eq!(parser.processed_chars, chunks * chunk.chars().count());
            assert!(matches!(parser.mode, Mode::Text));
            assert!(parser.styles.is_empty());
        }
    }
}
