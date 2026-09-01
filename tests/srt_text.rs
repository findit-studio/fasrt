//! Tests for the SubRip cue-body layers: the markup DFA token stream and the
//! clean-text extraction built on top of it.
//!
//! Every case here is a row of the dialect contract documented on
//! `fasrt::srt::text`, and each row is what FFmpeg's `subrip` decoder, VLC's
//! subtitle decoder and Aegisub's SRT reader agree to do — SubRip has no
//! specification, so the players are the specification.

use fasrt::srt::text::{
  Attribute, InlineCode, PlainText, Segment, StartTag, Tag, TextParser, TextToken,
};

/// Helper: collect a token stream.
fn tokens(input: &str) -> Vec<TextToken<'_>> {
  TextParser::new(input).collect()
}

/// Helper: collect the cleaned segments, which need no allocator.
fn segments(input: &str) -> Vec<Segment<'_>> {
  PlainText::new(input).segments().collect()
}

/// Helper: the cleaned text of a cue body.
///
/// `normalize` can only clean with `alloc`; without it the method is
/// documented to return the raw text, so every case that calls this is gated
/// to the tiers where its answer is meaningful.
#[cfg(any(feature = "alloc", feature = "std"))]
fn clean(input: &str) -> String {
  PlainText::new(input).normalize().to_string()
}

/// Helper: the sole start tag of a body.
fn start_tag(input: &str) -> StartTag<'_> {
  match tokens(input).into_iter().next() {
    Some(TextToken::StartTag(tag)) => tag,
    other => panic!("expected a start tag, got {other:?}"),
  }
}

// ── Token stream: literal text ─────────────────────────────────────────────

#[test]
fn plain_text_is_one_token() {
  assert_eq!(tokens("hello world"), [TextToken::Text("hello world")]);
}

#[test]
fn an_empty_body_yields_no_tokens() {
  assert_eq!(tokens(""), []);
}

#[test]
fn non_ascii_text_is_borrowed_whole() {
  assert_eq!(tokens("君の声が"), [TextToken::Text("君の声が")]);
}

#[test]
fn a_cue_body_keeps_its_own_newlines() {
  // A SubRip body is several lines; the breaks between them are ordinary
  // text, and only `<br>` is reported as a break.
  assert_eq!(tokens("first\nsecond"), [TextToken::Text("first\nsecond")]);
}

// ── Token stream: the five styling tags ────────────────────────────────────

#[test]
fn the_italic_pair_is_the_corpus_workhorse() {
  // `<i>…</i>` is all but five of the ~17 800 tags in fixtures/srt.
  assert_eq!(
    tokens("<i>Hello</i>"),
    [
      TextToken::StartTag(StartTag::new(Tag::Italic, "")),
      TextToken::Text("Hello"),
      TextToken::EndTag(Tag::Italic),
    ],
  );
}

#[test]
fn every_styling_tag_is_recognized() {
  for (body, tag) in [
    ("<b>x</b>", Tag::Bold),
    ("<i>x</i>", Tag::Italic),
    ("<u>x</u>", Tag::Underline),
    ("<s>x</s>", Tag::Strikeout),
    ("<font>x</font>", Tag::Font),
  ] {
    assert_eq!(
      tokens(body),
      [
        TextToken::StartTag(StartTag::new(tag, "")),
        TextToken::Text("x"),
        TextToken::EndTag(tag),
      ],
      "{body}",
    );
  }
}

#[test]
fn tag_names_are_ascii_case_insensitive() {
  // FFmpeg lowercases with `av_tolower`, VLC compares with `strcasecmp`, and
  // Aegisub matches with a case-insensitive regex.
  for (body, tag) in [
    ("<B>x</B>", Tag::Bold),
    ("<I>x</I>", Tag::Italic),
    ("<U>x</U>", Tag::Underline),
    ("<S>x</S>", Tag::Strikeout),
    ("<FONT>x</FONT>", Tag::Font),
    ("<FoNt>x</fOnT>", Tag::Font),
  ] {
    assert_eq!(
      tokens(body),
      [
        TextToken::StartTag(StartTag::new(tag, "")),
        TextToken::Text("x"),
        TextToken::EndTag(tag),
      ],
      "{body}",
    );
  }
}

#[test]
fn a_styling_tag_may_carry_attributes() {
  assert_eq!(start_tag("<i lang=en>x").attributes(), "lang=en");
}

// ── Token stream: `<font>` attributes ──────────────────────────────────────

