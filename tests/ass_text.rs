//! Tests for the ASS/SSA event-text layers: the override-tag DFA token stream
//! and the clean-text extraction built on top of it.

use fasrt::ass::text::{Override, PlainText, Segment, TextParser, TextToken};

/// Helper: collect a token stream.
fn tokens(input: &str) -> Vec<TextToken<'_>> {
  TextParser::new(input).collect()
}

/// Helper: the cleaned text of an event body.
///
/// `normalize` can only clean with `alloc`; without it the method is
/// documented to return the raw text, so every case that calls this is gated
/// to the tiers where its answer is meaningful.
#[cfg(any(feature = "alloc", feature = "std"))]
fn clean(input: &str) -> String {
  PlainText::new(input).normalize().to_string()
}

// ── Token stream: plain text ───────────────────────────────────────────────

#[test]
fn plain_text_is_one_token() {
  assert_eq!(tokens("hello world"), [TextToken::Text("hello world")]);
}

#[test]
fn empty_text_yields_no_tokens() {
  assert_eq!(tokens(""), []);
}

#[test]
fn non_ascii_text_is_borrowed_whole() {
  assert_eq!(tokens("君の声が"), [TextToken::Text("君の声が")]);
}

// ── Token stream: escapes ──────────────────────────────────────────────────

#[test]
fn hard_break_is_recognized() {
  assert_eq!(
    tokens("a\\Nb"),
    [
      TextToken::Text("a"),
      TextToken::HardBreak,
      TextToken::Text("b"),
    ],
  );
}

#[test]
fn soft_break_is_distinct_from_hard_break() {
  assert_eq!(
    tokens("a\\nb"),
    [
      TextToken::Text("a"),
      TextToken::SoftBreak,
      TextToken::Text("b"),
    ],
  );
}

