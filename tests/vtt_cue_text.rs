#![cfg(any(feature = "alloc", feature = "std"))]

use fasrt::vtt::cue::{
  Annotation, Classes, CueParser, CueStr, CueText, CueToken, DEFAULT_MAX_DEPTH, Node, Options, Tag,
  TagNode,
};

/// A token's annotation as W3C WebVTT §6.4's annotation state produces it.
fn token_annotation<'t>(token: &'t CueToken<'_>) -> Option<&'t str> {
  match token {
    CueToken::StartTag { annotation, .. } => annotation.as_ref().map(Annotation::normalize),
    other => panic!("expected a start tag, got {other:?}"),
  }
}

/// A token's annotation as the cue spelled it.
fn token_annotation_raw<'a>(token: &CueToken<'a>) -> Option<&'a str> {
  match token {
    CueToken::StartTag { annotation, .. } => annotation.as_ref().map(Annotation::as_raw),
    other => panic!("expected a start tag, got {other:?}"),
  }
}

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
      ..
    }
  ));
  assert_eq!(token_annotation(&tokens[0]), Some("Roger Bingham"));
}

#[test]
fn parse_lang_tag() {
  let tokens: Vec<_> = CueParser::new("<lang en>hello</lang>").collect();
  assert_eq!(tokens.len(), 3);
  assert!(matches!(
    &tokens[0],
    CueToken::StartTag { tag: Tag::Lang, .. }
  ));
  assert_eq!(token_annotation(&tokens[0]), Some("en"));
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
      assert_eq!(tag.annotation().map(Annotation::normalize), Some("Speaker"));
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
      ..
    }
  ));
  assert_eq!(token_annotation(&tokens[0]), Some("Speaker"));
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
    CueToken::StartTag { tag: Tag::Lang, .. }
  ));
  assert_eq!(token_annotation(&tokens[0]), Some("en"));
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

// ── W3C §6.4 class list conformance ──────────────────────────────────────────
//
// §6.4 attaches a node whose "list of applicable classes" is the start tag's
// classes "excluding any classes that are the empty string". The raw slice
// between the tag name and the annotation is *not* that list: it still carries
// the separators, so a consumer splitting it naively sees empty classes the
// spec says do not exist.

/// The tree's first node, which every fixture below builds as a tag.
fn first_tag<'t, 'a>(tree: &'t CueText<'a>) -> &'t TagNode<'a> {
  match &tree.children()[0] {
    Node::Tag(node) => node,
    other => panic!("expected a tag node, got {other:?}"),
  }
}

/// The issue's row: `<c.a..b>` has two classes, not one raw `"a..b"` that a
/// naive split reads as three.
#[test]
fn empty_classes_are_excluded_from_the_class_list() {
  let tree = CueText::parse("<c.a..b>x</c>");
  let node = first_tag(&tree);

  assert_eq!(node.classes().collect::<Vec<_>>(), ["a", "b"]);

  // The raw slice is still the source text, separators and all — which is
  // exactly why splitting it is the trap this face closes.
  assert_eq!(node.classes_raw(), "a..b");
  assert_eq!(node.classes_raw().split('.').count(), 3);
}

/// Every dot shape, read off a real parse: U+002E is the only separator, and
/// an empty run between two of them contributes nothing.
#[test]
fn the_class_list_splits_on_full_stop_and_skips_empties() {
  const CASES: &[(&str, &[&str])] = &[
    ("<c>x</c>", &[]),
    ("<c.loud>x</c>", &["loud"]),
    ("<c.loud.important>x</c>", &["loud", "important"]),
    // The issue's shape, and its neighbour.
    ("<c.a..b>x</c>", &["a", "b"]),
    ("<c.a...b>x</c>", &["a", "b"]),
    // Leading, trailing, and nothing but dots.
    ("<c..a>x</c>", &["a"]),
    ("<c.a.>x</c>", &["a"]),
    ("<c.>x</c>", &[]),
    ("<c..>x</c>", &[]),
    ("<c...>x</c>", &[]),
    // Classes end where the annotation begins.
    ("<v.a..b Esme>x</v>", &["a", "b"]),
    ("<c.a.. b>x</c>", &["a"]),
    // A class name is arbitrary text, not ASCII.
    ("<c.日本語..b>x</c>", &["日本語", "b"]),
    // Every tag that can carry classes reads them the same way.
    ("<b.a..b>x</b>", &["a", "b"]),
    ("<i.a..b>x</i>", &["a", "b"]),
    ("<u.a..b>x</u>", &["a", "b"]),
    ("<ruby.a..b>x</ruby>", &["a", "b"]),
    ("<lang.a..b en>x</lang>", &["a", "b"]),
  ];

  for (input, expected) in CASES {
    let tree = CueText::parse(input);
    let classes: Vec<_> = first_tag(&tree).classes().collect();
    assert_eq!(classes, *expected, "input: {input:?}");
  }
}