#[test]
fn the_corpus_font_tag_is_read_whole() {
  // The exact tag carried by the five Naruto fixtures.
  let tag = start_tag("<font color=\"#ffff00\" size=14>x");
  assert_eq!(tag.tag(), Tag::Font);
  assert_eq!(tag.attributes(), "color=\"#ffff00\" size=14");

  let attrs: Vec<_> = tag.attrs().collect();
  assert_eq!(attrs.len(), 2);
  assert_eq!(attrs[0].name(), "color");
  assert_eq!(attrs[0].value(), Some("#ffff00"));
  assert_eq!(attrs[1].name(), "size");
  assert_eq!(attrs[1].value(), Some("14"));
  assert!(attrs.iter().all(Attribute::is_known));
}

#[test]
fn attribute_values_may_be_quoted_bare_or_absent() {
  let attrs: Vec<_> = start_tag("<font face='Comic Sans' size=1 color>x")
    .attrs()
    .collect();
  assert_eq!(attrs[0].value(), Some("Comic Sans"));
  assert_eq!(attrs[1].value(), Some("1"));
  // No `=` at all is different from an empty value.
  assert_eq!(attrs[2].value(), None);
  assert_eq!(
    start_tag("<font color=\"\">x")
      .attrs()
      .next()
      .unwrap()
      .value(),
    Some("")
  );
}

#[test]
fn an_unclosed_quote_runs_to_the_end_of_the_tag() {
  let attrs: Vec<_> = start_tag("<font face=\"Comic Sans>x").attrs().collect();
  assert_eq!(attrs.len(), 1);
  assert_eq!(attrs[0].value(), Some("Comic Sans"));
}

#[test]
fn only_color_size_and_face_are_common_ground() {
  // VLC reads six more, but these three are what every surveyed player reads.
  let attrs: Vec<_> = start_tag("<font SIZE=1 Color=red FACE=x back-color=blue alpha=1>y")
    .attrs()
    .collect();
  let known: Vec<_> = attrs.iter().map(Attribute::is_known).collect();
  assert_eq!(known, [true, true, true, false, false]);
  // The name is reported verbatim; only the comparison folds case.
  assert_eq!(attrs[0].name(), "SIZE");
}

#[test]
fn attribute_names_may_be_hyphenated() {
  let attrs: Vec<_> = start_tag("<font outline-color=red>x").attrs().collect();
  assert_eq!(attrs.len(), 1);
  assert_eq!(attrs[0].name(), "outline-color");
  assert_eq!(attrs[0].value(), Some("red"));
}

#[test]
fn a_tag_ends_at_the_first_close_angle_even_inside_a_quote() {
  // FFmpeg's tag scan and VLC's both stop at the first `>` with no regard for
  // quoting, so a `>` in an attribute value ends the tag there and the
  // remainder is text. Every one of them agrees, so this is contract.
  assert_eq!(
    tokens("<font face=\"a>b\">x"),
    [
      TextToken::StartTag(StartTag::new(Tag::Font, "face=\"a")),
      TextToken::Text("b\">x"),
    ],
  );
}

#[test]
fn a_styling_tags_attributes_are_dropped_with_it() {
  // `<i am here>` is an italic tag carrying attributes in every surveyed
  // player — FFmpeg splits the name at the first space, VLC reads the name
  // then skips to the `>`, Aegisub's regex takes the rest as attributes — so
  // the words inside it are markup, not text, in all of them.
  assert_eq!(
    tokens("<i am here>text"),
    [
      TextToken::StartTag(StartTag::new(Tag::Italic, "am here")),
      TextToken::Text("text"),
    ],
  );
}

// ── Token stream: a `<` that opens no tag ──────────────────────────────────

#[test]
fn a_bare_left_angle_is_literal_text() {
  // The sharpest divergence from a WebVTT tokenizer, which consumes this as
  // an unterminated tag and loses the rest of the line.
  assert_eq!(
    tokens("I <3 this"),
    [
      TextToken::Text("I "),
      TextToken::Text("<"),
      TextToken::Text("3 this"),
    ],
  );
}

#[test]
fn an_angle_bracketed_phrase_is_literal_text() {
  // The Japanese narration convention, 141 times over in fixtures/srt: the
  // brackets are punctuation, not markup, and every surveyed player shows
  // them because the name is not one it knows.
  assert_eq!(
    tokens("<けど>"),
    [TextToken::Text("<"), TextToken::Text("けど>")],
  );
}

