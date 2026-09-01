//! Tests for the ASS/SSA document and event-row layers.

use fasrt::ass::{
  Block, Event, EventField, EventFormat, EventKind, Options, ParseAssError, Parser, Property,
  Section, Timestamp,
};
use fasrt::types::{Centisecond, Minute, Second};

// `ass::Writer` exists only with `std`, so the cases that exercise it are gated
// to match. Everything else in this file runs on every feature tier.
#[cfg(feature = "std")]
use fasrt::ass::Writer;

/// Loads a fixture from `fixtures/ass/`.
fn fixture(name: &str) -> String {
  let path = format!("{}/fixtures/ass/{name}", env!("CARGO_MANIFEST_DIR"));
  std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Every fixture that must parse cleanly in strict mode.
const VALID_FIXTURES: &[&str] = &[
  "aegisub_dialogue.ass",
  "typesetting.ass",
  "karaoke.ass",
  "speaker_names.ass",
  "ssa_v4.ssa",
  "crlf_bom.ass",
  "embedded_fonts.ass",
];

/// Fixtures written in the canonical form the writer emits, so that
/// parse → write reproduces the input byte for byte.
#[cfg(feature = "std")]
const CANONICAL_FIXTURES: &[&str] = &[
  "aegisub_dialogue.ass",
  "typesetting.ass",
  "karaoke.ass",
  "speaker_names.ass",
  "embedded_fonts.ass",
];

/// Helper: collect all blocks with the strict parser.
fn blocks(input: &str) -> Result<Vec<Block<'_>>, ParseAssError> {
  Parser::strict(input).collect()
}

/// Helper: collect only the events.
fn events(input: &str) -> Vec<Event<'_>> {
  Parser::strict(input)
    .filter_map(|block| match block {
      Ok(Block::Event(event)) => Some(event),
      _ => None,
    })
    .collect()
}

/// Helper: build an ASS timestamp.
fn ts(hours: u64, minutes: u8, seconds: u8, centis: u8) -> Timestamp {
  Timestamp::from_hmsc(
    fasrt::ass::Hour::with(hours),
    Minute::with(minutes),
    Second::with(seconds),
    Centisecond::with(centis),
  )
}

// ── Timestamps ─────────────────────────────────────────────────────────────

#[test]
fn timestamp_parses_the_canonical_form() {
  let parsed = Timestamp::parse("0:01:02.34").unwrap();
  assert_eq!(parsed, ts(0, 1, 2, 34));
  assert_eq!(parsed.encode().as_str(), "0:01:02.34");
}

#[test]
fn timestamp_hour_is_not_zero_padded() {
  assert_eq!(ts(0, 0, 0, 0).encode().as_str(), "0:00:00.00");
  assert_eq!(ts(9, 0, 0, 0).encode().as_str(), "9:00:00.00");
  assert_eq!(ts(10, 0, 0, 0).encode().as_str(), "10:00:00.00");
}

#[test]
fn timestamp_accepts_multi_digit_hours() {
  let parsed = Timestamp::parse("123:45:06.78").unwrap();
  assert_eq!(parsed.hours().as_u64(), 123);
  assert_eq!(parsed.encode().as_str(), "123:45:06.78");
}

#[test]
fn timestamp_encode_buffer_fits_the_largest_hour() {
  let max = ts(0, 0, 0, 0).with_hours(fasrt::ass::Hour::with(u64::MAX));
  assert_eq!(max.encode().as_str(), "18446744073709551615:00:00.00");
  assert_eq!(max.encoded_len(), max.encode().as_str().len());
}

#[test]
fn timestamp_rejects_malformed_input() {
  // Too short.
  assert!(Timestamp::parse("").is_err());
  assert!(Timestamp::parse("0:00:00.0").is_err());
  // Milliseconds are the WebVTT/SRT form, not ASS.
  assert!(Timestamp::parse("0:00:01.000").is_err());
  // Wrong separators.
  assert!(Timestamp::parse("0:00:00,00").is_err());
  assert!(Timestamp::parse("0-00-00.00").is_err());
  // Non-digit components.
  assert!(Timestamp::parse("0:0a:00.00").is_err());
  assert!(Timestamp::parse("x:00:00.00").is_err());
  // Out-of-range minutes and seconds.
  assert!(Timestamp::parse("0:60:00.00").is_err());
  assert!(Timestamp::parse("0:00:60.00").is_err());
}

