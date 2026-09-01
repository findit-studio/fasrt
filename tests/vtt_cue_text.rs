#![cfg(any(feature = "alloc", feature = "std"))]

use fasrt::vtt::cue::{
  CueParser, CueStr, CueText, CueToken, DEFAULT_MAX_DEPTH, Node, Options, Tag,
};

// ── CueParser (token iterator) tests ────────────────────────────────────────

#[test]
fn parse_plain_text() {
  let tokens: Vec<_> = CueParser::new("hello world").collect();
  assert_eq!(tokens.len(), 1);
  assert_eq!(tokens[0], CueToken::Text(CueStr::borrowed("hello world")));
}

#[test]
fn parse_bold_tag() {
  let tokens: Vec<_> = CueParser::new("<b>bold</b>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Bold,
      classes,
      annotation: None
    } if classes.is_empty()
  ));
  assert_eq!(tokens[1], CueToken::Text(CueStr::borrowed("bold")));
  assert_eq!(tokens[2], CueToken::EndTag(Tag::Bold));
}

#[test]
fn parse_italic_tag() {
  let tokens: Vec<_> = CueParser::new("<i>italic</i>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Italic,
      ..
    }
  ));
  assert_eq!(tokens[2], CueToken::EndTag(Tag::Italic));
}

#[test]
fn parse_underline_tag() {
  let tokens: Vec<_> = CueParser::new("<u>underline</u>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Underline,
      ..
    }
  ));
}

#[test]
fn parse_class_with_classes() {
  let tokens: Vec<_> = CueParser::new("<c.loud.important>text</c>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Class,
      classes: "loud.important",
      annotation: None,
    }
  ));
}

#[test]
fn parse_voice_tag() {
  let tokens: Vec<_> = CueParser::new("<v Roger Bingham>text</v>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Voice,
      annotation: Some("Roger Bingham"),
      ..
    }
  ));
}

#[test]
fn parse_lang_tag() {
  let tokens: Vec<_> = CueParser::new("<lang en>hello</lang>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Lang,
      annotation: Some("en"),
      ..
    }
  ));
}

#[test]
fn parse_ruby_tags() {
  let tokens: Vec<_> = CueParser::new("<ruby>base<rt>text</rt></ruby>").collect();
  assert_eq!(tokens.len(), 6);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag { tag: Tag::Ruby, .. }
  ));
  assert_eq!(tokens[1], CueToken::Text(CueStr::borrowed("base")));
  assert!(matches!(
    &tokens[2],
    CueToken::StartTag {
      tag: Tag::RubyText,
      ..
    }
  ));
  assert_eq!(tokens[3], CueToken::Text(CueStr::borrowed("text")));
  assert_eq!(tokens[4], CueToken::EndTag(Tag::RubyText));
  assert_eq!(tokens[5], CueToken::EndTag(Tag::Ruby));
}

#[test]
fn parse_timestamp_tag() {
  let tokens: Vec<_> = CueParser::new("text<00:05.000>more").collect();
  assert_eq!(tokens.len(), 3);
  assert_eq!(tokens[0], CueToken::Text(CueStr::borrowed("text")));
  assert!(matches!(&tokens[1], CueToken::Timestamp(ts) if ts.to_duration().as_secs() == 5));
  assert_eq!(tokens[2], CueToken::Text(CueStr::borrowed("more")));
}

#[test]
fn parse_timestamp_tag_long_form() {
  let tokens: Vec<_> = CueParser::new("<01:02:03.456>").collect();
  assert_eq!(tokens.len(), 1);
  if let CueToken::Timestamp(ts) = &tokens[0] {
    assert_eq!(ts.to_duration().as_millis(), 3723456);
  } else {
    panic!("expected timestamp");
  }
}

#[test]
fn parse_entities() {
  let tokens: Vec<_> = CueParser::new("a&amp;b&lt;c&gt;d").collect();
  assert_eq!(tokens.len(), 1);
  // Raw text still contains entities
  assert_eq!(tokens[0].as_raw_text().unwrap(), "a&amp;b&lt;c&gt;d");
  // Normalized text has them decoded
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "a&b<c>d");
  // Flag is set
  assert!(tokens[0].requires_normalization());
}

#[test]
fn parse_entity_nbsp() {
  let tokens: Vec<_> = CueParser::new("hello&nbsp;world").collect();
  assert_eq!(
    tokens[0].as_normalized_text().unwrap(),
    "hello\u{00A0}world"
  );
}

#[test]
fn parse_entity_lrm_rlm() {
  let tokens: Vec<_> = CueParser::new("a&lrm;b&rlm;c").collect();
  assert_eq!(
    tokens[0].as_normalized_text().unwrap(),
    "a\u{200E}b\u{200F}c"
  );
}

#[test]
fn parse_unknown_entity_passthrough() {
  let tokens: Vec<_> = CueParser::new("a&unknown;b").collect();
  let text = tokens[0].as_normalized_text().unwrap();
  assert!(text.contains("&unknown;"));
}