#[test]
fn an_unclosed_narration_bracket_keeps_the_whole_line() {
  // 198 lines of fixtures/srt open a bracket and never close it.
  assert_eq!(
    tokens("<こんなにも 見事な逃げ足を"),
    [
      TextToken::Text("<"),
      TextToken::Text("こんなにも 見事な逃げ足を"),
    ],
  );
}

#[test]
fn an_unterminated_known_tag_is_literal_text() {
  // FFmpeg's tag scan needs a `>` before it will call this markup, and
  // Aegisub's regex needs one too. A WebVTT tokenizer would read `<b` as a
  // start tag; here it is two characters of text.
  assert_eq!(
    tokens("bold? <b"),
    [
      TextToken::Text("bold? "),
      TextToken::Text("<"),
      TextToken::Text("b"),
    ]
  );
}

#[test]
fn an_unknown_tag_name_is_literal_text() {
  assert_eq!(
    tokens("<span class=\"x\">hi"),
    [TextToken::Text("<"), TextToken::Text("span class=\"x\">hi"),],
  );
}

#[test]
fn a_name_that_merely_starts_with_a_known_one_is_not_that_tag() {
  // `<basic>` is not `<b>`: the name must be the whole name.
  assert_eq!(
    tokens("<basic>"),
    [TextToken::Text("<"), TextToken::Text("basic>")],
  );
}

#[test]
fn a_run_of_left_angles_is_literal_text() {
  // FFmpeg calls these "likely latin guillemets in ASCII".
  assert_eq!(
    tokens("<<quoted>>"),
    [
      TextToken::Text("<"),
      TextToken::Text("<"),
      TextToken::Text("quoted>>"),
    ],
  );
}

#[test]
fn a_voice_tag_is_literal_text() {
  // SubRip has no speaker vocabulary; `<v>` is a WebVTT tag and nothing here
  // recognizes it.
  assert_eq!(
    tokens("<v Roger>Hi"),
    [TextToken::Text("<"), TextToken::Text("v Roger>Hi")],
  );
}

// ── Token stream: `<br>` ───────────────────────────────────────────────────

#[test]
fn every_form_of_br_is_one_line_break() {
  // FFmpeg reaches its `br` branch whether or not the tag was closing.
  for body in ["a<br>b", "a<br/>b", "a<br />b", "a</br>b"] {
    assert_eq!(
      tokens(body),
      [
        TextToken::Text("a"),
        TextToken::LineBreak,
        TextToken::Text("b"),
      ],
      "{body}",
    );
  }
}

#[test]
fn br_is_not_read_as_bold() {
  assert_eq!(tokens("<br>"), [TextToken::LineBreak]);
  assert_eq!(
    tokens("<b>x</b>"),
    [
      TextToken::StartTag(StartTag::new(Tag::Bold, "")),
      TextToken::Text("x"),
      TextToken::EndTag(Tag::Bold),
    ],
  );
}

// ── Token stream: inline style codes left by a converter ───────────────────

#[test]
fn ssa_and_microdvd_codes_are_reported_separately_from_text() {
  assert_eq!(
    tokens("{\\an8}top"),
    [
      TextToken::InlineCode(InlineCode::new("\\an8")),
      TextToken::Text("top"),
    ],
  );
  assert_eq!(
    tokens("{Y:i}slanted"),
    [
      TextToken::InlineCode(InlineCode::new("Y:i")),
      TextToken::Text("slanted"),
    ],
  );
}

#[test]
fn the_microdvd_letter_set_is_ffmpegs() {
  for letter in ["C", "c", "F", "f", "o", "P", "S", "s", "Y", "y"] {
    let body = std::format!("{{{letter}:1}}x");
    assert!(
      matches!(tokens(&body)[0], TextToken::InlineCode(_)),
      "{body}",
    );
  }
  // A letter outside the set is not a style code.
  assert_eq!(
    tokens("{Z:1}x"),
    [TextToken::Text("{"), TextToken::Text("Z:1}x")],
  );
}

#[test]
fn a_brace_that_opens_no_code_is_literal_text() {
  assert_eq!(
    tokens("a{b}c"),
    [
      TextToken::Text("a"),
      TextToken::Text("{"),
      TextToken::Text("b}c"),
    ]
  );
}

#[test]
fn an_unterminated_code_is_literal_text() {
  // Cleaning never deletes text a renderer would show, so a `{` with no `}`
  // stays put.
  assert_eq!(
    tokens("{\\an8"),
    [TextToken::Text("{"), TextToken::Text("\\an8")],
  );
}