#[test]
fn timestamp_hour_overflow_is_an_error() {
  assert!(Timestamp::parse("18446744073709551616:00:00.00").is_err());
}

#[test]
fn timestamp_round_trips_through_duration() {
  let original = ts(1, 2, 3, 4);
  assert_eq!(Timestamp::from_duration(original.to_duration()), original);
}

#[test]
fn timestamp_from_duration_truncates_below_a_centisecond() {
  let ts = Timestamp::from_duration(core::time::Duration::from_millis(3_723_049));
  assert_eq!(ts.encode().as_str(), "1:02:03.04");
}

#[test]
fn timestamps_order_chronologically() {
  assert!(ts(0, 0, 0, 1) > ts(0, 0, 0, 0));
  assert!(ts(0, 1, 0, 0) > ts(0, 0, 59, 99));
  assert!(ts(1, 0, 0, 0) > ts(0, 59, 59, 99));
}

// ── Event format resolution ────────────────────────────────────────────────

#[test]
fn ass_preset_matches_the_declared_v4_plus_order() {
  let declared =
    EventFormat::new("Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text");
  assert_eq!(declared, EventFormat::ass());
}

#[test]
fn actor_is_accepted_as_a_spelling_of_name() {
  let declared =
    EventFormat::new("Layer, Start, End, Style, Actor, MarginL, MarginR, MarginV, Effect, Text");
  assert_eq!(declared, EventFormat::ass());
  assert_eq!(declared.index_of(EventField::Name), Some(4));
}

#[test]
fn unknown_format_names_still_occupy_a_position() {
  let declared = EventFormat::new("Layer, Mystery, Start, End, Text");
  assert_eq!(declared.len(), 5);
  assert_eq!(declared.index_of(EventField::Layer), Some(0));
  assert_eq!(declared.index_of(EventField::Start), Some(2));
  assert_eq!(declared.index_of(EventField::Text), Some(4));
  assert_eq!(declared.field_at(1), None);
}

#[test]
fn an_empty_format_declaration_yields_an_empty_format() {
  // `"".split(',')` yields one empty item, which must not be counted as a
  // column: an empty declaration declares nothing.
  assert!(EventFormat::new("").is_empty());
  assert!(EventFormat::new("   ").is_empty());
  assert_eq!(EventFormat::new("").len(), 0);
}

#[test]
fn an_empty_format_line_rejects_the_following_event() {
  let script = "[Events]\nFormat:\nDialogue: hello\n";
  assert!(matches!(blocks(script), Err(ParseAssError::MissingFormat)));
}

#[test]
fn duplicate_format_names_resolve_to_the_last_position() {
  // libass walks the declaration in order and each assignment overwrites the
  // previous, so a repeated column resolves to its last occurrence.
  let declared = EventFormat::new("Start, Start, Text");
  assert_eq!(declared.index_of(EventField::Start), Some(1));
  assert_eq!(declared.len(), 3);

  let event = Event::parse(
    "Dialogue: 0:00:01.00,0:00:09.00,text",
    &EventFormat::new("Start, Start, Text"),
  )
  .unwrap();
  assert_eq!(event.start(), Some(ts(0, 0, 9, 0)));
}

#[test]
fn duplicate_columns_resolve_to_the_last_for_every_field() {
  let declared = EventFormat::new("Layer, Style, Name, Layer, Style, Name, Text");
  assert_eq!(declared.index_of(EventField::Layer), Some(3));
  assert_eq!(declared.index_of(EventField::Style), Some(4));
  assert_eq!(declared.index_of(EventField::Name), Some(5));

  let event = Event::parse("Dialogue: 1,A,First,2,B,Second,text", &declared).unwrap();
  assert_eq!(event.layer(), Some(2));
  assert_eq!(event.style(), Some("B"));
  assert_eq!(event.name(), Some("Second"));
}

#[test]
fn absurdly_long_format_lines_are_capped_not_panicked() {
  let declaration = "X,".repeat(5_000);
  let declared = EventFormat::new(&declaration);
  assert_eq!(declared.len(), EventFormat::MAX_FIELDS);
}

#[test]
fn field_at_is_the_inverse_of_index_of() {
  let format = EventFormat::ass();
  for index in 0..format.len() {
    let field = format.field_at(index).unwrap();
    assert_eq!(format.index_of(field), Some(index));
  }
  assert_eq!(format.field_at(format.len()), None);
}