#[test]
fn parse_unknown_tag_skipped() {
  let tokens: Vec<_> = CueParser::new("<unknown>text</unknown>").collect();
  // Unknown tags are skipped, text is still emitted
  assert_eq!(tokens.len(), 1);
  assert_eq!(tokens[0], CueToken::Text(CueStr::borrowed("text")));
}

#[test]
fn parse_nested_tags() {
  let tokens: Vec<_> = CueParser::new("<b><i>bold italic</i></b>").collect();
  assert_eq!(tokens.len(), 5);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag { tag: Tag::Bold, .. }
  ));
  assert!(matches!(
    &tokens[1],
    CueToken::StartTag {
      tag: Tag::Italic,
      ..
    }
  ));
  assert_eq!(tokens[2], CueToken::Text(CueStr::borrowed("bold italic")));
  assert_eq!(tokens[3], CueToken::EndTag(Tag::Italic));
  assert_eq!(tokens[4], CueToken::EndTag(Tag::Bold));
}

#[test]
fn parse_empty_input() {
  let tokens: Vec<_> = CueParser::new("").collect();
  assert!(tokens.is_empty());
}

#[test]
fn parse_text_no_entities_not_normalized() {
  let tokens: Vec<_> = CueParser::new("just text").collect();
  if let CueToken::Text(cue_str) = &tokens[0] {
    assert!(!cue_str.requires_normalization());
  } else {
    panic!("expected text token");
  }
}

#[test]
fn parse_text_with_entities_requires_normalization() {
  let tokens: Vec<_> = CueParser::new("a&amp;b").collect();
  if let CueToken::Text(cue_str) = &tokens[0] {
    assert!(cue_str.requires_normalization());
    assert_eq!(cue_str.as_raw(), "a&amp;b");
    assert_eq!(cue_str.normalize(), "a&b");
  } else {
    panic!("expected text token");
  }
}

#[test]
fn parse_null_requires_normalization() {
  let tokens: Vec<_> = CueParser::new("hello\0world").collect();
  if let CueToken::Text(cue_str) = &tokens[0] {
    assert!(cue_str.requires_normalization());
    assert_eq!(cue_str.normalize(), "hello\u{FFFD}world");
  } else {
    panic!("expected text token");
  }
}

#[test]
fn normalize_is_lazy_and_cached() {
  let tokens: Vec<_> = CueParser::new("a&amp;b").collect();
  if let CueToken::Text(cue_str) = &tokens[0] {
    // First call computes the normalized form
    let first = cue_str.normalize();
    // Second call returns the same cached reference
    let second = cue_str.normalize();
    assert!(core::ptr::eq(first, second));
  } else {
    panic!("expected text token");
  }
}

// ── CueText (DOM tree) tests ────────────────────────────────────────────────

#[test]
fn tree_plain_text() {
  let tree = CueText::parse("hello");
  assert_eq!(tree.children().len(), 1);
  assert!(matches!(&tree.children()[0], Node::Text(t) if t.normalize() == "hello"));
}

#[test]
fn tree_bold_text() {
  let tree = CueText::parse("<b>bold</b>");
  assert_eq!(tree.children().len(), 1);
  match &tree.children()[0] {
    Node::Tag(tag) => {
      assert_eq!(tag.tag(), Tag::Bold);
      assert_eq!(tag.children().len(), 1);
      assert!(matches!(&tag.children()[0], Node::Text(t) if t.normalize() == "bold"));
    }
    _ => panic!("expected tag node"),
  }
}

#[test]
fn tree_nested_tags() {
  let tree = CueText::parse("<b><i>text</i></b>");
  assert_eq!(tree.children().len(), 1);
  match &tree.children()[0] {
    Node::Tag(outer) => {
      assert_eq!(outer.tag(), Tag::Bold);
      assert_eq!(outer.children().len(), 1);
      match &outer.children()[0] {
        Node::Tag(inner) => {
          assert_eq!(inner.tag(), Tag::Italic);
          assert_eq!(inner.children().len(), 1);
        }
        _ => panic!("expected inner tag"),
      }
    }
    _ => panic!("expected outer tag"),
  }
}

#[test]
fn tree_mixed_text_and_tags() {
  let tree = CueText::parse("before <b>bold</b> after");
  assert_eq!(tree.children().len(), 3);
  assert!(matches!(&tree.children()[0], Node::Text(t) if t.normalize() == "before "));
  assert!(matches!(
    &tree.children()[1],
    Node::Tag(t) if t.tag() == Tag::Bold
  ));
  assert!(matches!(&tree.children()[2], Node::Text(t) if t.normalize() == " after"));
}

#[test]
fn tree_unclosed_tag() {
  let tree = CueText::parse("<b>unclosed");
  assert_eq!(tree.children().len(), 1);
  match &tree.children()[0] {
    Node::Tag(tag) => {
      assert_eq!(tag.tag(), Tag::Bold);
      assert_eq!(tag.children().len(), 1);
    }
    _ => panic!("expected tag node"),
  }
}