#[test]
fn a_nested_brace_stops_the_code() {
  // Braces do not nest in SSA, so an inner `{` means the outer one opened
  // nothing — and bounding the scan this way is what keeps tokenizing linear.
  assert_eq!(
    tokens("{\\pos(1,{2)}"),
    [
      TextToken::Text("{"),
      TextToken::Text("\\pos(1,"),
      TextToken::Text("{"),
      TextToken::Text("2)}"),
    ],
  );
}

// ── Serialization ──────────────────────────────────────────────────────────

#[test]
fn tags_and_codes_serialize_back_to_markup() {
  assert_eq!(StartTag::new(Tag::Italic, "").to_string(), "<i>");
  assert_eq!(
    StartTag::new(Tag::Font, "size=14").to_string(),
    "<font size=14>",
  );
  assert_eq!(InlineCode::new("\\an8").to_string(), "{\\an8}");
  assert_eq!(Tag::Strikeout.to_string(), "s");
}

#[test]
fn an_attribute_serializes_and_quotes_only_when_it_must() {
  let attrs: Vec<_> = start_tag("<font size=14 face=\"Comic Sans\" color>x")
    .attrs()
    .collect();
  assert_eq!(attrs[0].to_string(), "size=14");
  assert_eq!(attrs[1].to_string(), "face=\"Comic Sans\"");
  assert_eq!(attrs[2].to_string(), "color");
}

// ── Segments: allocation-free cleaning, available on every tier ────────────

#[test]
fn segments_drop_markup_and_keep_text() {
  assert_eq!(
    segments("<i>Hi</i><br>there"),
    [
      Segment::Text("Hi"),
      Segment::LineBreak,
      Segment::Text("there"),
    ],
  );
}

#[test]
fn segments_keep_a_bare_left_angle() {
  assert_eq!(
    segments("I <3 this"),
    [
      Segment::Text("I "),
      Segment::Text("<"),
      Segment::Text("3 this"),
    ],
  );
}

#[test]
fn segments_drop_inline_style_codes() {
  assert_eq!(segments("{\\an8}top"), [Segment::Text("top")]);
}