// ── Event rows ─────────────────────────────────────────────────────────────

#[test]
fn dialogue_row_parses_every_field() {
  let line = "Dialogue: 3,0:00:01.29,0:00:03.85,Default,Rin,10,20,30,Karaoke,Hello there";
  let event = Event::parse(line, &EventFormat::ass()).unwrap();

  assert_eq!(event.kind(), EventKind::Dialogue);
  assert_eq!(event.layer(), Some(3));
  assert_eq!(event.start(), Some(ts(0, 0, 1, 29)));
  assert_eq!(event.end(), Some(ts(0, 0, 3, 85)));
  assert_eq!(event.style(), Some("Default"));
  assert_eq!(event.name(), Some("Rin"));
  assert_eq!(event.margin_l(), Some(10));
  assert_eq!(event.margin_r(), Some(20));
  assert_eq!(event.margin_v(), Some(30));
  assert_eq!(event.effect(), Some("Karaoke"));
  assert_eq!(event.text(), "Hello there");
}

#[test]
fn text_field_keeps_its_commas() {
  let line = "Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,One, two, three";
  let event = Event::parse(line, &EventFormat::ass()).unwrap();
  assert_eq!(event.text(), "One, two, three");
}

#[test]
fn text_field_keeps_its_trailing_space() {
  let line = "Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,trailing  ";
  let event = Event::parse(line, &EventFormat::ass()).unwrap();
  assert_eq!(event.text(), "trailing  ");
}

#[test]
fn empty_fields_are_absent() {
  let line = "Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,text";
  let event = Event::parse(line, &EventFormat::ass()).unwrap();
  assert_eq!(event.name(), None);
  assert_eq!(event.effect(), None);
}

#[test]
fn comment_rows_are_events_of_their_own_kind() {
  let line = "Comment: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,note to self";
  let event = Event::parse(line, &EventFormat::ass()).unwrap();
  assert_eq!(event.kind(), EventKind::Comment);
  assert_eq!(event.text(), "note to self");
}

#[test]
fn every_ssa_event_kind_is_recognized() {
  for (keyword, expected) in [
    ("Dialogue", EventKind::Dialogue),
    ("Comment", EventKind::Comment),
    ("Picture", EventKind::Picture),
    ("Sound", EventKind::Sound),
    ("Movie", EventKind::Movie),
    ("Command", EventKind::Command),
  ] {
    let line = format!("{keyword}: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,x");
    let event = Event::parse(&line, &EventFormat::ass()).unwrap();
    assert_eq!(event.kind(), expected);
  }
}

#[test]
fn short_row_is_an_error_in_strict_mode() {
  let line = "Dialogue: 0,0:00:01.00,0:00:03.00";
  assert!(matches!(
    Event::parse(line, &EventFormat::ass()),
    Err(ParseAssError::TooFewFields {
      expected: 10,
      found: 3
    }),
  ));
}

#[test]
fn short_row_is_tolerated_in_lossy_mode() {
  let line = "Dialogue: 0,0:00:01.00,0:00:03.00";
  let event = Event::parse_with(line, &EventFormat::ass(), &Options::lossy()).unwrap();
  assert_eq!(event.start(), Some(ts(0, 0, 1, 0)));
  assert_eq!(event.style(), None);
  assert_eq!(event.text(), "");
}

#[test]
fn malformed_number_is_an_error_in_strict_mode() {
  let line = "Dialogue: x,0:00:01.00,0:00:03.00,Default,,0,0,0,,text";
  assert!(matches!(
    Event::parse(line, &EventFormat::ass()),
    Err(ParseAssError::InvalidField(EventField::Layer)),
  ));
}

#[test]
fn malformed_field_is_absent_in_lossy_mode() {
  let line = "Dialogue: x,bad,0:00:03.00,Default,,0,0,0,,text";
  let event = Event::parse_with(line, &EventFormat::ass(), &Options::lossy()).unwrap();
  assert_eq!(event.layer(), None);
  assert_eq!(event.start(), None);
  assert_eq!(event.end(), Some(ts(0, 0, 3, 0)));
  assert_eq!(event.text(), "text");
}

#[test]
fn an_empty_format_cannot_parse_a_row() {
  assert!(matches!(
    Event::parse_fields(EventKind::Dialogue, "a,b", &EventFormat::empty()),
    Err(ParseAssError::MissingFormat),
  ));
}