#[test]
fn tree_with_timestamp() {
  let tree = CueText::parse("text<00:05.000>more");
  assert_eq!(tree.children().len(), 3);
  assert!(matches!(&tree.children()[0], Node::Text(_)));
  assert!(matches!(&tree.children()[1], Node::Timestamp(_)));
  assert!(matches!(&tree.children()[2], Node::Text(_)));
}

#[test]
fn tree_voice_with_annotation() {
  let tree = CueText::parse("<v Speaker>dialogue</v>");
  assert_eq!(tree.children().len(), 1);
  match &tree.children()[0] {
    Node::Tag(tag) => {
      assert_eq!(tag.tag(), Tag::Voice);
      assert_eq!(tag.annotation(), Some("Speaker"));
      assert_eq!(tag.children().len(), 1);
    }
    _ => panic!("expected tag"),
  }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

trait CueTokenExt {
  fn as_raw_text(&self) -> Option<&str>;
  fn as_normalized_text(&self) -> Option<&str>;
  fn requires_normalization(&self) -> bool;
}

impl CueTokenExt for CueToken<'_> {
  fn as_raw_text(&self) -> Option<&str> {
    match self {
      CueToken::Text(s) => Some(s.as_raw()),
      _ => None,
    }
  }
  fn as_normalized_text(&self) -> Option<&str> {
    match self {
      CueToken::Text(s) => Some(s.normalize()),
      _ => None,
    }
  }
  fn requires_normalization(&self) -> bool {
    match self {
      CueToken::Text(s) => s.requires_normalization(),
      _ => false,
    }
  }
}

// ── Numeric character reference tests ─────────────────────────────────────────

#[test]
fn parse_numeric_char_ref_decimal() {
  let tokens: Vec<_> = CueParser::new("&#65;").collect();
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "A");
}

#[test]
fn parse_numeric_char_ref_hex() {
  let tokens: Vec<_> = CueParser::new("&#x41;").collect();
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "A");
}

#[test]
fn parse_numeric_char_ref_hex_uppercase() {
  let tokens: Vec<_> = CueParser::new("&#X42;").collect();
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "B");
}

#[test]
fn parse_numeric_char_ref_null_replaced() {
  let tokens: Vec<_> = CueParser::new("&#0;").collect();
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "\u{FFFD}");
}

#[test]
fn parse_numeric_char_ref_invalid_codepoint() {
  // U+FFFFFF is not a valid Unicode codepoint
  let tokens: Vec<_> = CueParser::new("&#xFFFFFF;").collect();
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "\u{FFFD}");
}

#[test]
fn parse_numeric_char_ref_no_digits() {
  let tokens: Vec<_> = CueParser::new("&#;").collect();
  let text = tokens[0].as_normalized_text().unwrap();
  assert!(text.contains("&#"));
}

#[test]
fn parse_numeric_char_ref_hex_no_digits() {
  let tokens: Vec<_> = CueParser::new("&#x;").collect();
  let text = tokens[0].as_normalized_text().unwrap();
  assert!(text.contains("&#x"));
}

#[test]
fn parse_ampersand_followed_by_non_alnum() {
  let tokens: Vec<_> = CueParser::new("a&!b").collect();
  assert_eq!(tokens[0].as_normalized_text().unwrap(), "a&!b");
}

#[test]
fn parse_trailing_ampersand_hash() {
  let tokens: Vec<_> = CueParser::new("a&#").collect();
  let text = tokens[0].as_normalized_text().unwrap();
  assert!(text.contains("&#"));
}

#[test]
fn parse_numeric_ref_without_semicolon() {
  let tokens: Vec<_> = CueParser::new("&#65x").collect();
  let text = tokens[0].as_normalized_text().unwrap();
  // Should decode &#65 as 'A' and output 'x'
  assert!(text.contains('A'));
}

// ── CueStr Clone and Debug tests ─────────────────────────────────────────────

#[test]
fn cue_str_clone() {
  let tokens: Vec<_> = CueParser::new("a&amp;b").collect();
  if let CueToken::Text(cue_str) = &tokens[0] {
    let cloned = cue_str.clone();
    assert_eq!(cloned.as_raw(), cue_str.as_raw());
    assert_eq!(
      cloned.requires_normalization(),
      cue_str.requires_normalization()
    );
  }
}

#[test]
fn cue_str_debug() {
  let tokens: Vec<_> = CueParser::new("test").collect();
  if let CueToken::Text(cue_str) = &tokens[0] {
    let debug = format!("{:?}", cue_str);
    assert!(debug.contains("CueStr"));
    assert!(debug.contains("test"));
  }
}

// ── Unterminated tag tests ────────────────────────────────────────────────────

#[test]
fn parse_unterminated_bold() {
  let tokens: Vec<_> = CueParser::new("<b").collect();
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag { tag: Tag::Bold, .. }
  ));
}