/// The list is derived, never stored, so it reads the same off a hand-built
/// node — and a cloned iterator re-reads it from the start.
#[test]
fn the_class_list_is_derived_from_the_raw_slice() {
  let node = TagNode::new(Tag::Class).with_classes(".a..b.");
  let classes = node.classes();

  assert_eq!(classes.clone().collect::<Vec<_>>(), ["a", "b"]);
  assert_eq!(classes.count(), 2);
  assert_eq!(node.classes_raw(), ".a..b.");

  let mut node = TagNode::new(Tag::Class);
  node.set_classes("..");
  assert_eq!(node.classes().next(), None);
  assert_eq!(node.classes_raw(), "..");
}

/// A token-level consumer never builds a tree, so the same face is reachable
/// straight from a start tag's raw class list.
#[test]
fn a_start_tag_tokens_classes_read_the_same() {
  let tokens: Vec<_> = CueParser::new("<c.a..b>x</c>").collect();
  let CueToken::StartTag { classes, .. } = &tokens[0] else {
    panic!("expected a start tag, got {:?}", tokens[0]);
  };

  assert_eq!(*classes, "a..b");
  assert_eq!(Classes::new(classes).collect::<Vec<_>>(), ["a", "b"]);
}

/// Excluding the empty classes does not rewrite the cue: the node keeps the
/// raw list, so serializing the tree writes back what was read.
#[test]
fn the_raw_class_list_survives_a_round_trip() {
  const CASES: &[&str] = &[
    "<c.a..b>x</c>",
    "<c..a>x</c>",
    "<c.a.>x</c>",
    "<v.a..b Esme>x</v>",
  ];

  for input in CASES {
    assert_eq!(
      CueText::parse(input).to_string(),
      *input,
      "input: {input:?}"
    );
  }

  // A list that is nothing but separators is the one shape whose dots do not
  // survive: the tokenizer keeps no class text at all for `<c.>`.
  assert_eq!(CueText::parse("<c.>x</c>").to_string(), "<c>x</c>");
}

// ── W3C §6.4 applicable language conformance ─────────────────────────────────
//
// §6.4 keeps a language stack: `<lang>` pushes its annotation before it
// attaches, `</lang>` pops when it closes a `<lang>`, and every attached node
// is stamped with the top entry. That stack is exactly the chain of enclosing
// `<lang>` nodes, so the language is derived from the tree rather than stored
// on every node — where an edit through `children_mut` could leave it stale.

/// Each node paired with its applicable language, in document order, with tag
/// nodes named and text nodes quoted.
fn languages(tree: &CueText<'_>) -> Vec<(String, Option<String>)> {
  tree
    .nodes_with_language()
    .map(|(node, language)| {
      let name = match node {
        Node::Text(text) => format!("{:?}", text.normalize()),
        Node::Timestamp(_) => "<timestamp>".to_owned(),
        Node::Tag(tag) => tag.tag().to_string(),
      };
      (name, language.map(str::to_owned))
    })
    .collect()
}

/// A language the walk reports as applicable, for the tables below.
fn lang(language: &str) -> Option<String> {
  Some(language.to_owned())
}

/// `<lang>` is the only tag that pushes, and it pushes its annotation.
#[test]
fn declared_language_is_the_language_stack_push() {
  assert_eq!(
    TagNode::new(Tag::Lang)
      .with_annotation(Some(Annotation::new("en")))
      .declared_language(),
    Some("en")
  );

  // §6.4 pushes the annotation whatever it is, and an absent one is the empty
  // string — an entry that clears rather than one that inherits.
  assert_eq!(TagNode::new(Tag::Lang).declared_language(), Some(""));

  // No other tag touches the stack, `<v>`'s annotation included.
  for tag in [
    Tag::Bold,
    Tag::Italic,
    Tag::Underline,
    Tag::Class,
    Tag::Ruby,
  ] {
    assert_eq!(TagNode::new(tag).declared_language(), None, "tag: {tag}");
  }
  assert_eq!(
    TagNode::new(Tag::Voice)
      .with_annotation(Some(Annotation::new("Esme")))
      .declared_language(),
    None
  );
}

/// The applicable language is the nearest enclosing `<lang>`'s annotation, and
/// the empty string where none encloses the node. A `<lang>` node carries the
/// language it declares, because §6.4 pushes before it attaches.
#[test]
fn the_applicable_language_is_the_nearest_enclosing_lang() {
  let tree = CueText::parse("<lang en>a<b>b</b></lang>c");

  assert_eq!(
    languages(&tree),
    [
      ("lang".to_owned(), lang("en")),
      ("\"a\"".to_owned(), lang("en")),
      ("b".to_owned(), lang("en")),
      ("\"b\"".to_owned(), lang("en")),
      ("\"c\"".to_owned(), None),
    ]
  );
}