// ── Matroska packets: layer 2 standalone ───────────────────────────────────

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn matroska_packet_parses_without_a_document() {
  // Exactly the payload Matroska carries for S_TEXT/ASS: no `Dialogue:`
  // keyword and no Start/End — the container supplies the timing.
  let packet = "12,0,Default,Rin,0,0,0,,{\\i1}Hello{\\i0}";
  let event = Event::parse_fields(EventKind::Dialogue, packet, &EventFormat::matroska()).unwrap();

  assert_eq!(event.read_order(), Some(12));
  assert_eq!(event.layer(), Some(0));
  assert_eq!(event.style(), Some("Default"));
  assert_eq!(event.name(), Some("Rin"));
  assert_eq!(event.start(), None);
  assert_eq!(event.end(), None);
  assert_eq!(event.text(), "{\\i1}Hello{\\i0}");
  assert_eq!(event.plain_text().normalize(), "Hello");
}

#[test]
fn matroska_packet_keeps_commas_in_text() {
  let packet = "3,0,Default,,0,0,0,,Wait, what?";
  let event = Event::parse_fields(EventKind::Dialogue, packet, &EventFormat::matroska()).unwrap();
  assert_eq!(event.text(), "Wait, what?");
}

#[test]
fn matroska_format_declares_no_timing_columns() {
  let format = EventFormat::matroska();
  assert_eq!(format.index_of(EventField::Start), None);
  assert_eq!(format.index_of(EventField::End), None);
  assert_eq!(format.index_of(EventField::ReadOrder), Some(0));
}

// ── Document layer ─────────────────────────────────────────────────────────

#[test]
fn sections_properties_and_comments_are_distinguished() {
  let script = "[Script Info]\n; a comment\nTitle: Example\n";
  let parsed = blocks(script).unwrap();
  assert_eq!(
    parsed,
    [
      Block::Section(Section::ScriptInfo),
      Block::Comment(" a comment"),
      Block::Property(Property::new("Title", "Example")),
    ],
  );
}

#[test]
fn section_names_are_case_insensitive() {
  let parsed = blocks("[events]\n").unwrap();
  assert_eq!(parsed, [Block::Section(Section::Events)]);
}

#[test]
fn unknown_sections_are_preserved_verbatim() {
  let parsed = blocks("[Aegisub Project Garbage]\n").unwrap();
  assert_eq!(
    parsed,
    [Block::Section(Section::Other("Aegisub Project Garbage"))],
  );
}

#[test]
fn style_rows_are_only_recognized_in_a_style_section() {
  // Inside a style section, `Style:` is a row.
  let inside = blocks("[V4+ Styles]\nStyle: Default,Arial,20\n").unwrap();
  assert!(matches!(inside[1], Block::Style(_)));

  // Outside one, the same line is an ordinary property.
  let outside = blocks("[Script Info]\nStyle: Default,Arial,20\n").unwrap();
  assert!(matches!(outside[1], Block::Property(_)));
}

#[test]
fn v4_plus_plus_styles_still_carry_style_rows() {
  let parsed = blocks("[V4++ Styles]\nStyle: Default,Arial,20\n").unwrap();
  assert!(matches!(parsed[1], Block::Style(_)));
}

#[test]
fn event_rows_are_only_recognized_in_the_events_section() {
  // A `Comment:` line in `[Script Info]` is a property, not an event.
  let parsed = blocks("[Script Info]\nComment: not an event\n").unwrap();
  assert_eq!(
    parsed[1],
    Block::Property(Property::new("Comment", "not an event")),
  );
}

#[test]
fn a_second_events_section_needs_its_own_format() {
  let script = "\
[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:03.00,Default,A,0,0,0,,first

[Events]
Dialogue: 0,0:00:04.00,0:00:06.00,Default,B,0,0,0,,second
";
  assert!(matches!(blocks(script), Err(ParseAssError::MissingFormat),));
}

#[test]
fn parser_exposes_the_current_section_and_format() {
  let script =
    "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n";
  let mut parser = Parser::strict(script);
  assert_eq!(parser.section(), None);
  let _ = parser.next();
  assert_eq!(parser.section(), Some(Section::Events));
  let _ = parser.next();
  assert_eq!(parser.event_format(), Some(EventFormat::ass()));
}

#[test]
fn blank_lines_are_skipped() {
  let parsed = blocks("\n\n[Events]\n\n\n").unwrap();
  assert_eq!(parsed, [Block::Section(Section::Events)]);
}

