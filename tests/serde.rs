//! Round-trip tests for the `serde` feature.
//!
//! Every public options/knob type a caller configures — `srt::Options`,
//! `ass::Options`, `ass::EventFormat` and `vtt::cue::Options` — implements
//! `Serialize`/`Deserialize`. Each gets: a round trip through every named
//! preset, a golden of the document form, and proof that the documented
//! default survives a round trip and that an omitted field falls back to it.

use fasrt::ass::{EventFormat, Options as AssOptions};
use fasrt::srt::Options as SrtOptions;

/// Serializes `value`, deserializes it back, and asserts the result matches.
fn round_trip<T>(value: T)
where
  T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
{
  let json = serde_json::to_string(&value).expect("serialize");
  let back: T = serde_json::from_str(&json).expect("deserialize");
  assert_eq!(value, back, "round trip through {json:?} changed the value");
}

// ---------------------------------------------------------------------------
// srt::Options
// ---------------------------------------------------------------------------

#[test]
fn srt_options_round_trip() {
  round_trip(SrtOptions::strict());
  round_trip(SrtOptions::lossy());
  round_trip(SrtOptions::strict().with_allow_missing_index(true));
}

#[test]
fn srt_options_default_is_strict_and_round_trips() {
  assert_eq!(SrtOptions::default(), SrtOptions::strict());
  round_trip(SrtOptions::default());
}

#[test]
fn srt_options_golden() {
  let strict = serde_json::to_value(SrtOptions::strict()).unwrap();
  assert_eq!(
    strict,
    serde_json::json!({
      "allow_missing_index": false,
      "ignore_orphan_text": false,
      "ignore_broken_header": false,
      "monotonic_index": true,
    })
  );

  let lossy = serde_json::to_value(SrtOptions::lossy()).unwrap();
  assert_eq!(
    lossy,
    serde_json::json!({
      "allow_missing_index": true,
      "ignore_orphan_text": true,
      "ignore_broken_header": true,
      "monotonic_index": false,
    })
  );
}