#[test]
fn parse_unterminated_italic_with_class() {
  let tokens: Vec<_> = CueParser::new("<i.highlight").collect();
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Italic,
      ..
    }
  ));
}

#[test]
fn parse_unterminated_voice() {
  let tokens: Vec<_> = CueParser::new("<v Speaker").collect();
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Voice,
      annotation: Some("Speaker"),
      ..
    }
  ));
}

#[test]
fn parse_unterminated_ruby() {
  let tokens: Vec<_> = CueParser::new("<ruby").collect();
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag { tag: Tag::Ruby, .. }
  ));
}

#[test]
fn parse_unterminated_rt() {
  let tokens: Vec<_> = CueParser::new("<ruby><rt").collect();
  assert_eq!(tokens.len(), 2);
  assert!(matches!(
    &tokens[1],
    CueToken::StartTag {
      tag: Tag::RubyText,
      ..
    }
  ));
}

#[test]
fn parse_unterminated_lang() {
  let tokens: Vec<_> = CueParser::new("<lang en").collect();
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag {
      tag: Tag::Lang,
      annotation: Some("en"),
      ..
    }
  ));
}

#[test]
fn parse_unterminated_unknown() {
  let tokens: Vec<_> = CueParser::new("<xyz").collect();
  assert!(tokens.is_empty());
}

#[test]
fn parse_unterminated_empty() {
  let tokens: Vec<_> = CueParser::new("<").collect();
  assert!(tokens.is_empty());
}

#[test]
fn parse_unterminated_ruby_wrong_char() {
  let tokens: Vec<_> = CueParser::new("<rubyX").collect();
  assert!(tokens.is_empty());
}

#[test]
fn parse_unterminated_rt_wrong_char() {
  let tokens: Vec<_> = CueParser::new("<rtX").collect();
  assert!(tokens.is_empty());
}

#[test]
fn parse_unterminated_lang_wrong_char() {
  let tokens: Vec<_> = CueParser::new("<langX").collect();
  assert!(tokens.is_empty());
}

// ── CueText DOM tree edge cases ──────────────────────────────────────────────

#[test]
fn tree_rt_outside_ruby_ignored() {
  let tree = CueText::parse("<rt>text</rt>");
  // <rt> outside <ruby> should be ignored per spec
  assert_eq!(tree.children().len(), 1);
  assert!(matches!(&tree.children()[0], Node::Text(t) if t.normalize() == "text"));
}

#[test]
fn tree_end_rt_outside_ruby_ignored() {
  let tree = CueText::parse("text</rt>more");
  // </rt> outside <ruby> should be ignored, text nodes are separate
  assert_eq!(tree.children().len(), 2);
  assert!(matches!(&tree.children()[0], Node::Text(t) if t.normalize() == "text"));
  assert!(matches!(&tree.children()[1], Node::Text(t) if t.normalize() == "more"));
}

#[test]
fn tree_into_children() {
  let tree = CueText::parse("hello");
  let children = tree.into_children();
  assert_eq!(children.len(), 1);
}

// ── W3C §6.4 ruby conformance ────────────────────────────────────────────────
//
// §6.4 scopes `<rt>` on the *current* node ("If current is a WebVTT Ruby
// Object, then attach a WebVTT Ruby Text Object"), and closes an open `<rt>`
// from one end tag only — `</ruby>`, which closes the `<ruby>` along with it.
// Every ruby case in the shipped WPT `tree-building` fixtures puts `<rt>`
// directly inside `<ruby>`, where an ancestor reading of the same rules agrees;
// these fixtures are the cases that discriminate.

/// Whether any node in the tree carries the given tag. Walked iteratively, so
/// the assertion is not itself a recursive walk of the tree under test.
fn contains_tag(tree: &CueText<'_>, wanted: Tag) -> bool {
  let mut work: Vec<&Node<'_>> = tree.children().iter().collect();
  while let Some(node) = work.pop() {
    if let Node::Tag(tag) = node {
      if tag.tag() == wanted {
        return true;
      }
      work.extend(tag.children());
    }
  }
  false
}

/// `<rt>` attaches only while `current` is the `<ruby>` itself. An intervening
/// `<b>` makes it a token §6.4 ignores, so its text lands in the `<b>` and no
/// ruby text node is built at all.
#[test]
fn rt_attaches_only_when_current_is_ruby() {
  let tree = CueText::parse("<ruby><b><rt>x</rt></b></ruby>");

  assert_eq!(tree.to_string(), "<ruby><b>x</b></ruby>");
  assert_eq!(tag_depth(&tree), 2);
  assert!(
    !contains_tag(&tree, Tag::RubyText),
    "an <rt> whose current node is <b> must not be attached"
  );
}

/// A second `<rt>` inside an open `<rt>` is ignored for the same reason:
/// `current` is the ruby text object, not a ruby object.
#[test]
fn rt_inside_rt_is_ignored() {
  let tree = CueText::parse("<ruby><rt>a<rt>b</ruby>");

  assert_eq!(tree.to_string(), "<ruby><rt>ab</rt></ruby>");
  assert_eq!(tag_depth(&tree), 2);
}