#[test]
fn hard_space_is_recognized() {
  assert_eq!(
    tokens("a\\hb"),
    [
      TextToken::Text("a"),
      TextToken::HardSpace,
      TextToken::Text("b"),
    ],
  );
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn unrecognized_escape_keeps_both_characters() {
  // libass emits the backslash and advances one byte, so the `d` is examined
  // afresh and both characters survive.
  assert_eq!(
    tokens("a\\db"),
    [
      TextToken::Text("a"),
      TextToken::Text("\\"),
      TextToken::Text("db"),
    ],
  );
  assert_eq!(clean("a\\db"), "a\\db");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn double_backslash_is_literal() {
  assert_eq!(
    tokens("a\\\\b"),
    [
      TextToken::Text("a"),
      TextToken::Text("\\"),
      TextToken::Text("\\"),
      TextToken::Text("b"),
    ]
  );
  assert_eq!(clean("a\\\\b"), "a\\\\b");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn a_backslash_before_an_escape_does_not_consume_it() {
  // Only the first backslash is literal; the second one still starts `\N`,
  // because libass re-examines the byte after an unrecognized escape.
  assert_eq!(
    tokens("a\\\\Nb"),
    [
      TextToken::Text("a"),
      TextToken::Text("\\"),
      TextToken::HardBreak,
      TextToken::Text("b"),
    ],
  );
  assert_eq!(clean("a\\\\Nb"), "a\\\nb");

  assert_eq!(clean("a\\\\hb"), "a\\\u{00A0}b");
  assert_eq!(clean("a\\\\{b"), "a\\{b");
}

#[test]
fn trailing_backslash_is_literal() {
  assert_eq!(
    tokens("end\\"),
    [TextToken::Text("end"), TextToken::Text("\\")],
  );
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn escaped_open_brace_is_a_literal_brace() {
  // libass reads `\{` as an escape for a literal `{`, so it must not open an
  // override block and must not swallow the text that follows.
  assert_eq!(
    tokens("a\\{\\i1}b"),
    [
      TextToken::Text("a"),
      TextToken::EscapedBrace("{"),
      TextToken::Text("\\"),
      TextToken::Text("i1}b"),
    ],
  );
  assert_eq!(clean("a\\{\\i1}b"), "a{\\i1}b");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn escaped_close_brace_is_a_literal_brace() {
  assert_eq!(
    tokens("a\\}b"),
    [
      TextToken::Text("a"),
      TextToken::EscapedBrace("}"),
      TextToken::Text("b"),
    ],
  );
  assert_eq!(clean("a\\}b"), "a}b");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn escaped_braces_survive_cleaning_as_a_pair() {
  assert_eq!(clean("\\{not an override\\}"), "{not an override}");
}

// ── Token stream: override blocks ──────────────────────────────────────────

#[test]
fn override_block_strips_braces() {
  assert_eq!(
    tokens("{\\i1}x"),
    [
      TextToken::Override(Override::new("\\i1")),
      TextToken::Text("x"),
    ],
  );
}

#[test]
fn block_ends_at_the_first_closing_brace() {
  // The inner `{` is part of the block's comment text; the trailing `}` is
  // literal.
  let tokens = tokens("{a{b}c}");
  assert_eq!(
    tokens,
    [
      TextToken::Override(Override::new("a{b")),
      TextToken::Text("c}"),
    ],
  );
}

#[test]
fn stray_closing_brace_is_literal() {
  assert_eq!(tokens("a}b"), [TextToken::Text("a}b")]);
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn an_unmatched_open_brace_is_literal_text() {
  // libass only enters an override block when a closing `}` exists; treating
  // an unmatched `{` as a block would silently delete visible text.
  // The field holds no `}`, so it is lexed without the block rule and the
  // unmatched `{` merges into the surrounding literal run.
  assert_eq!(
    tokens("a{\\i1"),
    [
      TextToken::Text("a{"),
      TextToken::Text("\\"),
      TextToken::Text("i1"),
    ],
  );
  assert_eq!(clean("a{\\i1"), "a{\\i1");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn text_after_an_unmatched_brace_is_never_dropped() {
  assert_eq!(clean("visible{\\i1"), "visible{\\i1");
  assert_eq!(
    clean("{no closing brace and some words"),
    "{no closing brace and some words"
  );
}

#[test]
fn empty_block_is_a_block() {
  assert_eq!(tokens("{}"), [TextToken::Override(Override::new(""))]);
}

// ── Override tag splitting ─────────────────────────────────────────────────

#[test]
fn simple_tags_split_into_name_and_args() {
  let block = Override::new("\\i1\\b0");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 2);
  assert_eq!((tags[0].name(), tags[0].args()), ("i", "1"));
  assert_eq!((tags[1].name(), tags[1].args()), ("b", "0"));
}

#[test]
fn longest_known_name_wins() {
  // `fscx` must not be split as `fs` + `cx`, and `iclip` must not be `i`.
  let block = Override::new("\\fscx200\\fs40\\iclip(1,2,3,4)\\i1");
  let names: Vec<_> = block.tags().map(|tag| tag.name()).collect();
  assert_eq!(names, ["fscx", "fs", "iclip", "i"]);
}

#[test]
fn pos_is_not_split_as_p() {
  let block = Override::new("\\pos(320,240)");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 1);
  assert_eq!(tags[0].name(), "pos");
  assert_eq!(tags[0].args(), "(320,240)");
  assert_eq!(block.drawing_scale(), None);
}

#[test]
fn org_is_not_split_as_or() {
  let block = Override::new("\\org(640,360)");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags[0].name(), "org");
  assert_eq!(tags[0].args(), "(640,360)");
}

#[test]
fn nested_parens_belong_to_the_enclosing_tag() {
  let block = Override::new("\\t(0,500,\\frz360)\\i1");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 2);
  assert_eq!(tags[0].name(), "t");
  assert_eq!(tags[0].args(), "(0,500,\\frz360)");
  assert_eq!(tags[1].name(), "i");
}

#[test]
fn an_argument_list_ends_at_the_first_close_paren() {
  // libass does not track nesting: the inner `)` closes the argument list, so
  // `\p0` is a following tag and drawing mode is switched off.
  let block = Override::new("\\t(0,500,\\clip(0,0,10,10)\\p0");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 2);
  assert_eq!(tags[0].name(), "t");
  assert_eq!(tags[0].args(), "(0,500,\\clip(0,0,10,10)");
  assert_eq!(tags[1].name(), "p");
  assert_eq!(block.drawing_scale(), Some(0));
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn a_one_close_transform_before_p0_ends_drawing_mode() {
  // End to end: geometry must not leak into the cleaned text.
  assert_eq!(
    clean("{\\p1}m 0 0 l 9 9{\\t(0,500,\\clip(0,0,1,1)\\p0}caption"),
    "caption",
  );
}

#[test]
fn spaces_after_the_backslash_are_skipped() {
  // libass skips spaces between the backslash and the tag name.
  let block = Override::new("\\ p1");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 1);
  assert_eq!(tags[0].name(), "p");
  assert_eq!(tags[0].args(), "1");
  assert!(tags[0].is_known());
  assert_eq!(block.drawing_scale(), Some(1));
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn a_spaced_drawing_tag_still_suppresses_geometry() {
  assert_eq!(clean("{\\ p1}m 0 0 l 9 9{\\ p0}caption"), "caption");
}

#[test]
fn spaces_before_the_backslash_do_not_break_tag_names() {
  let block = Override::new("\\pos(10,20) \\i1");
  let names: Vec<_> = block.tags().map(|tag| tag.name()).collect();
  assert_eq!(names, ["pos", "i"]);
}

#[test]
fn unbalanced_parens_do_not_panic() {
  let block = Override::new("\\clip(1,2\\i1");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 1);
  assert_eq!(tags[0].name(), "clip");
  assert_eq!(tags[0].args(), "(1,2\\i1");
}

#[test]
fn leading_comment_before_the_first_tag_is_skipped() {
  let block = Override::new("comment here\\i1");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 1);
  assert_eq!(tags[0].name(), "i");
}

#[test]
fn a_block_with_no_tags_yields_nothing() {
  assert_eq!(Override::new("just a comment").tags().count(), 0);
}

#[test]
fn unknown_tags_are_tokenized_best_effort() {
  let block = Override::new("\\zz9\\i1");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(tags.len(), 2);
  assert_eq!((tags[0].name(), tags[0].args()), ("zz", "9"));
  assert!(!tags[0].is_known());
  assert!(tags[1].is_known());
}

#[test]
fn karaoke_tags_keep_their_case() {
  let block = Override::new("\\k21\\kf25\\ko19\\K22");
  let names: Vec<_> = block.tags().map(|tag| tag.name()).collect();
  assert_eq!(names, ["k", "kf", "ko", "K"]);
}

#[test]
fn colour_tags_are_recognized() {
  let block = Override::new("\\1c&H00FFFF&\\3c&H000000&\\4a&HFF&\\alpha&H80&");
  let names: Vec<_> = block.tags().map(|tag| tag.name()).collect();
  assert_eq!(names, ["1c", "3c", "4a", "alpha"]);
}

#[test]
fn tag_round_trips_through_display() {
  let block = Override::new("\\pos(10,20)\\i1");
  let rendered: String = block.tags().map(|tag| tag.to_string()).collect();
  assert_eq!(rendered, "\\pos(10,20)\\i1");
  assert_eq!(block.to_string(), "{\\pos(10,20)\\i1}");
}

#[test]
fn fsc_and_kt_are_not_split_as_shorter_tags() {
  // `\fsc` must not be read as `\fs` + "c", and `\kt` must not be `\k` + "t".
  let block = Override::new("\\fsc\\kt100\\fs40\\k21");
  let tags: Vec<_> = block.tags().collect();
  assert_eq!(
    tags.iter().map(|tag| tag.name()).collect::<Vec<_>>(),
    ["fsc", "kt", "fs", "k"],
  );
  assert_eq!(tags[0].args(), "");
  assert_eq!(tags[1].args(), "100");
  assert!(tags.iter().all(|tag| tag.is_known()));
}

// ── Drawing mode ───────────────────────────────────────────────────────────

#[test]
fn drawing_payload_is_reported_separately() {
  assert_eq!(
    tokens("{\\p1}m 0 0 l 10 0{\\p0}text"),
    [
      TextToken::Override(Override::new("\\p1")),
      TextToken::Drawing("m 0 0 l 10 0"),
      TextToken::Override(Override::new("\\p0")),
      TextToken::Text("text"),
    ],
  );
}

#[test]
fn drawing_mode_persists_to_the_end_of_the_field() {
  assert_eq!(
    tokens("{\\p1}m 0 0"),
    [
      TextToken::Override(Override::new("\\p1")),
      TextToken::Drawing("m 0 0"),
    ],
  );
}

#[test]
fn drawing_scale_reads_the_last_p_tag() {
  assert_eq!(Override::new("\\p1\\p0").drawing_scale(), Some(0));
  assert_eq!(Override::new("\\p0\\p4").drawing_scale(), Some(4));
}

#[test]
fn non_numeric_drawing_scale_switches_drawing_off() {
  assert_eq!(Override::new("\\pzz").drawing_scale(), Some(0));
}

#[test]
fn parser_exposes_drawing_state() {
  let mut parser = TextParser::new("{\\p1}m 0 0{\\p0}x");
  assert!(!parser.is_drawing());
  let _ = parser.next();
  assert!(parser.is_drawing());
  let _ = parser.next();
  let _ = parser.next();
  assert!(!parser.is_drawing());
}

#[test]
fn escapes_inside_drawing_mode_stay_drawing() {
  let tokens = tokens("{\\p1}m 0\\N0");
  assert_eq!(
    tokens,
    [
      TextToken::Override(Override::new("\\p1")),
      TextToken::Drawing("m 0"),
      TextToken::Drawing("\\N"),
      TextToken::Drawing("0"),
    ],
  );
}

// ── Clean text ─────────────────────────────────────────────────────────────

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn plain_line_needs_no_normalization() {
  let text = PlainText::new("Just a line of dialogue.");
  assert!(!text.requires_normalization());
  assert_eq!(text.normalize(), "Just a line of dialogue.");
  // Zero-copy: the borrowed slice is returned unchanged.
  assert!(std::ptr::eq(text.normalize(), text.as_raw()));
}

#[test]
fn markup_triggers_normalization() {
  assert!(PlainText::new("{\\i1}x").requires_normalization());
  assert!(PlainText::new("a\\Nb").requires_normalization());
  assert!(!PlainText::new("a}b").requires_normalization());
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn override_tags_are_dropped() {
  assert_eq!(clean("{\\i1}Hello{\\i0} world"), "Hello world");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn hard_break_becomes_a_newline() {
  assert_eq!(clean("one\\Ntwo"), "one\ntwo");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn soft_break_becomes_a_newline() {
  assert_eq!(clean("one\\ntwo"), "one\ntwo");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn hard_space_becomes_a_no_break_space() {
  assert_eq!(clean("one\\htwo"), "one\u{00A0}two");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn drawing_payloads_are_skipped() {
  assert_eq!(clean("{\\p1}m 0 0 l 99 99{\\p0}caption"), "caption");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn stray_braces_survive_cleaning() {
  assert_eq!(clean("a}b"), "a}b");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn karaoke_syllables_join_into_a_line() {
  assert_eq!(
    clean("{\\k21}ki{\\k18}mi{\\k30}no{\\k24} {\\k27}ko{\\k22}e"),
    "kimino koe",
  );
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn typesetting_tags_leave_only_the_words() {
  assert_eq!(
    clean("{\\move(0,360,1280,360,0,2900)\\frz350.5\\fad(250,250)}scrolling banner"),
    "scrolling banner",
  );
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn normalization_is_cached_and_stable() {
  let text = PlainText::new("{\\b1}bold{\\b0}");
  let first = text.normalize().to_string();
  let second = text.normalize().to_string();
  assert_eq!(first, "bold");
  assert_eq!(first, second);
}

#[test]
fn display_writes_the_cleaned_text() {
  assert_eq!(PlainText::new("{\\i1}shown{\\i0}").to_string(), "shown");
}

// ── Segments ───────────────────────────────────────────────────────────────

#[test]
fn segments_expose_break_kinds_separately() {
  let text = PlainText::new("a\\Nb\\nc\\hd");
  let segments: Vec<_> = text.segments().collect();
  assert_eq!(
    segments,
    [
      Segment::Text("a"),
      Segment::HardBreak,
      Segment::Text("b"),
      Segment::SoftBreak,
      Segment::Text("c"),
      Segment::HardSpace,
      Segment::Text("d"),
    ],
  );
}

#[test]
fn segments_drop_markup_and_drawings() {
  let text = PlainText::new("{\\an8}sign{\\p1}m 0 0{\\p0}!");
  let segments: Vec<_> = text.segments().collect();
  assert_eq!(segments, [Segment::Text("sign"), Segment::Text("!")]);
}

// ── Robustness ─────────────────────────────────────────────────────────────

/// Every input must tokenize to completion without panicking, and cleaning
/// must never panic either.
// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn adversarial_inputs_do_not_panic() {
  let inputs = [
    "",
    "{",
    "}",
    "{}",
    "\\",
    "\\\\",
    "\\N",
    "{\\",
    "{\\p",
    "{\\p1",
    "{\\p1}",
    "{\\p999999999999999999999999}x",
    "{\\t(((((}x",
    "{\\clip(",
    "){\\i1}",
    "{{{{{",
    "}}}}}",
    "\\{\\}",
    "{\\1c&H}",
    "君の{\\i1}声\\Nが{\\p1}m 0 0",
    "{\\p1}\\h\\N\\n{\\p0}",
    "a\\",
    "{\\fn日本語フォント}text",
    "{\\p1}{\\p0}{\\p1}{\\p0}",
  ];

  for input in inputs {
    let count = TextParser::new(input).count();
    let text = PlainText::new(input);
    let cleaned = text.normalize().to_string();
    let segments = text.segments().count();
    // The assertions exist to keep the work from being optimized away.
    assert!(count <= input.len() + 1, "input {input:?}");
    assert!(cleaned.len() <= input.len() * 3, "input {input:?}");
    assert!(segments <= count, "input {input:?}");
  }
}

/// A field made of many unmatched `{` must tokenize in linear time.
///
/// The override-block rule scans forward for a `}`; if it were allowed to run
/// where no `}` remains, each `{` would rescan the rest of the field and a
/// moderately large field would monopolize the CPU. Growing the input by 8x
/// must not grow the work by anything like 64x, so this asserts a wall-clock
/// ratio far below quadratic.
// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn unmatched_braces_tokenize_in_linear_time() {
  fn elapsed(count: usize) -> std::time::Duration {
    // `{\a` repeated: each `{` is unmatched and followed by a backslash, the
    // shape that defeats a naive maximal-munch scan.
    let input = "{\\a".repeat(count);
    let start = std::time::Instant::now();
    let tokens = TextParser::new(&input).count();
    let cleaned = PlainText::new(&input).normalize().len();
    assert!(tokens >= count, "expected at least one token per repeat");
    assert_eq!(cleaned, input.len(), "unmatched braces are literal text");
    start.elapsed()
  }

  // Warm up so the first measurement does not carry one-time costs.
  let _ = elapsed(1_000);

  let small = elapsed(4_000);
  let large = elapsed(32_000);

  // Quadratic growth would be ~64x. Allow a generous margin for timer noise
  // on a loaded machine while still failing decisively on O(n^2).
  let bound = small.saturating_mul(24) + std::time::Duration::from_millis(50);
  assert!(
    large <= bound,
    "tokenizing scaled superlinearly: {small:?} for 4k repeats, {large:?} for 32k",
  );
}

/// Tokens must cover the input exactly: concatenating every token's source
/// text reproduces the original field.
#[test]
fn tokens_cover_the_whole_input() {
  let inputs = [
    "hello",
    "{\\i1}hello{\\i0}",
    "a\\Nb\\nc\\hd",
    "a\\db",
    "{a{b}c}",
    "{\\p1}m 0 0{\\p0}tail",
    "trailing\\",
    "unmatched{\\i1",
    "escaped\\{braces\\}here",
  ];

  for input in inputs {
    let mut rebuilt = String::new();
    for token in TextParser::new(input) {
      match token {
        TextToken::Text(run) | TextToken::Drawing(run) => rebuilt.push_str(run),
        TextToken::EscapedBrace(brace) => {
          rebuilt.push('\\');
          rebuilt.push_str(brace);
        }
        TextToken::HardBreak => rebuilt.push_str("\\N"),
        TextToken::SoftBreak => rebuilt.push_str("\\n"),
        TextToken::HardSpace => rebuilt.push_str("\\h"),
        TextToken::Override(block) => rebuilt.push_str(&block.to_string()),
      }
    }
    assert_eq!(rebuilt, input, "token stream lost bytes for {input:?}");
  }
}