/// A nested `<lang>` shadows the one above it for its subtree only, and an
/// annotation-less `<lang>` shadows with the empty language rather than
/// letting the enclosing one through.
#[test]
fn a_nested_lang_shadows_the_one_above_it() {
  let nested = CueText::parse("<lang en>a<lang ja>b</lang>c</lang>");
  assert_eq!(
    languages(&nested),
    [
      ("lang".to_owned(), lang("en")),
      ("\"a\"".to_owned(), lang("en")),
      ("lang".to_owned(), lang("ja")),
      ("\"b\"".to_owned(), lang("ja")),
      ("\"c\"".to_owned(), lang("en")),
    ]
  );

  let cleared = CueText::parse("<lang en>a<lang>b</lang>c</lang>");
  assert_eq!(
    languages(&cleared),
    [
      ("lang".to_owned(), lang("en")),
      ("\"a\"".to_owned(), lang("en")),
      // An empty *push*, not an empty stack.
      ("lang".to_owned(), lang("")),
      ("\"b\"".to_owned(), lang("")),
      ("\"c\"".to_owned(), lang("en")),
    ]
  );
}

/// `None` and `Some("")` are different answers. `None` is §6.4's empty
/// language stack — nothing in the cue speaks to this node's language, so a
/// fallback from outside the cue applies to it. `Some("")` is an
/// annotation-less `<lang>`, which pushed the empty string and thereby said
/// the subtree is in no known language, clearing that fallback rather than
/// deferring to it.
#[test]
fn an_empty_language_is_not_the_absence_of_one() {
  let tree = CueText::parse("<b>x</b><lang><b>y</b></lang>");

  assert_eq!(
    languages(&tree),
    [
      ("b".to_owned(), None),
      ("\"x\"".to_owned(), None),
      ("lang".to_owned(), lang("")),
      ("b".to_owned(), lang("")),
      ("\"y\"".to_owned(), lang("")),
    ]
  );

  // The one-step accessor draws the same line, which is what lets the walk.
  assert_eq!(TagNode::new(Tag::Bold).declared_language(), None);
  assert_eq!(TagNode::new(Tag::Lang).declared_language(), Some(""));
}

/// The stack and the tree cannot drift apart, because §6.4 only ever pops when
/// it closes a `<lang>` node. An end tag it ignores pops nothing, and the one
/// end tag that closes a node it does not name — `</ruby>` over an open `<rt>`
/// — closes no Language Object either.
#[test]
fn only_closing_a_lang_ends_its_scope() {
  // `</lang>` while current is the `<b>`: §6.4 ignores it, so the `<lang>`
  // stays open and everything after it is still in `en`.
  let ignored = CueText::parse("<lang en><b></lang>x");
  assert_eq!(ignored.to_string(), "<lang en><b>x</b></lang>");
  assert_eq!(
    languages(&ignored),
    [
      ("lang".to_owned(), lang("en")),
      ("b".to_owned(), lang("en")),
      ("\"x\"".to_owned(), lang("en")),
    ]
  );

  // The ruby double-close crosses two nodes at once; neither is a `<lang>`.
  let ruby = CueText::parse("<lang en><ruby>a<rt>b</ruby>c</lang>d");
  assert_eq!(
    ruby.to_string(),
    "<lang en><ruby>a<rt>b</rt></ruby>c</lang>d"
  );
  assert_eq!(
    languages(&ruby),
    [
      ("lang".to_owned(), lang("en")),
      ("ruby".to_owned(), lang("en")),
      ("\"a\"".to_owned(), lang("en")),
      ("rt".to_owned(), lang("en")),
      ("\"b\"".to_owned(), lang("en")),
      ("\"c\"".to_owned(), lang("en")),
      ("\"d\"".to_owned(), None),
    ]
  );

  // A `<lang>` left open at end of input scopes everything it opened.
  let unclosed = CueText::parse("<lang en>a<i>b");
  assert_eq!(
    languages(&unclosed),
    [
      ("lang".to_owned(), lang("en")),
      ("\"a\"".to_owned(), lang("en")),
      ("i".to_owned(), lang("en")),
      ("\"b\"".to_owned(), lang("en")),
    ]
  );
}