/// An end tag that does not name the current node is ignored — it does not
/// close an open `<rt>` on the way. What follows it stays inside the `<rt>`,
/// one level deeper than a drain-before-every-end-tag reading builds.
#[test]
fn an_unmatched_end_tag_does_not_close_an_open_rt() {
  let tree = CueText::parse("<ruby><rt>x</b><i>y");

  assert_eq!(tree.to_string(), "<ruby><rt>x<i>y</i></rt></ruby>");
  assert_eq!(tag_depth(&tree), 3);

  let ruby = match &tree.children()[0] {
    Node::Tag(node) => node,
    other => panic!("expected a <ruby> node, got {other:?}"),
  };
  assert_eq!(
    ruby.children().len(),
    1,
    "</b> must not have closed the <rt> and made <i> its sibling"
  );
}

/// `</ruby>` is the one end tag that closes a node it does not name: with an
/// `<rt>` open it closes both, and `current` returns to the ruby's parent.
#[test]
fn end_ruby_closes_an_open_rt_and_the_ruby() {
  let tree = CueText::parse("<ruby><rt>x</ruby>y");

  assert_eq!(tree.to_string(), "<ruby><rt>x</rt></ruby>y");
  assert_eq!(tree.children().len(), 2);
  assert!(matches!(&tree.children()[1], Node::Text(t) if t.normalize() == "y"));
}

/// §6.4's end tag step in full: an end tag closes the current node when it
/// names it (the seven listed pairs, plus the "lang" clause that follows
/// them), `</ruby>` also closes an open `<rt>` together with its `<ruby>`, and
/// every other end tag is ignored.
#[test]
fn end_tag_branch_table() {
  const CASES: &[(&str, &str)] = &[
    // The end tag names the current node.
    ("<c>x</c>y", "<c>x</c>y"),
    ("<i>x</i>y", "<i>x</i>y"),
    ("<b>x</b>y", "<b>x</b>y"),
    ("<u>x</u>y", "<u>x</u>y"),
    ("<ruby>x</ruby>y", "<ruby>x</ruby>y"),
    ("<ruby><rt>x</rt>y</ruby>", "<ruby><rt>x</rt>y</ruby>"),
    ("<v a>x</v>y", "<v a>x</v>y"),
    ("<lang en>x</lang>y", "<lang en>x</lang>y"),
    // "ruby" while current is a Ruby Text Object: closes both.
    ("<ruby><rt>x</ruby>y", "<ruby><rt>x</rt></ruby>y"),
    // Otherwise, ignore the token — an open <rt> survives every other end tag.
    ("<ruby><rt>x</b>y</ruby>", "<ruby><rt>xy</rt></ruby>"),
    ("<ruby><rt>x</i>y</ruby>", "<ruby><rt>xy</rt></ruby>"),
    ("<ruby><rt>x</u>y</ruby>", "<ruby><rt>xy</rt></ruby>"),
    ("<ruby><rt>x</c>y</ruby>", "<ruby><rt>xy</rt></ruby>"),
    ("<ruby><rt>x</v>y</ruby>", "<ruby><rt>xy</rt></ruby>"),
    ("<ruby><rt>x</lang>y</ruby>", "<ruby><rt>xy</rt></ruby>"),
    // …including an end tag with nothing of its name open.
    ("stray </b> end tag", "stray  end tag"),
    ("<ruby></rt>x</ruby>", "<ruby>x</ruby>"),
  ];

  for (input, expected) in CASES {
    assert_eq!(
      CueText::parse(input).to_string(),
      *expected,
      "input: {input:?}"
    );
  }
}

// ── Malformed timestamp rejection tests ──────────────────────────────────────
//
// These verify that malformed cue-text timestamp tags are safely rejected
// (treated as unknown tags or skipped) without panicking, even in debug builds.

/// Colons where digits are expected: `<:::.000>` matches the old loose regex
/// but is rejected by the tightened DFA.
#[test]
fn cue_text_rejects_colons_as_digits() {
  let tokens: Vec<_> = CueParser::new("<:::.000>").collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "colons-only should not parse as timestamp"
  );
}

/// Minutes out of range: `<99:99.000>` — rejected by the DFA (`[0-5][0-9]`).
#[test]
fn cue_text_rejects_out_of_range_minutes() {
  let tokens: Vec<_> = CueParser::new("<99:99.000>").collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "99:99 should not parse as timestamp"
  );
}

/// Seconds out of range: `<00:60.000>` — rejected by the DFA.
#[test]
fn cue_text_rejects_out_of_range_seconds() {
  let tokens: Vec<_> = CueParser::new("<00:60.000>").collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "60 seconds should not parse as timestamp"
  );
}

/// Non-digit bytes in hour position: `<ab:00:00.000>`.
#[test]
fn cue_text_rejects_non_digit_hours() {
  let tokens: Vec<_> = CueParser::new("<ab:00:00.000>").collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "non-digit hours should not parse as timestamp"
  );
}

