//! Regenerate Flutter's real-Core event fixtures with:
//! cargo run -q -p operit-util --example markdown_render_fixture
use operit_util::MarkdownRenderStream::MarkdownRenderEventStream;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
fn main() {
    let sources = [
        "**粗体 *斜体* `代码` [链接](https://example.com/a_(b)) https://example.org**",
        "# **标题**\n- **粗体**\n+ - 保留正文符号\n  * __缩进项__\n3) 第三项\n---\n***粗斜***",
        "> **引用**\n> - **列表**\n> > 嵌套引用\n\n结束",
        "**标题** [链接](https://example.com)\n\n```dart\nfinal value = 1;\n```",
        "| 名称 | 数量 |\n| --- | ---: |\n| 苹果 | **2** |",
        "名称",
        "数量",
        "苹果",
        "**2**",
        "正文[^1]和**加粗[^note]**，重复[^1]，未定义[^missing]。\n\n[^1]: 第一条**脚注**与[链接](https://example.com)\n[^note]: 第二条注释\n    续行内容\n\n结束",
        "相邻[^a][^b]，`[^code]` 和 [清晰](https://example.com) 与 \\[^escaped]。\n\n[^a]: A\n[^b]: B",
        "空脚注[^empty]\n\n[^empty]:",
        "> 引用里的脚注[^quote]\n\n[^quote]: 引用注释",
        "独立文档[^1]，尚无定义",
    ];
    let mut pending: VecDeque<String> = sources.into_iter().map(str::to_string).collect();
    let mut seen = BTreeSet::new();
    let mut fixtures = Vec::new();
    while let Some(source) = pending.pop_front() {
        if !seen.insert(source.clone()) {
            continue;
        }
        let events = MarkdownRenderEventStream::fromContent(source.clone());
        let mut nested: BTreeMap<u64, String> = BTreeMap::new();
        for event in &events {
            if event.eventType == "chunk" {
                if let (Some(parent), Some(value)) = (event.parentBlockId, event.value.as_ref()) {
                    nested.entry(parent).or_default().push_str(value);
                }
            }
        }
        pending.extend(nested.into_values());
        fixtures.push(serde_json::json!({"source":source,"events":events}));
    }
    println!("{}", serde_json::to_string_pretty(&fixtures).unwrap());
}