/// The walk answers for the tree it is given. A `<lang>` past
/// `Options::max_depth` is discarded exactly as an unrecognized tag is, and it
/// takes its scope with it along with its markup — so the text it covered
/// reads as no language at all. `try_parse` is the way to refuse such input
/// rather than accept a tree that dropped part of it.
#[test]
fn an_over_deep_lang_takes_its_scope_with_it() {
  let opts = Options::new().with_max_depth(1);
  let bounded = CueText::parse_with("<b><lang fr>x</lang>y</b>", opts);
  assert_eq!(bounded.to_string(), "<b>xy</b>");
  assert_eq!(
    languages(&bounded),
    [
      ("b".to_owned(), None),
      ("\"x\"".to_owned(), None),
      ("\"y\"".to_owned(), None),
    ]
  );
  assert!(CueText::try_parse_with("<b><lang fr>x</lang>y</b>", opts).is_err());

  // Within the limit, the same cue keeps the scope.
  let within = CueText::parse_with(
    "<b><lang fr>x</lang>y</b>",
    Options::new().with_max_depth(2),
  );
  assert_eq!(
    languages(&within),
    [
      ("b".to_owned(), None),
      ("lang".to_owned(), lang("fr")),
      ("\"x\"".to_owned(), lang("fr")),
      ("\"y\"".to_owned(), None),
    ]
  );

  // `max_depth` zero keeps the text and no tag at all.
  let flat = CueText::parse_with("<lang fr>x</lang>y", Options::new().with_max_depth(0));
  assert_eq!(flat.to_string(), "xy");
  assert!(languages(&flat).iter().all(|(_, lang)| lang.is_none()));

  // And the same at the default limit's boundary, where the depth bound is
  // holding back a cue that nests past it.
  let deep = format!("{}<lang fr>x</lang>y", "<i>".repeat(DEFAULT_MAX_DEPTH));
  let at_boundary = CueText::parse(&deep);
  assert!(
    languages(&at_boundary)
      .iter()
      .all(|(_, lang)| lang.is_none()),
    "the over-deep <lang> is not in the tree, so neither is its scope"
  );
  assert!(CueText::try_parse(&deep).is_err());
}

/// The scope is §6.4's, and so now is the *value*: what a `<lang>` pushes is
/// the annotation its annotation state produced, with character references
/// decoded and whitespace runs collapsed. This fixture was the contract of
/// absence — it pinned `"en&#x2D;US"` as the declared language while the
/// annotation was a slice borrowed from the cue — and now pins the behaviour
/// that replaced it.
#[test]
fn the_language_is_the_normalized_annotation() {
  let entity = CueText::parse("<lang en&#x2D;US>x</lang>");
  assert_eq!(
    languages(&entity),
    [
      ("lang".to_owned(), lang("en-US")),
      ("\"x\"".to_owned(), lang("en-US")),
    ]
  );

  let runs = CueText::parse("<lang en   US>x</lang>");
  assert_eq!(
    languages(&runs),
    [
      ("lang".to_owned(), lang("en US")),
      ("\"x\"".to_owned(), lang("en US")),
    ]
  );

  // The source text is still there, and is still what the node serializes
  // from — the two faces answer different questions and never disagree.
  let node = first_tag(&entity);
  assert_eq!(
    node.annotation().map(Annotation::as_raw),
    Some("en&#x2D;US")
  );
  assert_eq!(
    node.declared_language(),
    node.annotation().map(Annotation::normalize)
  );
  assert_eq!(entity.to_string(), "<lang en&#x2D;US>x</lang>");
}

/// The walk is document order — every node, each tag before its children, and
/// timestamps and text alike.
#[test]
fn the_language_walk_is_document_order() {
  let tree = CueText::parse("<b>1<i>2</i><00:01.000>3</b>4");

  assert_eq!(
    languages(&tree),
    [
      ("b".to_owned(), None),
      ("\"1\"".to_owned(), None),
      ("i".to_owned(), None),
      ("\"2\"".to_owned(), None),
      ("<timestamp>".to_owned(), None),
      ("\"3\"".to_owned(), None),
      ("\"4\"".to_owned(), None),
    ]
  );
  assert_eq!(tree.nodes_with_language().count(), 7);
}

/// The walk keeps its ancestors on the heap, so a tree deeper than a recursive
/// descent would care to be costs it no stack — and the language it carries
/// survives the whole descent.
#[test]
fn the_language_walk_costs_no_stack_in_the_depth_of_the_tree() {
  let depth = 256;
  let payload = format!(
    "<lang en>{}deep{}</lang>",
    "<i>".repeat(depth),
    "</i>".repeat(depth)
  );
  let tree = CueText::parse_with(&payload, Options::new().with_max_depth(depth + 1));

  let visited: Vec<_> = tree.nodes_with_language().collect();
  // One `<lang>`, `depth` italics and one text node — all of them in `en`.
  assert_eq!(visited.len(), depth + 2);
  assert!(visited.iter().all(|(_, language)| *language == Some("en")));
}