/// Empty hour prefix: `<:00:00.000>` — colon where a digit is expected.
#[test]
fn cue_text_rejects_empty_hour_prefix() {
  let tokens: Vec<_> = CueParser::new("<:00:00.000>").collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "empty hour prefix should not parse as timestamp"
  );
}

/// Very large hours that would overflow u64: 30-digit hour value.
#[test]
fn cue_text_rejects_overflowing_hours() {
  // 30 digits exceeds u64::MAX (20 digits)
  let tag = format!("<{}:00:00.000>", "9".repeat(30));
  let tokens: Vec<_> = CueParser::new(&tag).collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "overflowing hours should not parse as timestamp"
  );
}

/// Unterminated timestamp tag with valid format goes through `parse_timestamp_cue`.
#[test]
fn cue_text_unterminated_valid_timestamp() {
  // `<00:05.000` without closing `>` — handled by try_parse_unterminated
  let tokens: Vec<_> = CueParser::new("<00:05.000").collect();
  assert!(
    tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "unterminated but valid timestamp should parse"
  );
}

/// Unterminated timestamp with invalid digits goes through `parse_timestamp_cue`
/// and is safely rejected.
#[test]
fn cue_text_unterminated_invalid_timestamp() {
  let tokens: Vec<_> = CueParser::new("<99:99.000").collect();
  assert!(
    !tokens.iter().any(|t| matches!(t, CueToken::Timestamp(_))),
    "unterminated invalid timestamp should be rejected"
  );
}

// ── Nesting depth tests ──────────────────────────────────────────────────────

/// The deepest run of nested tags in the tree.
///
/// Walked iteratively: these tests feed the parser payloads that would abort
/// the test binary if anything walked the result recursively without a bound.
fn tag_depth(tree: &CueText<'_>) -> usize {
  let mut deepest = 0;
  let mut work: Vec<(&Node<'_>, usize)> = tree.children().iter().map(|n| (n, 0)).collect();
  while let Some((node, depth)) = work.pop() {
    if let Node::Tag(tag) = node {
      deepest = deepest.max(depth + 1);
      work.extend(tag.children().iter().map(|child| (child, depth + 1)));
    }
  }
  deepest
}

/// The tree's text in document order, with markup removed.
fn tree_text(tree: &CueText<'_>) -> String {
  let mut out = String::new();
  let mut work: Vec<&Node<'_>> = tree.children().iter().rev().collect();
  while let Some(node) = work.pop() {
    match node {
      Node::Text(text) => out.push_str(text.normalize()),
      Node::Timestamp(_) => {}
      Node::Tag(tag) => work.extend(tag.children().iter().rev()),
    }
  }
  out
}

/// A cue payload nested far past anything a renderer would draw.
///
/// Depth 20 000 is the fixture that aborted a test binary against 0.3.0: the
/// tree it built was walked recursively by drop glue, `Display`, `Debug`,
/// `Clone` and `PartialEq`, and a stack overflow is an abort rather than a
/// catchable panic. Every one of those five walks is exercised here in
/// process, which is only safe because the tree is bounded.
/// Skipped under Miri: interpreting 40 000 tokens costs minutes there, and an
/// interpreter has no host stack to exhaust, so the walk this fixture is here
/// to bound is not the walk Miri would be checking.
#[cfg_attr(
  miri,
  ignore = "20 000-token fixture, and Miri cannot overflow the host stack"
)]
#[test]
fn deep_nesting_is_safe_to_build_walk_and_drop() {
  let depth = 20_000;
  let payload = format!("{}words{}", "<i>".repeat(depth), "</i>".repeat(depth));

  let tree = CueText::parse(&payload);

  assert_eq!(tag_depth(&tree), DEFAULT_MAX_DEPTH);
  assert_eq!(tree_text(&tree), "words");

  let rendered = tree.to_string();
  assert!(rendered.starts_with("<i><i>"));
  assert!(rendered.contains("words"));

  let debugged = format!("{tree:?}");
  assert!(debugged.starts_with("CueText"));

  let cloned = tree.clone();
  assert_eq!(cloned, tree);

  drop(cloned);
  drop(tree);
}

/// Unclosed nesting is bounded too — the parser folds the open tags back into
/// their parents at end of input, and that fold must not rebuild a deep tree.
/// Skipped under Miri: interpreting 40 000 tokens costs minutes there, and an
/// interpreter has no host stack to exhaust, so the walk this fixture is here
/// to bound is not the walk Miri would be checking.
#[cfg_attr(
  miri,
  ignore = "20 000-token fixture, and Miri cannot overflow the host stack"
)]
#[test]
fn deep_unclosed_nesting_is_bounded() {
  let payload = "<i>".repeat(20_000);
  let tree = CueText::parse(&payload);

  assert_eq!(tag_depth(&tree), DEFAULT_MAX_DEPTH);
  assert_eq!(tree.children().len(), 1);
}