#[test]
fn srt_options_omitted_field_falls_back_to_default() {
  let opts: SrtOptions = serde_json::from_str(r#"{"allow_missing_index": true}"#).unwrap();
  assert_eq!(opts, SrtOptions::strict().with_allow_missing_index(true));

  let opts: SrtOptions = serde_json::from_str("{}").unwrap();
  assert_eq!(opts, SrtOptions::default());
}

// ---------------------------------------------------------------------------
// ass::Options
// ---------------------------------------------------------------------------

#[test]
fn ass_options_round_trip() {
  round_trip(AssOptions::strict());
  round_trip(AssOptions::lossy());
  round_trip(AssOptions::strict().with_allow_short_event(true));
}

#[test]
fn ass_options_default_is_strict_and_round_trips() {
  assert_eq!(AssOptions::default(), AssOptions::strict());
  round_trip(AssOptions::default());
}

#[test]
fn ass_options_golden() {
  let strict = serde_json::to_value(AssOptions::strict()).unwrap();
  assert_eq!(
    strict,
    serde_json::json!({
      "allow_missing_format": false,
      "allow_short_event": false,
      "allow_malformed_fields": false,
      "ignore_unknown_lines": false,
    })
  );

  let lossy = serde_json::to_value(AssOptions::lossy()).unwrap();
  assert_eq!(
    lossy,
    serde_json::json!({
      "allow_missing_format": true,
      "allow_short_event": true,
      "allow_malformed_fields": true,
      "ignore_unknown_lines": true,
    })
  );
}

#[test]
fn ass_options_omitted_field_falls_back_to_default() {
  let opts: AssOptions = serde_json::from_str(r#"{"allow_short_event": true}"#).unwrap();
  assert_eq!(opts, AssOptions::strict().with_allow_short_event(true));

  let opts: AssOptions = serde_json::from_str("{}").unwrap();
  assert_eq!(opts, AssOptions::default());
}

// ---------------------------------------------------------------------------
// ass::EventFormat
// ---------------------------------------------------------------------------

#[test]
fn ass_event_format_round_trip() {
  round_trip(EventFormat::empty());
  round_trip(EventFormat::ass());
  round_trip(EventFormat::ssa());
  round_trip(EventFormat::matroska());
  round_trip(EventFormat::new(
    "Layer, Start, End, Style, Name, Whatever, Text",
  ));
}

#[test]
fn ass_event_format_default_is_ass_preset_and_round_trips() {
  assert_eq!(EventFormat::default(), EventFormat::ass());
  round_trip(EventFormat::default());
}

#[test]
fn ass_event_format_golden() {
  // The document form is the type's own internal state: `slots[i]` is the
  // position of `EventField::ALL[i]` — ReadOrder, Marked, Layer, Start, End,
  // Style, Name, MarginL, MarginR, MarginV, Effect, Text, in that order — or
  // `null` when that field is not declared; `fields` is the total column
  // count. See the `EventFormat` doc comment for why this form was chosen
  // over a friendlier ordered-name list.
  let empty = serde_json::to_value(EventFormat::empty()).unwrap();
  assert_eq!(
    empty,
    serde_json::json!({
      "slots": [null, null, null, null, null, null, null, null, null, null, null, null],
      "fields": 0,
    })
  );

  let ass = serde_json::to_value(EventFormat::ass()).unwrap();
  assert_eq!(
    ass,
    serde_json::json!({
      "slots": [null, null, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
      "fields": 10,
    })
  );

  let matroska = serde_json::to_value(EventFormat::matroska()).unwrap();
  assert_eq!(
    matroska,
    serde_json::json!({
      "slots": [0, null, 1, null, null, 2, 3, 4, 5, 6, 7, 8],
      "fields": 9,
    })
  );
}

#[test]
fn ass_event_format_omitted_field_falls_back_to_default() {
  // Omitting the document entirely: the whole value defaults to `ass()`.
  let format: EventFormat = serde_json::from_str("{}").unwrap();
  assert_eq!(format, EventFormat::default());
}

#[test]
fn ass_event_format_rejects_a_position_outside_the_declared_count() {
  // `slots` defaults to the `ass` preset (which reaches position 9); `fields`
  // alone says only 1 column is declared. A derived `Deserialize` would
  // accept this and silently misread every ASS event row parsed against it —
  // seven of the ass preset's ten fields, including `Text`, sit past the one
  // column `fields` admits.
  let err = serde_json::from_str::<EventFormat>(r#"{"fields": 1}"#).unwrap_err();
  assert!(
    err.to_string().contains("outside the 1 declared field"),
    "unexpected error: {err}"
  );
}

#[test]
fn ass_event_format_rejects_duplicate_positions() {
  // Fully specified, no defaulting involved: `slots[2]` (Layer) and
  // `slots[3]` (Start) both claim position 0.
  let err = serde_json::from_str::<EventFormat>(
    r#"{"slots": [null, null, 0, 0, null, null, null, null, null, null, null, null], "fields": 2}"#,
  )
  .unwrap_err();
  assert!(
    err.to_string().contains("repeats position 0"),
    "unexpected error: {err}"
  );
}

#[test]
fn ass_event_format_accepts_a_defaulted_document_that_is_still_valid() {
  // Omitting `fields` alone leaves it at the default preset's own count
  // (10), which is exactly what `slots` (also defaulted) already declares —
  // a defaulted-but-valid combination is still accepted.
  let format: EventFormat =
    serde_json::from_str(r#"{"slots": [null, null, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9]}"#).unwrap();
  assert_eq!(format, EventFormat::ass());
}

/// Captures the struct name a `Deserialize` implementation asks for, without
/// needing a name-sensitive format crate (RON, for instance) as a
/// dependency: `deserialize_struct`'s first argument is exactly what such a
/// format uses to find the value it should decode.
struct NameProbe;

impl<'de> serde::Deserializer<'de> for NameProbe {
  type Error = serde::de::value::Error;

  fn deserialize_struct<V>(
    self,
    name: &'static str,
    _fields: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: serde::de::Visitor<'de>,
  {
    use serde::de::Error as _;
    Err(Self::Error::custom(format!("probe:{name}")))
  }

  fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
  where
    V: serde::de::Visitor<'de>,
  {
    use serde::de::Error as _;
    Err(Self::Error::custom(
      "probe: deserialize_any reached, expected deserialize_struct",
    ))
  }

  serde::forward_to_deserialize_any! {
      bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
      bytes byte_buf option unit unit_struct newtype_struct seq tuple
      tuple_struct map enum identifier ignored_any
  }
}

#[test]
fn ass_event_format_deserialize_requests_its_own_serde_name() {
  // The derived `Serialize` above emits `serialize_struct("EventFormat",
  // ...)`. The handwritten `Deserialize` delegates to a helper type only it
  // uses; that helper must request the same name, or a name-sensitive
  // format round-trips EventFormat's own output to a "no struct named ..."
  // error rather than a value — caught in review before it shipped.
  let err = <EventFormat as serde::Deserialize>::deserialize(NameProbe).unwrap_err();
  assert_eq!(err.to_string(), "probe:EventFormat");
}

// ---------------------------------------------------------------------------
// vtt::cue::Options — only exists with `alloc`/`std`, like the type itself.
// ---------------------------------------------------------------------------

#[cfg(any(feature = "alloc", feature = "std"))]
mod vtt_cue_options {
  use fasrt::vtt::cue::{DEFAULT_MAX_DEPTH, Options as CueOptions};

  use super::round_trip;

  #[test]
  fn round_trips() {
    round_trip(CueOptions::new());
    round_trip(CueOptions::new().with_max_depth(4));
    round_trip(CueOptions::new().with_max_depth(0));
  }

  #[test]
  fn default_matches_new_and_round_trips() {
    assert_eq!(CueOptions::default(), CueOptions::new());
    round_trip(CueOptions::default());
  }

  #[test]
  fn golden() {
    let json = serde_json::to_value(CueOptions::new()).unwrap();
    assert_eq!(json, serde_json::json!({ "max_depth": DEFAULT_MAX_DEPTH }));
  }

  #[test]
  fn omitted_field_falls_back_to_default() {
    let opts: CueOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(opts, CueOptions::default());
  }
}