// ── W3C §6.4 tag delimiters ──────────────────────────────────────────────────
//
// §6.4's tokenizer leaves the tag-name and class-list states on any of four
// ASCII whitespace characters — TAB, LF, FF and SPACE — and enters the start
// tag annotation state. A cue payload spans lines, so an LF inside a tag is
// reachable input; recognizing only TAB and SPACE dropped such a tag entirely
// (taking a `<lang>` scope with it) and let an LF sit inside a class name.

/// The four delimiters §6.4 names, each of which ends a tag name.
const DELIMITERS: [char; 4] = ['\t', '\n', '\u{000C}', ' '];

#[test]
fn every_delimiter_ends_a_tag_name() {
  for delimiter in DELIMITERS {
    let input = format!("<lang{delimiter}en>x</lang>");
    let tokens: Vec<_> = CueParser::new(&input).collect();
    assert!(
      matches!(
        &tokens[0],
        CueToken::StartTag {
          tag: Tag::Lang,
          classes: "",
          ..
        }
      ),
      "delimiter {delimiter:?} gave {:?}",
      tokens.first()
    );
    assert_eq!(token_annotation(&tokens[0]), Some("en"), "{delimiter:?}");

    // And in the tree, where the tag's whole scope rides on it being seen.
    let tree = CueText::parse(&input);
    assert_eq!(
      languages(&tree),
      [
        ("lang".to_owned(), lang("en")),
        ("\"x\"".to_owned(), lang("en")),
      ],
      "delimiter {delimiter:?}"
    );
  }
}

#[test]
fn every_delimiter_ends_a_class_list() {
  for delimiter in DELIMITERS {
    let input = format!("<c.a..b{delimiter}note>x</c>");
    let tokens: Vec<_> = CueParser::new(&input).collect();
    assert!(
      matches!(
        &tokens[0],
        CueToken::StartTag {
          tag: Tag::Class,
          classes: "a..b",
          ..
        }
      ),
      "delimiter {delimiter:?} gave {:?}",
      tokens.first()
    );
    assert_eq!(token_annotation(&tokens[0]), Some("note"), "{delimiter:?}");

    let tree = CueText::parse(&input);
    let node = first_tag(&tree);
    assert_eq!(
      node.classes().collect::<Vec<_>>(),
      ["a", "b"],
      "delimiter {delimiter:?}"
    );
    assert_eq!(
      node.annotation().map(Annotation::normalize),
      Some("note"),
      "delimiter {delimiter:?}"
    );
  }
}

/// §6.4 trims the annotation over Infra's ASCII whitespace, not Unicode's set
/// — so a NO-BREAK SPACE is annotation text, not padding. That trim set is one
/// character wider than the delimiters above: CR ends no state, but it is
/// still ASCII whitespace and is still trimmed.
#[test]
fn the_annotation_is_trimmed_over_ascii_whitespace() {
  let tree = CueText::parse("<v \t\n\u{000C}\rEsme \t\n\u{000C}\r>x</v>");
  assert_eq!(
    first_tag(&tree).annotation().map(Annotation::as_raw),
    Some("Esme")
  );

  let kept = CueText::parse("<v \u{00A0}Esme\u{00A0}>x</v>");
  assert_eq!(
    first_tag(&kept).annotation().map(Annotation::normalize),
    Some("\u{00A0}Esme\u{00A0}")
  );

  // CR is trimmed but never delimits, so a language keeps none of it.
  let cr_padded = CueText::parse("<lang \ren\r>x</lang>");
  assert_eq!(first_tag(&cr_padded).declared_language(), Some("en"));
  assert!(
    languages(&cr_padded)
      .iter()
      .all(|(_, language)| *language == lang("en"))
  );

  // Whitespace-only leaves no annotation at all, CR-only included, and the
  // unterminated path answers the same way.
  for input in ["<v \t >x</v>", "<v \r>x</v>", "<v \r\n>x</v>"] {
    assert!(
      first_tag(&CueText::parse(input)).annotation().is_none(),
      "{input:?}"
    );
  }
  let unterminated: Vec<_> = CueParser::new("<v \rEsme\r").collect();
  assert_eq!(token_annotation_raw(&unterminated[0]), Some("Esme"));
}