/// Skipped under Miri: interpreting 40 000 tokens costs minutes there, and an
/// interpreter has no host stack to exhaust, so the walk this fixture is here
/// to bound is not the walk Miri would be checking.
#[cfg_attr(
  miri,
  ignore = "20 000-token fixture, and Miri cannot overflow the host stack"
)]
#[test]
fn try_parse_refuses_deep_nesting() {
  let err = CueText::try_parse(&"<i>".repeat(20_000)).unwrap_err();
  assert_eq!(err.max_depth(), DEFAULT_MAX_DEPTH);
}

#[test]
fn try_parse_accepts_input_at_the_limit() {
  let depth = DEFAULT_MAX_DEPTH;
  let payload = format!("{}words{}", "<i>".repeat(depth), "</i>".repeat(depth));

  let tree = CueText::try_parse(&payload).expect("input at the limit is accepted");
  assert_eq!(tag_depth(&tree), depth);

  let one_deeper = format!(
    "{}words{}",
    "<i>".repeat(depth + 1),
    "</i>".repeat(depth + 1)
  );
  assert!(CueText::try_parse(&one_deeper).is_err());
}

/// Past the limit the markup goes, the text stays.
#[test]
fn depth_limit_keeps_the_cue_text() {
  let opts = Options::new().with_max_depth(1);
  let tree = CueText::parse_with("<b>one<i>two<u>three</u></i></b>", opts);

  assert_eq!(tag_depth(&tree), 1);
  assert_eq!(tree_text(&tree), "onetwothree");
  assert_eq!(tree.to_string(), "<b>onetwothree</b>");
}

/// An over-deep run consumes its own end tags, so the structure that follows it
/// is the structure an unbounded parse would have produced.
#[test]
fn depth_limit_preserves_structure_after_the_deep_run() {
  let opts = Options::new().with_max_depth(1);
  let tree = CueText::parse_with("<b><i><i><i>x</i></i></i></b><u>tail</u>", opts);

  assert_eq!(tree.to_string(), "<b>x</b><u>tail</u>");
  assert_eq!(tag_depth(&tree), 1);
}

#[test]
fn max_depth_zero_drops_every_tag() {
  let opts = Options::new().with_max_depth(0);
  let tree = CueText::parse_with("<b>bold</b> and <i>italic</i>", opts);

  assert_eq!(tag_depth(&tree), 0);
  assert_eq!(tree_text(&tree), "bold and italic");
}

/// `<rt>` attaches only while `current` is a `<ruby>`, and that test has to
/// see a `<ruby>` that was itself pushed past the limit — an over-deep tag
/// materializes no node but is still the current node.
#[test]
fn the_current_node_is_seen_past_the_limit() {
  let opts = Options::new().with_max_depth(1);
  let tree = CueText::parse_with("<b><ruby>base<rt>note</rt></ruby></b>", opts);

  assert_eq!(tree_text(&tree), "basenote");
  assert_eq!(tag_depth(&tree), 1);

  // An over-deep `<b>` is just as much the current node, so the `<rt>` inside
  // it is ignored past the limit exactly as it is within it.
  let scoped = CueText::parse_with("<ruby><b><rt>note</rt></b></ruby>", opts);
  assert_eq!(scoped.to_string(), "<ruby>note</ruby>");
  assert!(!contains_tag(&scoped, Tag::RubyText));

  // A bare <rt> with no <ruby> open is still dropped, at any depth.
  let bare = CueText::parse_with("<rt>note</rt>", opts);
  assert_eq!(bare.children().len(), 1);
  assert!(matches!(&bare.children()[0], Node::Text(_)));
}

/// A token §6.4 ignores costs no depth. Attaching `<rt>` on an ancestor test
/// spent a level of the budget on a node the spec never builds, so a cue whose
/// spec tree fits the limit could be refused for nesting it does not have.
#[test]
fn an_ignored_rt_costs_no_depth() {
  let opts = Options::new().with_max_depth(2);
  let tree = CueText::try_parse_with("<ruby><b><rt>x</rt></b></ruby>", opts)
    .expect("§6.4 builds this cue two deep, which is within the limit");

  assert_eq!(tree.to_string(), "<ruby><b>x</b></ruby>");
  assert_eq!(tag_depth(&tree), 2);
}

