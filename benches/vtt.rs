use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fasrt::vtt::Parser;
use fasrt::vtt::cue::CueParser;

const SMALL_VTT: &str = "\
WEBVTT

00:00:00.000 --> 00:00:01.000
Hello world!

00:00:01.000 --> 00:00:02.000
Goodbye world!
";

const SETTINGS_VTT: &str = "\
WEBVTT

STYLE
::cue { color: white; }

REGION
id:region1
width:40%
lines:3
regionanchor:0%,100%
viewportanchor:10%,90%
scroll:up

cue-1
00:00:00.000 --> 00:00:01.000 align:start position:10% size:80% line:0 vertical:rl region:region1
<b>Bold</b> and <i>italic</i> text

NOTE This is a comment

00:00:01.000 --> 00:00:05.000
Second cue with <v Roger Bingham>voice tag</v>
";

fn load_all_vtt_fixtures() -> String {
  let mut buf = String::new();
  for dir in &[
    "fixtures/webvtt/wpt-file-parsing",
    "fixtures/webvtt/wpt-cue-parsing",
  ] {
    if let Ok(read_dir) = std::fs::read_dir(dir) {
      let mut entries: Vec<_> = read_dir.filter_map(Result::ok).collect();
      entries.sort_by_key(|a| a.file_name());
      for entry in entries {
        if entry.path().extension().is_some_and(|e| e == "vtt")
          && let Ok(contents) = std::fs::read_to_string(entry.path())
        {
          buf.push_str(&contents);
          buf.push_str("\n\n");
        }
      }
    }
  }
  if buf.is_empty() {
    // Fallback to embedded samples so benches still run without fixtures.
    buf.push_str(SMALL_VTT);
    buf.push_str("\n\n");
    buf.push_str(SETTINGS_VTT);
  }
  buf
}

fn bench_vtt_parse(c: &mut Criterion) {
  let all_fixtures = load_all_vtt_fixtures();

  let mut group = c.benchmark_group("vtt/parse");

  // Small inline VTT
  group.throughput(Throughput::Bytes(SMALL_VTT.len() as u64));
  group.bench_function(BenchmarkId::new("parse", "small_2_cues"), |b| {
    b.iter(|| {
      let count = Parser::new(black_box(SMALL_VTT)).count();
      black_box(count);
    });
  });

  // VTT with settings, regions, styles, cue options
  group.throughput(Throughput::Bytes(SETTINGS_VTT.len() as u64));
  group.bench_function(BenchmarkId::new("parse", "with_settings"), |b| {
    b.iter(|| {
      let count = Parser::new(black_box(SETTINGS_VTT)).count();
      black_box(count);
    });
  });

  // All VTT fixtures
  group.throughput(Throughput::Bytes(all_fixtures.len() as u64));
  group.bench_function(BenchmarkId::new("parse", "all_fixtures"), |b| {
    b.iter(|| {
      let count = Parser::new(black_box(&all_fixtures)).count();
      black_box(count);
    });
  });

  group.finish();
}

fn bench_vtt_collect(c: &mut Criterion) {
  let mut group = c.benchmark_group("vtt/collect");

  group.throughput(Throughput::Bytes(SETTINGS_VTT.len() as u64));
  group.bench_function("with_settings", |b| {
    b.iter(|| {
      let blocks: Vec<_> = Parser::new(black_box(SETTINGS_VTT))
        .collect::<Result<_, _>>()
        .unwrap();
      black_box(blocks.len());
    });
  });

  group.finish();
}

/// Cue text with many timestamp tags to benchmark the cue-text parsing path.
fn build_cue_text_with_timestamps() -> String {
  let mut s = String::new();
  for i in 0..500 {
    let h = i / 3600;
    let m = (i % 3600) / 60;
    let sec = i % 60;
    s.push_str(&format!(
      "word <{h:02}:{m:02}:{sec:02}.{ms:03}>text",
      h = h,
      m = m,
      sec = sec,
      ms = (i * 7) % 1000
    ));
  }
  s
}

/// Cue text with tags but no timestamps.
const CUE_TEXT_TAGS: &str = "\
<b>bold <i>bold-italic</i></b> plain <u>underline</u> \
<v Roger>voice</v> <lang en>english</lang> <ruby>base<rt>ruby</rt></ruby> \
<c.highlight.big>classed</c> &amp; &lt; &gt; &nbsp; end";

fn bench_vtt_cue_text(c: &mut Criterion) {
  let ts_input = build_cue_text_with_timestamps();

  let mut group = c.benchmark_group("vtt/cue_text");

  // Tags only (no timestamps)
  group.throughput(Throughput::Bytes(CUE_TEXT_TAGS.len() as u64));
  group.bench_function("tags_only", |b| {
    b.iter(|| {
      let count = CueParser::new(black_box(CUE_TEXT_TAGS)).count();
      black_box(count);
    });
  });

  // Timestamp-heavy cue text
  group.throughput(Throughput::Bytes(ts_input.len() as u64));
  group.bench_function("500_timestamps", |b| {
    b.iter(|| {
      let count = CueParser::new(black_box(&ts_input)).count();
      black_box(count);
    });
  });

  group.finish();
}

criterion_group!(
  benches,
  bench_vtt_parse,
  bench_vtt_collect,
  bench_vtt_cue_text
);
criterion_main!(benches);
