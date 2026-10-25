//! Parser-only microbenchmark, without transport or Flutter painting.
//! Production FFI + Dart node benchmark: python3 tools/benchmarks/markdown_pipeline.py
//! Run: cargo run --release -p operit-util --example markdown_parse_bench
//! Chunk sizes are Unicode scalar values; input sizes and throughput use bytes.
use operit_util::streamnative::MarkdownInlineStream::MarkdownInlineStream;
use operit_util::MarkdownRenderStream::MarkdownRenderEventStream;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum Stage {
    Inline,
    Events,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Events => "events",
        }
    }
}

enum Parser {
    Inline(MarkdownInlineStream),
    Events(MarkdownRenderEventStream),
}
impl Parser {
    fn new(stage: Stage) -> Self {
        match stage {
            Stage::Inline => Self::Inline(MarkdownInlineStream::default()),
            Stage::Events => Self::Events(MarkdownRenderEventStream::new("benchmark-chat".into())),
        }
    }
    fn push(&mut self, chunk: &str) -> usize {
        match self {
            Self::Inline(parser) => {
                let out = parser.push(black_box(chunk));
                black_box(&out).len()
            }
            Self::Events(parser) => {
                let out = parser.pushChunk(black_box(chunk));
                black_box(&out).len()
            }
        }
    }
    fn finish(&mut self) -> usize {
        match self {
            Self::Inline(parser) => {
                let out = parser.finish();
                black_box(&out).len()
            }
            Self::Events(parser) => {
                let out = parser.finish();
                black_box(&out).len()
            }
        }
    }
}

fn source(pattern: &str, bytes: usize) -> String {
    let mut text = pattern.repeat(bytes.div_ceil(pattern.len()));
    let mut end = bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}
fn chunks(text: &str, chars: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for (count, (index, _)) in text.char_indices().enumerate() {
        if count > 0 && count % chars == 0 {
            chunks.push(&text[start..index]);
            start = index;
        }
    }
    if start < text.len() {
        chunks.push(&text[start..]);
    }
    chunks
}
fn run(stage: Stage, chunks: &[&str]) -> (Duration, usize) {
    let start = Instant::now();
    let mut parser = Parser::new(stage);
    let mut count = 0;
    for chunk in chunks {
        count += parser.push(chunk);
    }
    count += parser.finish();
    black_box(count);
    drop(parser);
    (start.elapsed(), count)
}
fn percentile(values: &mut [u128], percentile: usize) -> f64 {
    values.sort_unstable();
    let index = ((values.len() - 1) * percentile) / 100;
    values[index] as f64 / 1_000.0
}
fn bench(case: &str, text: &str, chunk_chars: usize, stage: Stage) {
    // Input construction/chunk splitting are outside all measured regions.
    let chunks = chunks(text, chunk_chars);
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_millis(50) {
        run(stage, &chunks);
    }
    let mut totals = Vec::new();
    let mut count = 0;
    for _ in 0..7 {
        let (elapsed, emitted) = run(stage, &chunks);
        totals.push(elapsed.as_nanos());
        count = emitted;
    }
    totals.sort_unstable();
    let median_ms = totals[totals.len() / 2] as f64 / 1_000_000.0;
    let mib_s = text.len() as f64 / 1_048_576.0 / (median_ms / 1_000.0);

    // A separate pass measures push latency so per-push clock calls do not
    // contaminate the throughput measurements above. Includes output drops.
    let mut parser = Parser::new(stage);
    let mut latency = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        let start = Instant::now();
        black_box(parser.push(chunk));
        latency.push(start.elapsed().as_nanos());
    }
    let start = Instant::now();
    black_box(parser.finish());
    let finish_us = start.elapsed().as_secs_f64() * 1_000_000.0;
    let window = (latency.len() / 10).max(1);
    let early_us = percentile(&mut latency[..window].to_vec(), 50);
    let late_us = percentile(&mut latency[latency.len() - window..].to_vec(), 50);
    let p95_us = percentile(&mut latency, 95);
    println!(
        "{case},{},{},{},{},{median_ms:.3},{mib_s:.2},{p95_us:.3},{early_us:.3},{late_us:.3},{finish_us:.3},{count}",
        stage.name(), text.len(), chunk_chars, chunks.len(),
    );
}
fn main() {
    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    println!(
        "# optimized Rust parser benchmark; arch={}; median of 7 warm runs",
        std::env::consts::ARCH
    );
    println!("case,stage,bytes,chunk_chars,pushes,median_ms,mib_per_s,push_p95_us,early_push_median_us,late_push_median_us,finish_us,output_items");
    let mixed = "# **标题**\n\n这是中文与 English 混合的正文，包含 **粗体 *斜体***、`code`、~~删除线~~ 和 [链接](https://example.com/a_(b))。\n\n- **列表项目**\n  - 第二层项目\n3) 有序项目\n\n> **引用**\n> - 引用中的列表\n\n```rust\nlet value = 42;\n```\n\n| 名称 | 数量 |\n| --- | ---: |\n| 苹果 | **2** |\n\n";
    for bytes in [10 * 1024, 100 * 1024, 1024 * 1024] {
        let text = source(mixed, bytes);
        for chunk_chars in [32, 128, 1024] {
            bench("mixed", &text, chunk_chars, Stage::Events);
        }
        bench("mixed", &text, 128, Stage::Inline);
    }
    let small = source(mixed, 100 * 1024);
    bench("mixed", &small, 1, Stage::Events);
    bench("mixed", &small, 1, Stage::Inline);
    let plain = "普通中文正文 with English text and numbers 1234567890，继续输出。";
    let styles = "**粗体 *斜体* `代码` [链接](https://example.com/a_(b))** ~~删除~~ ";
    for bytes in [100 * 1024, 1024 * 1024] {
        for (name, pattern) in [("plain", plain), ("dense_styles", styles)] {
            let text = source(pattern, bytes);
            bench(name, &text, 128, Stage::Events);
        }
        // Undecided syntax buffers until EOF: expose deferred work explicitly.
        let text = format!("[{}", source(plain, bytes - 1));
        bench("unclosed_link", &text, 128, Stage::Events);
    }
}