// ── Clean text ─────────────────────────────────────────────────────────────

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn markup_is_dropped_and_text_is_kept() {
  assert_eq!(clean("<i>Hello</i> world"), "Hello world");
  assert_eq!(clean("<b><u><s>x</s></u></b>"), "x");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn the_font_tag_is_dropped_by_contract() {
  // One of the two rows a consumer pinned against the WebVTT substitution,
  // which dropped `<font>` only because it happens not to be a WebVTT tag.
  assert_eq!(
    clean("<font color=\"#ffff00\" size=14>Naruto</font>"),
    "Naruto",
  );
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn a_bare_left_angle_keeps_the_rest_of_the_line() {
  // The other pinned row. The W3C cue-text algorithm loses everything after
  // the `<`; here nothing is lost.
  assert_eq!(clean("I <3 this"), "I <3 this");
  assert_eq!(clean("<けど>"), "<けど>");
  assert_eq!(
    clean("<こんなにも 見事な逃げ足を"),
    "<こんなにも 見事な逃げ足を",
  );
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn character_references_are_literal_text() {
  // No surveyed player decodes them: SubRip borrows HTML's tag syntax and
  // nothing else. `&lrm;` appears nine times in fixtures/srt.
  assert_eq!(clean("Tom &amp; Jerry"), "Tom &amp; Jerry");
  assert_eq!(clean("&lrm;いな"), "&lrm;いな");
  assert_eq!(clean("&#160;"), "&#160;");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn a_nul_byte_is_left_alone() {
  // WebVTT replaces NULL with U+FFFD; that is a WebVTT rule.
  assert_eq!(clean("a\0b"), "a\0b");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn an_unclosed_tag_still_yields_its_text() {
  // These tags are binary state markers, not a tree, so nothing is lost when
  // they do not pair up.
  assert_eq!(clean("<i>never closed"), "never closed");
  assert_eq!(
    clean("closed but never opened</i>"),
    "closed but never opened"
  );
  // FFmpeg's own example of why these are state markers and not a tree.
  assert_eq!(clean("<b> foo <i> bar </b> bla </i>"), " foo  bar  bla ");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn br_becomes_a_newline() {
  assert_eq!(clean("first<br>second"), "first\nsecond");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn the_bodys_own_newlines_are_preserved() {
  assert_eq!(clean("<i>first\nsecond</i>"), "first\nsecond");
  // Line terminators are not rewritten: markup removal is the whole job.
  assert_eq!(clean("<i>first\r\nsecond</i>"), "first\r\nsecond");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn inline_style_codes_are_dropped() {
  assert_eq!(clean("{\\an8}{\\i1}top{\\i0}"), "top");
  assert_eq!(clean("{Y:i}slanted"), "slanted");
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn a_body_without_markup_is_borrowed_not_copied() {
  let body = "no markup here";
  let plain = PlainText::new(body);
  assert!(!plain.requires_normalization());
  assert_eq!(plain.normalize().as_ptr(), body.as_ptr());
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn normalization_is_cached_and_stable() {
  let plain = PlainText::new("<i>hi</i>");
  let first = plain.normalize().as_ptr();
  assert_eq!(plain.normalize(), "hi");
  assert_eq!(plain.normalize().as_ptr(), first);
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn display_writes_the_cleaned_text() {
  assert_eq!(PlainText::new("<b>bold</b>").to_string(), "bold");
}

// ── Depth ──────────────────────────────────────────────────────────────────

/// Skipped under Miri: interpreting 200 000 tokens costs minutes there, and an
/// interpreter has no host stack to exhaust, so the recursion this fixture is
/// here to rule out is not one Miri would be checking.
#[cfg_attr(
  miri,
  ignore = "200 000-token fixture, and Miri cannot overflow the host stack"
)]
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn deeply_nested_markup_costs_no_stack() {
  // This module builds no tree, so there is no recursive walk to overflow and
  // no depth bound to configure. A payload that aborts an unbounded tree
  // builder is cleaned in one pass here.
  let depth = 100_000;
  let body = std::format!("{}words{}", "<i>".repeat(depth), "</i>".repeat(depth));

  let plain = PlainText::new(&body);
  assert_eq!(plain.normalize(), "words");
  assert_eq!(TextParser::new(&body).count(), 2 * depth + 1);
}

// ── The real-world corpus ──────────────────────────────────────────────────

/// Sweeps over `fixtures/srt` — some 8 MB across 332 files.
///
/// Every case here is skipped under Miri: reading and parsing that much text
/// in an interpreter costs many minutes per target, and what these sweeps
/// check is dialect conformance rather than anything an interpreter can see.
/// The allocation-free paths they exercise are covered above on inputs Miri
/// can afford.
#[cfg(feature = "std")]
mod corpus {
  use super::*;

  use fasrt::srt::Parser;

  /// The synthetic fixture that carries one contract row per entry.  It is
  /// excluded from the real-world sweeps below, which have their own oracle.
  const CONTRACT_FIXTURE: &str = "subrip_dialect.srt";

  /// Every cue body of one fixture, in file order.
  fn bodies_of(name: &str) -> Vec<String> {
    let path = std::format!("{}/fixtures/srt/{name}", env!("CARGO_MANIFEST_DIR"));
    let text =
      std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    Parser::lossy(&text)
      .filter_map(Result::ok)
      .map(|entry| entry.body_ref().to_string())
      .collect()
  }

  /// Every cue body of the real-world fixtures, in file order.
  ///
  /// Five fixtures are ISO-8859 Greek rather than UTF-8 and are skipped: a
  /// `&str` parser cannot be pointed at them at all, which is also why the
  /// corpus's only `<font …>` rows reach this module through
  /// [`CONTRACT_FIXTURE`] instead.
  fn bodies() -> Vec<String> {
    let dir = std::format!("{}/fixtures/srt", env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
      .unwrap_or_else(|e| panic!("failed to read {dir}: {e}"))
      .map(|entry| entry.unwrap().path())
      .filter(|path| path.extension().is_some_and(|ext| ext == "srt"))
      .filter(|path| path.file_name().is_none_or(|name| name != CONTRACT_FIXTURE))
      .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
      let Ok(text) = std::fs::read_to_string(&path) else {
        continue;
      };
      out.extend(
        Parser::lossy(&text)
          .filter_map(Result::ok)
          .map(|entry| entry.body_ref().to_string()),
      );
    }
    assert!(!out.is_empty(), "the corpus should not be empty");
    out
  }

  #[cfg_attr(miri, ignore = "8 MB corpus sweep")]
  #[test]
  fn the_contract_table_is_pinned_row_by_row() {
    // One row per entry of `fixtures/srt/subrip_dialect.srt`, read through the
    // whole-file parser so the pinning covers the path an embedded consumer
    // takes: cue body in, clean text out.
    let expected = [
      "Within the spreading darkness",    // the styling pair
      "Naruto",                           // `<font …>`, dropped by contract
      "I <3 this",                        // a `<` that opens no tag
      "<けど>",                           // an angle-bracketed phrase
      "<こんなにも 見事な逃げ足を",       // an unclosed narration bracket
      "Case folds",                       // names fold case
      "bold under struck",                // the other styling tags
      "first\nsecond",                    // `<br>`
      "top",                              // an SSA inline code
      "slanted",                          // a MicroDVD inline code
      "Tom &amp; Jerry",                  // not HTML: `&` is text
      "<span class=\"x\">unknown</span>", // an unrecognized name is text
      "never closed",                     // an unclosed tag keeps its text
      "<v Roger>Hi",                      // SubRip has no voice tag
      "bold? <b",                         // an unterminated tag is text
      "first line\nsecond line",          // the body's own newline survives
    ];

    let bodies = bodies_of(CONTRACT_FIXTURE);
    assert_eq!(bodies.len(), expected.len(), "row count drifted");
    for (body, want) in bodies.iter().zip(expected) {
      assert_eq!(PlainText::new(body).normalize(), want, "row {body:?}");
    }
  }

  /// An oracle independent of the parser under test: the census of
  /// `fixtures/srt` finds exactly four markup shapes in it — `<i>`, `</i>`,
  /// `<font …>` and `</font>` — so removing those, and nothing else, is what
  /// cleaning the corpus must produce.
  fn strip_corpus_markup(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;

    while let Some(i) = rest.find('<') {
      out.push_str(&rest[..i]);
      let tail = &rest[i..];

      if let Some(tag) = ["<i>", "</i>", "</font>"]
        .iter()
        .find(|tag| tail.starts_with(**tag))
      {
        rest = &tail[tag.len()..];
      } else if tail.starts_with("<font ")
        && let Some(end) = tail.find('>')
      {
        rest = &tail[end + 1..];
      } else {
        // Anything else is text, including the `<` itself.
        out.push('<');
        rest = &tail[1..];
      }
    }

    out.push_str(rest);
    out
  }

  #[cfg_attr(miri, ignore = "8 MB corpus sweep")]
  #[test]
  fn cleaning_the_corpus_removes_markup_and_nothing_else() {
    let bodies = bodies();
    let mut with_markup = 0usize;
    let mut with_bare_angle = 0usize;

    for body in &bodies {
      assert_eq!(
        PlainText::new(body).normalize(),
        strip_corpus_markup(body),
        "cleaning changed text in {body:?}",
      );
      if body.contains('<') {
        with_markup += 1;
      }
      if strip_corpus_markup(body).contains('<') {
        with_bare_angle += 1;
      }
    }

    // The rows that make the sweep worth running.
    assert!(with_markup > 0, "the corpus should exercise tags");
    assert!(
      with_bare_angle > 0,
      "the corpus should exercise a `<` that opens no tag",
    );
  }

  #[cfg_attr(miri, ignore = "8 MB corpus sweep")]
  #[test]
  fn no_corpus_body_is_emptied_by_cleaning() {
    for body in &bodies() {
      if !body.trim().is_empty() {
        assert!(
          !PlainText::new(body).normalize().trim().is_empty(),
          "cleaning emptied {body:?}",
        );
      }
    }
  }

  #[cfg_attr(miri, ignore = "8 MB corpus sweep")]
  #[test]
  fn the_corpus_rows_a_webvtt_tokenizer_gets_wrong() {
    use fasrt::vtt::cue::{CueParser, CueToken};

    // What routing a SubRip body through the WebVTT cue-text layer produces —
    // the substitution a consumer carries while SubRip has no face of its own.
    fn as_webvtt(body: &str) -> String {
      CueParser::new(body)
        .filter_map(|token| match token {
          CueToken::Text(text) => Some(text.normalize().to_string()),
          _ => None,
        })
        .collect()
    }

    let mut divergent = 0usize;
    for body in &bodies() {
      let ours = PlainText::new(body).normalize().to_string();
      if ours == as_webvtt(body) {
        continue;
      }
      divergent += 1;
      // Every divergence must be one the contract names: a `<` that opens no
      // tag, or a character reference the WebVTT layer decodes and this one
      // leaves alone.
      assert!(
        ours.contains('<') || ours.contains('&'),
        "undocumented divergence on {body:?}",
      );
    }

    assert!(
      divergent > 0,
      "the corpus should show why the substitution needed replacing",
    );
  }
}