/// Everything shallower than the limit parses exactly as it did before the
/// limit existed, which a raised limit makes checkable directly.
#[test]
fn ordinary_cues_are_unchanged_by_the_limit() {
  const CUES: &[&str] = &[
    "plain text",
    "<b>bold</b> and <i>italic</i>",
    "<v Roger Bingham>voice</v>",
    "<c.loud.important>classy</c>",
    "<lang en>hello</lang>",
    "<ruby>base<rt>note</rt></ruby>",
    "<ruby>base<rt>one<rt>two</ruby>",
    "<ruby><b><rt>scoped on current</rt></b></ruby>",
    "<ruby><rt>survives</b><i>an unmatched end tag",
    "<b><i><u><c>four deep</c></u></i></b>",
    "unclosed <b>bold",
    "stray </i> end tag",
    "<rt>orphan rt</rt>",
    "<00:01.000>timestamped",
    "a&amp;b &lt;tag&gt;",
  ];

  let raised = Options::new().with_max_depth(usize::MAX);
  for cue in CUES {
    let bounded = CueText::parse(cue);
    let unbounded = CueText::parse_with(cue, raised);
    assert_eq!(bounded, unbounded, "cue: {cue:?}");
    assert_eq!(CueText::try_parse(cue).unwrap(), unbounded, "cue: {cue:?}");
  }
}

#[test]
fn options_default_matches_new() {
  assert_eq!(Options::default(), Options::new());
  assert_eq!(Options::default().max_depth(), DEFAULT_MAX_DEPTH);

  let mut opts = Options::new();
  opts.set_max_depth(8);
  assert_eq!(opts.max_depth(), 8);
  assert_eq!(opts, Options::new().with_max_depth(8));
}

// ── Small-stack walk tests ───────────────────────────────────────────────────

/// The stack a walk of a default-limit tree is held to.
///
/// Sixteen times smaller than the 2 MiB a Rust thread is given by default, and
/// roughly two and a half times the ~50 KiB the most expensive walk actually
/// needs in an unoptimized build — margin for other targets' frame sizes
/// without letting a regression through.
const SMALL_STACK: usize = 128 * 1024;

/// Names the walk a re-executed child process should perform.
const WALK_VAR: &str = "FASRT_CUE_TEXT_SMALL_STACK_WALK";

/// What the child prints once it has actually completed its walk.
const WALK_DONE: &str = "fasrt-small-stack-walk-completed";

const WALKS: &[&str] = &["build", "drop", "display", "debug", "clone", "eq"];

/// Every recursive walk of a tree at the default limit has to fit a small
/// worker thread.
///
/// Bounding the tree is only half the fix: the bound has to be small enough
/// that the walks it permits are affordable. A walk that overflows aborts, and
/// an abort would take the test binary with it, so each walk runs in a child
/// process — this same test, re-executed with the walk named in the
/// environment — on a thread with an explicitly small stack. The parent only
/// has to see the child exit cleanly.
#[cfg_attr(miri, ignore = "spawns child processes")]
#[test]
fn every_walk_at_the_default_limit_fits_a_small_stack() {
  if let Ok(walk) = std::env::var(WALK_VAR) {
    run_walk_on_small_stack(&walk);
    // The parent matches on this. A filter that selects no test still exits
    // zero, so without a witness on stdout a renamed test would leave the
    // parent asserting nothing at all.
    println!("{WALK_DONE} {walk}");
    return;
  }

  let exe = std::env::current_exe().expect("test binary path");
  for walk in WALKS {
    let output = std::process::Command::new(&exe)
      .args([
        "--exact",
        "every_walk_at_the_default_limit_fits_a_small_stack",
        "--nocapture",
      ])
      .env(WALK_VAR, walk)
      .output()
      .expect("re-exec the test binary");

    assert!(
      output.status.success(),
      "the {walk} walk of a tree at DEFAULT_MAX_DEPTH ({DEFAULT_MAX_DEPTH}) did not \
       survive a {SMALL_STACK}-byte stack: {}",
      output.status
    );
    assert!(
      String::from_utf8_lossy(&output.stdout).contains(&format!("{WALK_DONE} {walk}")),
      "the child never reported running the {walk} walk; stdout was:\n{}",
      String::from_utf8_lossy(&output.stdout)
    );
  }
}

fn run_walk_on_small_stack(walk: &str) {
  let depth = DEFAULT_MAX_DEPTH;
  let payload = format!("{}words{}", "<i>".repeat(depth), "</i>".repeat(depth));
  let walk = walk.to_owned();

  std::thread::Builder::new()
    .stack_size(SMALL_STACK)
    .spawn(move || {
      let tree = CueText::parse(&payload);
      assert_eq!(tag_depth(&tree), depth);

      match walk.as_str() {
        // Construction is done; keeping the tree alive isolates it from drop.
        "build" => std::mem::forget(tree),
        "drop" => drop(tree),
        "display" => {
          assert!(tree.to_string().contains("words"));
          std::mem::forget(tree);
        }
        "debug" => {
          assert!(format!("{tree:?}").starts_with("CueText"));
          std::mem::forget(tree);
        }
        "clone" => {
          let cloned = tree.clone();
          assert_eq!(tag_depth(&cloned), depth);
          std::mem::forget(cloned);
          std::mem::forget(tree);
        }
        "eq" => {
          let cloned = tree.clone();
          assert!(cloned == tree);
          std::mem::forget(cloned);
          std::mem::forget(tree);
        }
        other => panic!("unknown walk {other}"),
      }
    })
    .expect("spawn a small-stack thread")
    .join()
    .expect("the walk must not unwind");
}