#[test]
fn a_bom_is_ignored() {
  let parsed = blocks("\u{feff}[Events]\n").unwrap();
  assert_eq!(parsed, [Block::Section(Section::Events)]);
}

#[test]
fn cr_and_crlf_line_endings_are_accepted() {
  for script in ["[Events]\r\nFormat: Text\r\n", "[Events]\rFormat: Text\r"] {
    let parsed = blocks(script).unwrap();
    assert_eq!(parsed.len(), 2, "failed for {script:?}");
    assert_eq!(parsed[0], Block::Section(Section::Events));
  }
}

// ── Strict-mode errors ─────────────────────────────────────────────────────

#[test]
fn unclosed_section_header_is_an_error() {
  assert!(matches!(
    blocks("[Events\n"),
    Err(ParseAssError::UnclosedSection),
  ));
}

#[test]
fn a_line_without_a_colon_is_an_error() {
  assert!(matches!(
    blocks("[Script Info]\nno colon here\n"),
    Err(ParseAssError::UnexpectedLine),
  ));
}

#[test]
fn an_empty_key_is_an_error() {
  assert!(matches!(
    blocks("[Script Info]\n: value\n"),
    Err(ParseAssError::UnexpectedLine),
  ));
}

#[test]
fn an_event_before_the_format_line_is_an_error() {
  let script = "[Events]\nDialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,x\n";
  assert!(matches!(blocks(script), Err(ParseAssError::MissingFormat)));
}

#[test]
fn iteration_stops_after_an_error() {
  let mut parser = Parser::strict("[Script Info]\nno colon here\nTitle: unreachable\n");
  assert!(parser.next().unwrap().is_ok());
  assert!(parser.next().unwrap().is_err());
  assert!(parser.next().is_none());
}

// ── Lossy mode ─────────────────────────────────────────────────────────────

#[test]
fn lossy_mode_recovers_the_readable_rows_of_a_broken_script() {
  let script = fixture("malformed.ass");
  let recovered = Parser::lossy(&script)
    .collect::<Result<Vec<_>, _>>()
    .expect("lossy mode must not fail");

  let events: Vec<_> = recovered
    .iter()
    .filter_map(|block| match block {
      Block::Event(event) => Some(event),
      _ => None,
    })
    .collect();

  // Every event row is recovered, including the one before the `Format:`
  // line and the ones with broken fields.
  let names: Vec<_> = events.iter().map(|event| event.name()).collect();
  assert_eq!(
    names,
    [
      Some("Early"),
      Some("Ok"),
      None,
      Some("Bad"),
      Some("Frag"),
      Some("Tail"),
      Some("Fine"),
    ],
  );

  // The broken start timestamp and the non-numeric margin become absent.
  assert_eq!(events[3].start(), None);
  assert_eq!(events[4].margin_l(), None);

  // The padded `0000` margins still parse.
  assert_eq!(events[1].margin_l(), Some(0));

  // An unmatched `{` is literal text, so nothing visible is lost.
  assert_eq!(
    events[5].plain_text().normalize(),
    "Unterminated override {\\i1",
  );
}

#[test]
fn the_malformed_fixture_fails_in_strict_mode() {
  let script = fixture("malformed.ass");
  assert!(blocks(&script).is_err());
}

// ── Fixtures ───────────────────────────────────────────────────────────────

