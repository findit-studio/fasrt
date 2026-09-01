//! WPT cue text parsing conformance tests.
//!
//! Loads the JSON fixtures from `tests/webvtt/wpt-cue-parsing/` and the
//! corresponding VTT files, parses cues through the file parser, then feeds
//! each cue body into `CueText::parse()` and compares the resulting tree
//! with the WPT expected output.

#![cfg(feature = "std")]

use fasrt::vtt::cue::{Annotation, CueText, Node, Tag};
use fasrt::vtt::{Block, Parser};
use serde::Deserialize;

// ── JSON fixture format ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WptTestCase {
  #[allow(dead_code)]
  text: String,
  #[serde(rename = "expectedTree")]
  expected_tree: String,
}

fn load_fixture(name: &str) -> Vec<WptTestCase> {
  let path = format!(
    "{}/fixtures/webvtt/wpt-cue-parsing/{name}",
    env!("CARGO_MANIFEST_DIR")
  );
  let data = std::fs::read_to_string(&path).unwrap();
  serde_json::from_str(&data).unwrap()
}

fn load_vtt(name: &str) -> String {
  std::fs::read_to_string(format!(
    "{}/fixtures/webvtt/wpt-cue-parsing/{name}",
    env!("CARGO_MANIFEST_DIR")
  ))
  .unwrap()
}

/// Extract cue bodies from a VTT file.
fn extract_cue_bodies(vtt: &str) -> Vec<&str> {
  Parser::new(vtt)
    .filter_map(|block| match block {
      Ok(Block::Cue(cue)) => Some(*cue.body_ref()),
      _ => None,
    })
    .collect()
}

// ── Tree serializer (WPT expected format) ──────────────────────────────────

fn format_tree(tree: &CueText<'_>) -> String {
  let mut out = String::new();
  for node in tree.children() {
    format_node(node, 0, &mut out);
  }
  out
}

fn format_node(node: &Node<'_>, depth: usize, out: &mut String) {
  let indent = "  ".repeat(depth);
  match node {
    Node::Text(text) => {
      let normalized = text.normalize();
      out.push_str(&format!("| {indent}\"{normalized}\"\n"));
    }
    Node::Timestamp(ts) => {
      let h = ts.hours().as_u64();
      let m = ts.minutes().as_str();
      let s = ts.seconds().as_str();
      let ms = ts.millis().as_str();
      out.push_str(&format!("| {indent}<?timestamp {h:02}:{m}:{s}.{ms}>\n"));
    }
    Node::Tag(tag_node) => {
      let html_tag = match tag_node.tag() {
        Tag::Bold => "b",
        Tag::Italic => "i",
        Tag::Underline => "u",
        Tag::Ruby => "ruby",
        Tag::RubyText => "rt",
        Tag::Class | Tag::Voice | Tag::Lang => "span",
      };
      out.push_str(&format!("| {indent}<{html_tag}>\n"));

      let child_indent = "  ".repeat(depth + 1);

      // Classes → the HTML `class` attribute. §6.4's list, not the raw slice:
      // splitting the raw text naively would spell `<c.a..b>` as `"a  b"`,
      // where the spec's list is the two classes `a` and `b`.
      let classes: Vec<&str> = tag_node.classes().collect();
      if !classes.is_empty() {
        let space_separated = classes.join(" ");
        out.push_str(&format!("| {child_indent}class=\"{space_separated}\"\n"));
      }

      // Voice annotation → title attribute. §6.4's annotation, not the source
      // text: the annotation state decodes character references and collapses
      // whitespace runs before it hands the buffer to the tree, so a browser's
      // `title` carries the decoded value.
      if tag_node.tag() == Tag::Voice {
        let ann = tag_node.annotation().map_or("", Annotation::normalize);
        out.push_str(&format!("| {child_indent}title=\"{ann}\"\n"));
      }

      // Lang annotation → lang attribute, read the same way.
      if tag_node.tag() == Tag::Lang {
        let ann = tag_node.annotation().map_or("", Annotation::normalize);
        out.push_str(&format!("| {child_indent}lang=\"{ann}\"\n"));
      }

      for child in tag_node.children() {
        format_node(child, depth + 1, out);
      }
    }
  }
}

// ── Test runners ───────────────────────────────────────────────────────────

fn run_wpt_suite(json_name: &str, vtt_name: &str) {
  let cases = load_fixture(json_name);
  let vtt = load_vtt(vtt_name);
  let cue_bodies = extract_cue_bodies(&vtt);

  assert_eq!(
    cue_bodies.len(),
    cases.len(),
    "VTT cue count ({}) != JSON test case count ({})",
    cue_bodies.len(),
    cases.len()
  );

  let mut failures = Vec::new();

  for (i, (body, case)) in cue_bodies.iter().zip(cases.iter()).enumerate() {
    let tree = CueText::parse(body);
    let actual = format_tree(&tree);

    if actual != case.expected_tree {
      failures.push((i, body.to_string(), actual, case.expected_tree.clone()));
    }
  }

  if !failures.is_empty() {
    let mut msg = format!(
      "\n{} of {} tests failed in {json_name}:\n",
      failures.len(),
      cases.len()
    );
    for (i, input, actual, expected) in &failures {
      msg.push_str(&format!(
        "\n--- Test {i} ---\n  Input:    {input:?}\n  Expected: {expected:?}\n  Actual:   {actual:?}\n"
      ));
    }
    panic!("{msg}");
  }
}

// ── Individual test suites ─────────────────────────────────────────────────

#[test]
fn wpt_cue_text() {
  run_wpt_suite("text.json", "text.vtt");
}

#[test]
fn wpt_cue_tags() {
  run_wpt_suite("tags.json", "tags.vtt");
}

#[test]
fn wpt_cue_entities() {
  run_wpt_suite("entities.json", "entities.vtt");
}

#[test]
fn wpt_cue_timestamps() {
  run_wpt_suite("timestamps.json", "timestamps.vtt");
}

#[test]
fn wpt_cue_tree_building() {
  run_wpt_suite("tree-building.json", "tree-building.vtt");
}