/// A tag left unterminated at end of input is recognized by a separate path,
/// which reads the same delimiter set.
#[test]
fn every_delimiter_ends_a_name_in_an_unterminated_tag() {
  for delimiter in DELIMITERS {
    let lang_input = format!("<lang{delimiter}en");
    let lang: Vec<_> = CueParser::new(&lang_input).collect();
    assert!(
      matches!(&lang[0], CueToken::StartTag { tag: Tag::Lang, .. }),
      "delimiter {delimiter:?} gave {:?}",
      lang.first()
    );
    assert_eq!(token_annotation(&lang[0]), Some("en"), "{delimiter:?}");

    let class_input = format!("<c.a..b{delimiter}note");
    let class: Vec<_> = CueParser::new(&class_input).collect();
    assert!(
      matches!(
        &class[0],
        CueToken::StartTag {
          tag: Tag::Class,
          classes: "a..b",
          ..
        }
      ),
      "delimiter {delimiter:?} gave {:?}",
      class.first()
    );
    assert_eq!(token_annotation(&class[0]), Some("note"), "{delimiter:?}");
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

// ── §6.4's start tag annotation state ───────────────────────────────────────
//
// The state accumulates an annotation into a buffer, decoding the character
// references it meets on the way, and on the `>` that ends the tag removes
// leading and trailing ASCII whitespace and replaces every run of it within
// with a single U+0020 SPACE. The whole of that is `Annotation::normalize`;
// `Annotation::as_raw` is the text the cue spelled, which is what a node is
// serialized from.

/// The annotation of the tree's first tag, both faces.
fn annotation_of(input: &str) -> (Option<String>, Option<String>) {
  let tree = CueText::parse(input);
  let node = first_tag(&tree);
  (
    node.annotation().map(|a| a.normalize().to_owned()),
    node.annotation().map(|a| a.as_raw().to_owned()),
  )
}

/// §6.4's annotation state consumes character references through the same
/// "consume a character reference" algorithm the data state uses, so an
/// annotation decodes exactly as cue text does — named, numeric, hexadecimal,
/// the legacy forms without a semicolon, and the ones that match nothing.
#[test]
fn character_references_in_an_annotation_are_decoded() {
  for (input, normalized, raw) in [
    ("<lang en&#x2D;US>x</lang>", "en-US", "en&#x2D;US"),
    ("<lang en&#45;US>x</lang>", "en-US", "en&#45;US"),
    ("<v R&amp;D>x</v>", "R&D", "R&amp;D"),
    // A legacy reference with no semicolon still decodes.
    ("<v R&ampD>x</v>", "R&D", "R&ampD"),
    // The longest match wins, and nothing beyond it is consumed.
    ("<v &notit;>x</v>", "\u{00AC}it;", "&notit;"),
    // No match at all passes through as text.
    ("<v &zzz;>x</v>", "&zzz;", "&zzz;"),
    ("<v a&>x</v>", "a&", "a&"),
    // A NUL is U+FFFD here as it is in cue text.
    ("<v a\0b>x</v>", "a\u{FFFD}b", "a\0b"),
  ] {
    assert_eq!(
      annotation_of(input),
      (Some(normalized.to_owned()), Some(raw.to_owned())),
      "{input:?}"
    );
  }
}

/// *"Replace any sequence of one or more consecutive ASCII whitespace
/// characters in buffer with a single U+0020 SPACE character."* One or more —
/// so a lone TAB is rewritten too — and over Infra's five, the set the trim
/// already used.
#[test]
fn internal_whitespace_runs_collapse_to_one_space() {
  for (input, normalized) in [
    ("<v Roger  Bingham>x</v>", "Roger Bingham"),
    ("<v Roger\tBingham>x</v>", "Roger Bingham"),
    ("<v Roger\nBingham>x</v>", "Roger Bingham"),
    ("<v Roger\u{000C}Bingham>x</v>", "Roger Bingham"),
    ("<v Roger\rBingham>x</v>", "Roger Bingham"),
    ("<v Roger \t\n\u{000C}\r Bingham>x</v>", "Roger Bingham"),
    ("<v a  b  c>x</v>", "a b c"),
    // A single space between two characters is already the normal form.
    ("<v Roger Bingham>x</v>", "Roger Bingham"),
    // NO-BREAK SPACE is not ASCII whitespace, so it is neither trimmed nor
    // collapsed — the distinction #27 drew, still drawn.
    ("<v a\u{00A0}\u{00A0}b>x</v>", "a\u{00A0}\u{00A0}b"),
  ] {
    assert_eq!(
      annotation_of(input).0,
      Some(normalized.to_owned()),
      "{input:?}"
    );
  }
}

/// The buffer §6.4 collapses holds *decoded* characters, so the two steps
/// compose in that order and only in that order: a `&#x20;` is whitespace by
/// the time the run is measured, and can be trimmed away entirely.
#[test]
fn the_annotation_decodes_before_it_collapses() {
  // Two references, one run, one space.
  assert_eq!(
    annotation_of("<v a&#x20;&#x20;b>x</v>").0,
    Some("a b".to_owned())
  );
  // A reference beside a literal space is the same run.
  assert_eq!(
    annotation_of("<v a&#x20; b>x</v>").0,
    Some("a b".to_owned())
  );
  // A decoded TAB is whitespace like any other.
  assert_eq!(annotation_of("<v a&Tab;b>x</v>").0, Some("a b".to_owned()));
  // And at the ends, where the source text has nothing to trim.
  assert_eq!(
    annotation_of("<v &#x20;a&#x20;>x</v>").0,
    Some("a".to_owned())
  );

  // An annotation that is nothing but whitespace references is present — the
  // trim of the source text saw no whitespace to remove — and normalizes to
  // the empty string. §6.4 draws no line between that and an absent
  // annotation, so neither may a caller.
  let (normalized, raw) = annotation_of("<v &#x20;>x</v>");
  assert_eq!(normalized, Some(String::new()));
  assert_eq!(raw, Some("&#x20;".to_owned()));
}

/// `&gt;` is a character reference, not a delimiter: the tokenizer ends the tag
/// on a literal `>` alone, so the annotation runs past it and decodes to one.
/// This is why a node is serialized from `as_raw` — writing the decoded `>`
/// back would end the start tag two characters early.
#[test]
fn a_greater_than_reference_stays_inside_the_annotation() {
  let tree = CueText::parse("<v a&gt;b>x</v>");
  let node = first_tag(&tree);
  assert_eq!(node.annotation().map(Annotation::normalize), Some("a>b"));
  assert_eq!(node.annotation().map(Annotation::as_raw), Some("a&gt;b"));
  assert_eq!(
    tree.children().len(),
    1,
    "the tag must not have ended early"
  );
  assert_eq!(tree.to_string(), "<v a&gt;b>x</v>");

  // Round-tripping the decoded value instead would build a different document.
  assert_eq!(CueText::parse("<v a>b>x</v>").children().len(), 1);
  assert_eq!(
    first_tag(&CueText::parse("<v a>b>x</v>"))
      .annotation()
      .map(Annotation::normalize),
    Some("a")
  );
}

/// Every annotation the crate reports is read the same way — `<v>`'s voice as
/// much as `<lang>`'s language, through the tree and through a bare token.
#[test]
fn the_voice_and_the_language_normalize_alike() {
  assert_eq!(
    annotation_of("<v Esme&#x20;&#x20;Vale>x</v>").0,
    Some("Esme Vale".to_owned())
  );
  assert_eq!(
    first_tag(&CueText::parse("<lang en&#x2D;US>x</lang>")).declared_language(),
    Some("en-US")
  );

  // The token path, with no tree at all.
  let tokens: Vec<_> = CueParser::new("<v Esme&#x9;Vale>x</v>").collect();
  assert_eq!(token_annotation(&tokens[0]), Some("Esme Vale"));
  assert_eq!(token_annotation_raw(&tokens[0]), Some("Esme&#x9;Vale"));

  // And the unterminated path, which builds its annotation through the same
  // function.
  let unterminated: Vec<_> = CueParser::new("<lang en&#x2D;US").collect();
  assert_eq!(token_annotation(&unterminated[0]), Some("en-US"));
}

/// An annotation already in §6.4's normal form is its own normalized value, so
/// it is handed back borrowed from the cue: the parser allocates for an
/// annotation only when the spec says the text has to change.
#[test]
fn an_annotation_already_normal_is_borrowed() {
  let input = String::from("<v Roger Bingham>x</v>");
  let tree = CueText::parse(&input);
  let annotation = first_tag(&tree).annotation().expect("an annotation");

  assert!(!annotation.requires_normalization());
  assert_eq!(
    annotation.normalize().as_ptr(),
    annotation.as_raw().as_ptr(),
    "a normal-form annotation must not be copied"
  );
  assert!(
    input
      .as_bytes()
      .as_ptr_range()
      .contains(&annotation.as_raw().as_ptr()),
    "and must still point into the cue"
  );

  // Everything the flag calls abnormal is one of §6.4's own rewrites.
  for raw in ["a  b", "a\tb", " a", "a ", "a&#x20;b", "a\0b"] {
    assert!(
      Annotation::new(raw).requires_normalization(),
      "{raw:?} is not in normal form"
    );
  }
}

/// Decoding happens once, on demand, and the answer is kept.
#[test]
fn normalizing_an_annotation_is_lazy_and_cached() {
  let annotation = Annotation::new("Esme&#x20;&#x20;Vale");
  assert!(annotation.requires_normalization());

  let first = annotation.normalize();
  let second = annotation.normalize();
  assert_eq!(first, "Esme Vale");
  assert_eq!(
    first.as_ptr(),
    second.as_ptr(),
    "the decoded annotation must be computed once"
  );
}

/// A tree writes its annotations back as the cue spelled them, so a parse and a
/// write is still the identity on a document whose annotations carry character
/// references or uncollapsed whitespace.
#[test]
fn the_writer_emits_the_annotation_as_stored() {
  for input in [
    "<lang en&#x2D;US>x</lang>",
    "<v Roger&#x20;&#x20;Bingham>x</v>",
    "<v a&gt;b>x</v>",
    "<v.loud Esme  Vale>x</v>",
  ] {
    assert_eq!(CueText::parse(input).to_string(), input, "{input:?}");
  }
}

/// The whole walk reports the normalized language, and nesting still shadows.
#[test]
fn the_language_walk_reports_the_normalized_language() {
  let tree = CueText::parse("<lang en&#x2D;US>a<lang ja&#x20;&#x20;JP>b</lang>c</lang>d");
  assert_eq!(
    languages(&tree),
    [
      ("lang".to_owned(), lang("en-US")),
      ("\"a\"".to_owned(), lang("en-US")),
      ("lang".to_owned(), lang("ja JP")),
      ("\"b\"".to_owned(), lang("ja JP")),
      ("\"c\"".to_owned(), lang("en-US")),
      ("\"d\"".to_owned(), None),
    ]
  );
}

/// HTML's numeric character reference end state replaces the legacy C1 code
/// points with the Windows-1252 characters an author writing them meant, and
/// §6.4 consumes references through that algorithm — in the cue text data state
/// and in the annotation state alike, since both run one decoder. Every row of
/// the table, in both radices, through both.
#[test]
fn legacy_c1_numeric_references_are_replaced() {
  const TABLE: [(u32, char); 27] = [
    (0x80, '\u{20AC}'),
    (0x82, '\u{201A}'),
    (0x83, '\u{0192}'),
    (0x84, '\u{201E}'),
    (0x85, '\u{2026}'),
    (0x86, '\u{2020}'),
    (0x87, '\u{2021}'),
    (0x88, '\u{02C6}'),
    (0x89, '\u{2030}'),
    (0x8A, '\u{0160}'),
    (0x8B, '\u{2039}'),
    (0x8C, '\u{0152}'),
    (0x8E, '\u{017D}'),
    (0x91, '\u{2018}'),
    (0x92, '\u{2019}'),
    (0x93, '\u{201C}'),
    (0x94, '\u{201D}'),
    (0x95, '\u{2022}'),
    (0x96, '\u{2013}'),
    (0x97, '\u{2014}'),
    (0x98, '\u{02DC}'),
    (0x99, '\u{2122}'),
    (0x9A, '\u{0161}'),
    (0x9B, '\u{203A}'),
    (0x9C, '\u{0153}'),
    (0x9E, '\u{017E}'),
    (0x9F, '\u{0178}'),
  ];

  for (code_point, replacement) in TABLE {
    let want = replacement.to_string();
    for reference in [format!("&#x{code_point:X};"), format!("&#{code_point};")] {
      // Cue text, through the data state.
      assert_eq!(
        CueStr::needs_normalization(&reference).normalize(),
        want,
        "text {reference}"
      );
      // An annotation, through the annotation state.
      assert_eq!(
        Annotation::new(&reference).normalize(),
        want,
        "annotation {reference}"
      );
    }
  }

  // The five the table omits have no Windows-1252 character, so they stay the
  // C1 control the reference named.
  for code_point in [0x81_u32, 0x8D, 0x8F, 0x90, 0x9D] {
    let reference = format!("&#x{code_point:X};");
    let want = char::from_u32(code_point)
      .expect("a C1 control")
      .to_string();
    assert_eq!(
      CueStr::needs_normalization(&reference).normalize(),
      want,
      "{reference}"
    );
    assert_eq!(Annotation::new(&reference).normalize(), want, "{reference}");
  }

  // The end state's other substitutions are unchanged: NULL, a surrogate and a
  // code point past U+10FFFF are all U+FFFD, and neither is a C1 replacement.
  for reference in ["&#0;", "&#x0;", "&#xD800;", "&#xFFFFFF;", "&#99999999999;"] {
    assert_eq!(
      CueStr::needs_normalization(reference).normalize(),
      "\u{FFFD}",
      "{reference}"
    );
    assert_eq!(
      Annotation::new(reference).normalize(),
      "\u{FFFD}",
      "{reference}"
    );
  }

  // And the replacement is a real character in the tree, collapsed with the
  // whitespace around it like any other.
  assert_eq!(
    annotation_of("<v a&#x80;&#x9;b>x</v>").0,
    Some("a\u{20AC} b".to_owned())
  );
}