#[test]
fn every_valid_fixture_parses_in_strict_mode() {
  for name in VALID_FIXTURES {
    let script = fixture(name);
    let parsed = blocks(&script);
    assert!(parsed.is_ok(), "{name} failed: {:?}", parsed.unwrap_err());
  }
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn aegisub_fixture_yields_the_expected_events() {
  let script = fixture("aegisub_dialogue.ass");
  let events = events(&script);

  assert_eq!(events.len(), 9);
  assert_eq!(events[0].name(), Some("Rin"));
  assert_eq!(events[0].start(), Some(ts(0, 0, 1, 29)));
  assert_eq!(events[0].end(), Some(ts(0, 0, 3, 85)));
  assert_eq!(events[0].text(), "Morning. You're up early for once.");

  // The italic line cleans down to its words.
  assert_eq!(
    events[2].plain_text().normalize(),
    "Not after what happened yesterday.",
  );

  // `\N` becomes a real line break.
  assert_eq!(
    events[3].plain_text().normalize(),
    "Then sit down.\nI'll make you something.",
  );

  // The `Comment:` row is present and typed.
  assert_eq!(events[5].kind(), EventKind::Comment);

  // Layer 1 sign line.
  assert_eq!(events[7].layer(), Some(1));
  assert_eq!(events[7].style(), Some("Sign"));
  assert_eq!(events[7].plain_text().normalize(), "Kitchen, 6:14 AM");
}

#[test]
fn speaker_names_fixture_surfaces_every_name() {
  let script = fixture("speaker_names.ass");
  let names: Vec<_> = events(&script).iter().map(|event| event.name()).collect();

  assert_eq!(
    names,
    [
      Some("Hanekawa Tsubasa"),
      Some("Hanekawa Tsubasa"),
      Some("Araragi"),
      Some("忍野メメ"),
      Some("Dr. Ashford"),
      Some("NARRATOR"),
      None,
      Some("Kid #2"),
    ],
  );
}

#[test]
fn ssa_fixture_carries_marked_instead_of_layer() {
  let script = fixture("ssa_v4.ssa");
  let events = events(&script);

  assert_eq!(events[0].marked(), Some("Marked=0"));
  assert_eq!(events[0].layer(), None);
  assert_eq!(events[0].name(), Some("Kaji"));
  assert_eq!(events[0].start(), Some(ts(0, 0, 0, 50)));
  // The padded SSA margins parse to plain numbers.
  assert_eq!(events[0].margin_l(), Some(0));
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn karaoke_fixture_cleans_to_lyrics() {
  let script = fixture("karaoke.ass");
  let events = events(&script);
  assert_eq!(events[1].effect(), Some("fx"));
  assert_eq!(events[1].plain_text().normalize(), "kimino koega");
  assert_eq!(events[5].plain_text().normalize(), "君の声が");
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn typesetting_fixture_skips_drawings_and_keeps_words() {
  let script = fixture("typesetting.ass");
  let events = events(&script);

  // A pure drawing line cleans to nothing.
  let drawing = events
    .iter()
    .find(|event| event.text().starts_with("{\\p1\\pos"))
    .expect("the drawing line must be present");
  assert_eq!(drawing.plain_text().normalize(), "");

  // A drawing followed by text keeps only the text.
  let mixed = events
    .iter()
    .find(|event| event.text().starts_with("{\\p4}"))
    .expect("the mixed drawing line must be present");
  assert_eq!(mixed.plain_text().normalize(), "after the drawing");

  // Commas inside the text field survive.
  let commas = events
    .iter()
    .find(|event| event.text().starts_with("Commas"))
    .expect("the comma line must be present");
  assert_eq!(commas.text(), "Commas, semicolons; and colons: all fine",);
}

// Asserts cleaned text, which `PlainText::normalize` can only produce with
// `alloc`; without it the method is documented to return the raw text.
#[cfg(any(feature = "alloc", feature = "std"))]
#[test]
fn crlf_fixture_parses_and_strips_the_bom() {
  let script = fixture("crlf_bom.ass");
  let parsed = blocks(&script).unwrap();
  assert_eq!(parsed[0], Block::Section(Section::ScriptInfo));

  let events = events(&script);
  assert_eq!(events.len(), 3);
  assert_eq!(events[1].text(), "Second line, with a comma.");
  assert_eq!(events[2].plain_text().normalize(), "Third line, italic.");
}

// ── Writer ─────────────────────────────────────────────────────────────────

#[cfg(feature = "std")]
#[test]
fn canonical_fixtures_round_trip_byte_for_byte() {
  for name in CANONICAL_FIXTURES {
    let script = fixture(name);
    let parsed = blocks(&script).unwrap();

    let mut buf = Vec::new();
    Writer::new(&mut buf).write_all(&parsed).unwrap();

    assert_eq!(
      String::from_utf8(buf).unwrap(),
      script,
      "{name} did not round-trip byte for byte",
    );
  }
}

#[cfg(feature = "std")]
#[test]
fn writing_is_idempotent_for_every_valid_fixture() {
  // The writer emits a canonical form, so the first write may normalize
  // non-canonical spellings (padded `0000` margins, CRLF). Every write after
  // that must be a fixed point, and the reparsed blocks must be identical.
  for name in VALID_FIXTURES {
    let script = fixture(name);
    let parsed = blocks(&script).unwrap();

    let mut first = Vec::new();
    Writer::new(&mut first).write_all(&parsed).unwrap();
    let first = String::from_utf8(first).unwrap();

    let reparsed = blocks(&first).unwrap();
    let mut second = Vec::new();
    Writer::new(&mut second).write_all(&reparsed).unwrap();
    let second = String::from_utf8(second).unwrap();

    assert_eq!(first, second, "{name} is not a writer fixed point");
    assert_eq!(
      reparsed,
      blocks(&second).unwrap(),
      "{name} changed meaning on a round-trip",
    );
  }
}

#[test]
fn embedded_resource_payloads_are_preserved_verbatim() {
  let script = fixture("embedded_fonts.ass");
  let parsed = blocks(&script).unwrap();

  // A payload line containing `:` must be a data line, not a property.
  let data: Vec<_> = parsed
    .iter()
    .filter_map(|block| match block {
      Block::Data(payload) => Some(*payload),
      _ => None,
    })
    .collect();
  assert!(
    data.iter().any(|line| line.contains(':')),
    "the fixture must exercise a payload line containing a colon",
  );

  // `fontname:` and `filename:` remain properties.
  let keys: Vec<_> = parsed
    .iter()
    .filter_map(|block| match block {
      Block::Property(property) => Some(property.key()),
      _ => None,
    })
    .collect();
  assert!(keys.contains(&"fontname"));
  assert!(keys.contains(&"filename"));
}

#[test]
fn a_resource_payload_line_is_not_split_on_its_colon() {
  let script = "[Fonts]\nfontname: x.ttf\n2c:34;58<=>?@ABC\n";
  let parsed = blocks(script).unwrap();
  assert_eq!(parsed[2], Block::Data("2c:34;58<=>?@ABC"));
}

#[test]
fn a_colonless_resource_payload_line_parses_in_strict_mode() {
  // Outside a resource section this line would be a strict-mode error.
  let script = "[Fonts]\nfontname: x.ttf\n!!!!!!!!!!!!!!!!\n";
  let parsed = blocks(script).unwrap();
  assert_eq!(parsed[2], Block::Data("!!!!!!!!!!!!!!!!"));
}

#[test]
fn a_resource_payload_line_starting_with_a_semicolon_is_data() {
  // The payload alphabet spans U+0021..=U+0061, so `;` is a payload byte and
  // must not be read as a comment.
  let script = "[Fonts]\nfontname: x.ttf\n;3456789<=>?@AB\n";
  let parsed = blocks(script).unwrap();
  assert_eq!(parsed[2], Block::Data(";3456789<=>?@AB"));
}

#[test]
fn only_the_exact_lowercase_resource_header_is_a_property() {
  // Uppercase is a legal payload spelling, so it is data, not a header.
  let script = "[Fonts]\nFONTNAME:PAYLOAD\n";
  assert_eq!(blocks(script).unwrap()[1], Block::Data("FONTNAME:PAYLOAD"));

  // `[Graphics]` uses `filename:`; `fontname:` there is payload.
  let script = "[Graphics]\nfontname: x.ttf\n";
  assert_eq!(blocks(script).unwrap()[1], Block::Data("fontname: x.ttf"));
}

#[test]
fn a_section_header_still_ends_a_resource_section() {
  let script = "[Fonts]\nfontname: x.ttf\n!!!!\n[Events]\nFormat: Text\n";
  let parsed = blocks(script).unwrap();
  assert_eq!(parsed[3], Block::Section(Section::Events));
}

#[cfg(feature = "std")]
#[test]
fn writer_follows_the_declared_field_order() {
  // A format that omits margins and reorders the leading columns.
  let script = "\
[Events]
Format: Start, End, Name, Style, Text
Dialogue: 0:00:01.00,0:00:03.00,Rin,Default,Hello
";
  let parsed = blocks(script).unwrap();

  let mut buf = Vec::new();
  Writer::new(&mut buf).write_all(&parsed).unwrap();
  assert_eq!(String::from_utf8(buf).unwrap(), script);
}

#[cfg(feature = "std")]
#[test]
fn writer_emits_empty_columns_for_absent_fields() {
  let event = Event::new(EventKind::Dialogue, "Hi").with_style(Some("Default"));

  let mut buf = Vec::new();
  Writer::new(&mut buf).write_event(&event).unwrap();
  assert_eq!(
    String::from_utf8(buf).unwrap(),
    "Dialogue: ,,,Default,,,,,,Hi\n",
  );
}

#[cfg(feature = "std")]
#[test]
fn unknown_format_columns_survive_a_round_trip() {
  // A `Format:` line may declare vendor or future columns. Their values must
  // not be silently dropped by a parse/write cycle.
  let script = "\
[Events]
Format: Layer, MarginT, MarginB, Text
Dialogue: 0,11,22,hello
";
  let parsed = blocks(script).unwrap();

  let mut buf = Vec::new();
  Writer::new(&mut buf).write_all(&parsed).unwrap();
  assert_eq!(String::from_utf8(buf).unwrap(), script);
}

#[test]
fn unknown_columns_are_readable_by_position() {
  let format = EventFormat::new("Layer, MarginT, MarginB, Text");
  let event = Event::parse("Dialogue: 0,11,22,hello", &format).unwrap();

  assert_eq!(event.layer(), Some(0));
  assert_eq!(event.field(1), Some("11"));
  assert_eq!(event.field(2), Some("22"));
  assert_eq!(event.text(), "hello");
  // A constructed event has no row to read from.
  assert_eq!(Event::new(EventKind::Dialogue, "x").field(0), None);
}

#[test]
fn event_equality_covers_unknown_column_data() {
  // Unknown columns are observable through `field()` and through what the
  // writer emits, so two events that differ only there must not be equal.
  let format = EventFormat::new("Layer, Vendor, Text");
  let a = Event::parse("Dialogue: 0,alpha,hi", &format).unwrap();
  let b = Event::parse("Dialogue: 0,beta,hi", &format).unwrap();

  assert_eq!(a.layer(), b.layer());
  assert_eq!(a.text(), b.text());
  assert_ne!(a.field(1), b.field(1));
  assert_ne!(
    a, b,
    "events differing in a vendor column must not be equal"
  );

  let same = Event::parse("Dialogue: 0,alpha,hi", &format).unwrap();
  assert_eq!(a, same);
}

#[cfg(feature = "std")]
#[test]
fn a_second_events_section_does_not_inherit_the_first_format() {
  // The parser resets its field order on entering `[Events]`, so the writer
  // must too: otherwise the second section's rows are re-emitted in the first
  // section's column order and lose every field it did not declare.
  let script = "\
[Events]
Format: Start, End, Text
Dialogue: 0:00:01.00,0:00:03.00,first

[Events]
Dialogue: 0,0:00:04.00,0:00:06.00,Default,B,0,0,0,,second
";
  let parsed: Vec<_> = Parser::lossy(script).collect::<Result<_, _>>().unwrap();

  let mut buf = Vec::new();
  Writer::new(&mut buf).write_all(&parsed).unwrap();
  let rewritten = String::from_utf8(buf).unwrap();

  assert_eq!(rewritten, script);
  assert_eq!(
    Parser::lossy(&rewritten)
      .collect::<Result<Vec<_>, _>>()
      .unwrap(),
    parsed,
  );
}

#[cfg(feature = "std")]
#[test]
fn writer_normalizes_padded_margins() {
  let script = "\
[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0000,0000,0000,,x
";
  let parsed = blocks(script).unwrap();

  let mut buf = Vec::new();
  Writer::new(&mut buf).write_all(&parsed).unwrap();
  assert!(
    String::from_utf8(buf)
      .unwrap()
      .ends_with("Default,,0,0,0,,x\n"),
  );
}

#[cfg(feature = "std")]
#[test]
fn writer_separates_sections_with_a_blank_line() {
  let script = "[Script Info]\nTitle: X\n\n[Events]\nFormat: Text\n";
  let parsed = blocks(script).unwrap();

  let mut buf = Vec::new();
  Writer::new(&mut buf).write_all(&parsed).unwrap();
  assert_eq!(String::from_utf8(buf).unwrap(), script);
}

#[cfg(feature = "std")]
#[test]
fn writer_round_trips_a_matroska_packet() {
  let packet = "12,0,Default,Rin,0,0,0,,{\\i1}Hello{\\i0}";
  let format = EventFormat::matroska();
  let event = Event::parse_fields(EventKind::Dialogue, packet, &format).unwrap();

  let mut buf = Vec::new();
  Writer::with_event_format(&mut buf, format)
    .write_event(&event)
    .unwrap();

  assert_eq!(
    String::from_utf8(buf).unwrap(),
    format!("Dialogue: {packet}\n"),
  );
}
