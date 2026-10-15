use super::super::{helpers::wrap_approx_lines, i18n::TuiText};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Node {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub props: Map<String, Value>,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub slots: std::collections::BTreeMap<String, Vec<Node>>,
}

#[derive(Clone, Debug)]
pub(super) struct Hit {
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub action: String,
    pub input: Option<String>,
}

#[derive(Default)]
pub(super) struct Surface {
    pub lines: Vec<Line<'static>>,
    pub hits: Vec<Hit>,
}

/// Reads a serialized callback reference produced by the Compose runtime.
pub(super) fn action(value: Option<&Value>) -> Result<Option<String>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(id)) if !id.is_empty() => {
            Ok(Some(id.strip_prefix("__action:").unwrap_or(id).to_string()))
        }
        Some(Value::Object(object)) => object
            .get("__actionId")
            .and_then(Value::as_str)
            .map(|id| Some(id.to_string()))
            .ok_or("Invalid Compose callback reference".into()),
        _ => Err("Invalid Compose callback reference".into()),
    }
}

/// Reads an optional text property according to the DSL's empty-text default.
fn text(node: &Node, name: &str) -> String {
    node.props
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// Appends a child surface and translates its click coordinates.
fn append(target: &mut Surface, mut child: Surface) {
    let offset = target.lines.len();
    for hit in &mut child.hits {
        hit.row += offset;
    }
    target.lines.extend(child.lines);
    target.hits.extend(child.hits);
}

/// Builds terminal lines for one plugin node without executing any plugin callback.
pub(super) fn render(node: &Node, width: usize, locale: TuiText) -> Result<Surface, String> {
    if width < 12 {
        return render_node(node, width, locale);
    }
    let mut surface = render_node(node, width - 4, locale)?;
    let border = Style::default().fg(Color::DarkGray);
    for line in &mut surface.lines {
        let padding = (width - 4).saturating_sub(line.width());
        line.spans.insert(0, Span::styled("│ ", border));
        line.spans.push(Span::raw(" ".repeat(padding)));
        line.spans.push(Span::styled(" │", border));
    }
    surface.lines.insert(
        0,
        Line::styled(format!("╭{}╮", "─".repeat(width - 2)), border),
    );
    surface
        .lines
        .push(Line::styled(format!("╰{}╯", "─".repeat(width - 2)), border));
    for hit in &mut surface.hits {
        hit.row += 1;
        hit.start += 2;
        hit.end += 2;
    }
    Ok(surface)
}

/// Lays out nested DSL widgets with terminal spacing and local semantic styles.
fn render_node(node: &Node, width: usize, locale: TuiText) -> Result<Surface, String> {
    let width = width.max(1);
    if !node.slots.is_empty() {
        return Err(format!(
            "Named slots are not supported for terminal node {}",
            node.kind
        ));
    }
    let enabled = node.props.get("enabled").and_then(Value::as_bool) != Some(false);
    let mut surface = Surface::default();
    match node.kind.as_str() {
        "Column" | "LazyColumn" | "Card" | "ElevatedCard" | "OutlinedCard" | "Box" | "Surface" => {
            for (index, child) in node.children.iter().enumerate() {
                if index > 0
                    && node
                        .props
                        .get("spacing")
                        .and_then(Value::as_f64)
                        .is_some_and(|gap| gap >= 8.0)
                {
                    surface.lines.push(Line::default());
                }
                append(&mut surface, render_node(child, width, locale)?);
            }
        }
        "Row" | "LazyRow" => {
            let count = node.children.len();
            if count > 0 {
                if width < count * 4 {
                    for child in &node.children {
                        append(&mut surface, render_node(child, width, locale)?);
                    }
                } else {
                    let gap = usize::from(width > count * 2);
                    let usable = width.saturating_sub(gap * count.saturating_sub(1));
                    let icons = node
                        .children
                        .iter()
                        .filter(|child| child.kind == "Icon")
                        .count();
                    let child_width =
                        (usable.saturating_sub(icons * 2) / (count - icons).max(1)).max(1);
                    let mut offset: usize = 0;
                    for child in &node.children {
                        let child_width = if child.kind == "Icon" { 2 } else { child_width };
                        let child = render_node(child, child_width, locale)?;
                        for (row, line) in child.lines.into_iter().enumerate() {
                            while surface.lines.len() <= row {
                                surface.lines.push(Line::default());
                            }
                            let target = &mut surface.lines[row];
                            target
                                .spans
                                .push(Span::raw(" ".repeat(offset.saturating_sub(target.width()))));
                            target.spans.extend(line.spans);
                        }
                        surface.hits.extend(child.hits.into_iter().map(|mut hit| {
                            hit.start += offset;
                            hit.end += offset;
                            hit
                        }));
                        offset += child_width + gap;
                    }
                }
            }
        }
        "Text" | "BasicText" => {
            surface.lines = wrap_approx_lines(&text(node, "text"), width)
                .into_iter()
                .map(Line::from)
                .collect();
        }
        "Markdown" => {
            surface.lines =
                super::super::markdown::render_markdown_lines(&text(node, "text"), width, locale);
            // The transcript owns the final terminal width; Markdown must not bleed into adjacent nodes.
            surface.lines = surface
                .lines
                .into_iter()
                .flat_map(|line| {
                    let value = line
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>();
                    wrap_approx_lines(&value, width)
                        .into_iter()
                        .map(Line::from)
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        "Button" | "TextButton" | "OutlinedButton" | "FilledTonalButton" | "ElevatedButton" => {
            let label = text(node, "text");
            if !label.is_empty() {
                surface.lines = wrap_approx_lines(&format!("[ {label} ]"), width)
                    .into_iter()
                    .map(Line::from)
                    .collect();
            }
            for child in &node.children {
                append(
                    &mut surface,
                    render_node(child, width.saturating_sub(4).max(1), locale)?,
                );
            }
            if label.is_empty() && !surface.lines.is_empty() {
                let marker = if node.kind == "FilledTonalButton" {
                    "● "
                } else {
                    "○ "
                };
                for (index, line) in surface.lines.iter_mut().enumerate() {
                    line.spans
                        .insert(0, Span::raw(if index == 0 { marker } else { "  " }));
                }
                for hit in &mut surface.hits {
                    hit.start += 2;
                    hit.end += 2;
                }
            }
            if node.kind == "FilledTonalButton" {
                for line in &mut surface.lines {
                    for span in &mut line.spans {
                        span.style = span.style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
                    }
                }
            }
        }
        "IconButton" => {
            let icon = text(node, "icon");
            let label = match icon.as_str() {
                "chevronLeft" => "‹",
                "chevronRight" => "›",
                "arrowBack" => "←",
                "arrowForward" => "→",
                _ => icon.as_str(),
            };
            surface.lines = wrap_approx_lines(&format!("[ {label} ]"), width)
                .into_iter()
                .map(Line::from)
                .collect();
        }
        "TextField" | "OutlinedTextField" => {
            let value = text(node, "value");
            surface.lines =
                wrap_approx_lines(&format!("{}: [{}]", text(node, "label"), value), width)
                    .into_iter()
                    .map(Line::from)
                    .collect();
            if enabled {
                if let Some(id) = action(node.props.get("onValueChange"))? {
                    for (row, line) in surface.lines.iter().enumerate() {
                        surface.hits.push(Hit {
                            row,
                            start: 0,
                            end: line.width().min(width),
                            action: id.clone(),
                            input: Some(value.clone()),
                        });
                    }
                }
            }
        }
        "Icon" => {
            surface.lines.push(Line::from("◆"));
        }
        "Spacer" => {
            surface.lines.push(Line::from(""));
        }
        "Divider" | "HorizontalDivider" => {
            surface.lines.push(Line::from("─".repeat(width)));
        }
        "CircularProgressIndicator" | "LinearProgressIndicator" => {
            surface.lines.push(Line::from("…"));
        }
        kind => return Err(format!("Unsupported terminal Compose node: {kind}")),
    }
    if !enabled {
        surface.hits.clear();
    }
    if enabled {
        let mut click = action(node.props.get("onClick"))?;
        if let Some(ops) = node
            .props
            .get("modifier")
            .and_then(|value| value.get("__modifierOps"))
            .and_then(Value::as_array)
        {
            for op in ops {
                if op.get("name").and_then(Value::as_str) == Some("clickable") {
                    click = action(
                        op.get("args")
                            .and_then(Value::as_array)
                            .and_then(|args| args.first()),
                    )?;
                }
            }
        }
        if let Some(id) = click {
            for (row, line) in surface.lines.iter().enumerate() {
                if !surface.hits.iter().any(|hit| hit.row == row) {
                    surface.hits.push(Hit {
                        row,
                        start: 0,
                        end: line.width().min(width),
                        action: id.clone(),
                        input: None,
                    });
                }
            }
        }
    }
    let is_control = matches!(
        node.kind.as_str(),
        "Button"
            | "TextButton"
            | "OutlinedButton"
            | "FilledTonalButton"
            | "ElevatedButton"
            | "IconButton"
            | "TextField"
            | "OutlinedTextField"
    );
    let color = match (enabled, text(node, "color").as_str()) {
        (false, _) => Some(Color::DarkGray),
        (true, "onSurfaceVariant") => Some(Color::DarkGray),
        (true, "primary") => Some(Color::Cyan),
        _ if is_control => Some(Color::Cyan),
        _ => None,
    };
    for line in &mut surface.lines {
        for span in &mut line.spans {
            if let Some(color) = color {
                span.style = span.style.fg(color);
            }
            if matches!(text(node, "fontWeight").as_str(), "bold" | "semibold") {
                span.style = span.style.add_modifier(Modifier::BOLD);
            }
        }
    }
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::super::super::i18n::TuiLanguage;
    use super::*;

    /// Exposes arrow button labels and hit targets while disabling unavailable navigation.
    #[test]
    fn icon_navigation_is_visible_and_clickable() {
        let node: Node = serde_json::from_value(serde_json::json!({"type":"Row","children":[
            {"type":"IconButton","props":{"icon":"chevronLeft","enabled":false,"onClick":{"__actionId":"previous"}}},
            {"type":"Text","props":{"text":"1/3"}},
            {"type":"IconButton","props":{"icon":"chevronRight","onClick":{"__actionId":"next"}}}
        ]})).unwrap();
        let surface = render(&node, 40, TuiLanguage::English.text()).unwrap();
        let content = surface
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<String>();
        assert!(content.contains("[ ‹ ]"));
        assert!(content.contains("[ › ]"));
        assert_eq!(surface.hits.len(), 1);
        assert_eq!(surface.hits[0].action, "next");
        assert!(surface.hits[0].end > surface.hits[0].start);
    }

    /// Ensures adjacent buttons have independent hit ranges and disabled controls have none.
    #[test]
    fn click_regions_follow_layout_and_enabled_state() {
        let node: Node = serde_json::from_value(serde_json::json!({"type":"Row","children":[
            {"type":"Button","props":{"text":"Yes","onClick":{"__actionId":"yes"}}},
            {"type":"Button","props":{"text":"No","enabled":false,"onClick":{"__actionId":"no"}}}
        ]}))
        .unwrap();
        let surface = render(&node, 30, TuiLanguage::English.text()).unwrap();
        assert_eq!(surface.hits.len(), 1);
        assert_eq!(surface.hits[0].action, "yes");
        assert!(surface.hits[0].end < 15);
    }

    /// Keeps a disabled parent from exposing nested controls and rejects unknown widgets.
    #[test]
    fn disabled_parent_blocks_children_and_unknown_nodes_are_errors() {
        let node: Node = serde_json::from_value(
            serde_json::json!({"type":"Card", "props":{"enabled":false}, "children":[
                {"type":"Button","props":{"text":"Run","onClick":{"__actionId":"run"}}}
            ]}),
        )
        .unwrap();
        assert!(render(&node, 10, TuiLanguage::English.text())
            .unwrap()
            .hits
            .is_empty());
        let unsupported: Node =
            serde_json::from_value(serde_json::json!({"type":"WebView"})).unwrap();
        assert!(render(&unsupported, 10, TuiLanguage::English.text()).is_err());
    }

    /// Keeps narrow layouts and wide Unicode labels inside their available columns.
    #[test]
    fn narrow_rows_keep_actions_inside_their_lines() {
        let node: Node = serde_json::from_value(serde_json::json!({"type":"Row","children":[
            {"type":"Button","props":{"text":"确认","onClick":{"__actionId":"yes"}}},
            {"type":"Button","props":{"text":"取消","onClick":{"__actionId":"no"}}}
        ]}))
        .unwrap();
        let surface = render(&node, 6, TuiLanguage::Chinese.text()).unwrap();
        assert!(surface.lines.iter().all(|line| line.width() <= 6));
        assert!(surface.hits.iter().any(|hit| hit.action == "yes"));
        assert!(surface.hits.iter().any(|hit| hit.action == "no"));
        assert!(surface
            .hits
            .iter()
            .all(|hit| hit.end <= 6 && hit.row < surface.lines.len()));
    }
}
