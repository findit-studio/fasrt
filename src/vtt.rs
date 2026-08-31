use logos::{Lexer, Logos};

use crate::{
  error::*,
  types::{Millisecond, Minute, Second},
  utils::Lines,
};

pub use types::{
  Align, Anchor, Block, Cue, CueId, CueOptions, Header, Hour, Line, LineAlign, LineValue,
  Percentage, Position, PositionAlign, Region, RegionId, Scroll, Size, Timestamp, Vertical,
};

mod types;

/// Cue text parsing (tags, entities, timestamps).
pub mod cue;

/// HTML5 named character reference table (auto-generated).
#[cfg(any(feature = "alloc", feature = "std"))]
mod html5_entities;

/// The error type for parsing WebVTT files.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseVttError {
  /// An error occurred while parsing the hour component of a timestamp.
  #[error(transparent)]
  ParseMinute(#[from] ParseMinuteError),
  /// An error occurred while parsing the second component of a timestamp.
  #[error(transparent)]
  ParseSecond(#[from] ParseSecondError),
  /// An error occurred while parsing the hour component of a timestamp.
  #[error(transparent)]
  ParseHour(#[from] ParseHourError),
  /// An error occurred while parsing the millisecond component of a timestamp.
  #[error(transparent)]
  ParseMillisecond(#[from] ParseMillisecondError),

  /// The file does not start with the required `WEBVTT` signature.
  #[error("missing WEBVTT signature")]
  MissingSignature,

  /// The character after `WEBVTT` is not a space, tab, or newline.
  #[error("invalid character after WEBVTT signature")]
  InvalidSignature,

  /// A timestamp could not be parsed.
  #[error("invalid timestamp: {0}")]
  InvalidTimestamp(TimestampError),

  /// The timing line is malformed (missing `-->`).
  #[error("invalid timing line: missing '-->' separator")]
  InvalidTimingLine,

  /// The end timestamp is missing after `-->`.
  #[error("unclosed duration, missing end timestamp")]
  UnclosedDuration,

  /// An unknown error occurred.
  #[error("unexpected token: {0}")]
  Unknown(&'static str),
}

impl Default for ParseVttError {
  fn default() -> Self {
    Self::Unknown("unknown lexer error")
  }
}

/// Token produced by the WebVTT lexer.
#[derive(Debug, Logos, PartialEq)]
#[logos(error = ParseVttError)]
enum Token {
  /// A WebVTT timing line with optional hours:
  /// `[HH:]MM:SS.mmm --> [HH:]MM:SS.mmm`
  ///
  /// This matches the timing portion only; cue settings that follow
  /// on the same line are parsed separately from the remainder.
  ///
  /// Hours accept 1+ digits per the W3C spec.
  /// Whitespace around `-->` includes space, tab, and form feed (U+000C).
  #[regex(
    r"(?:[0-9]+:)?[0-5][0-9]:[0-5][0-9]\.[0-9]{3}[ \t\x0C]+-->[ \t\x0C]+(?:[0-9]+:)?[0-5][0-9]:[0-5][0-9]\.[0-9]{3}",
    parse_timing,
  )]
  TimingLine((Timestamp, Timestamp)),
}

#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_timing(lex: &mut Lexer<'_, Token>) -> Result<(Timestamp, Timestamp), ParseVttError> {
  let s = lex.slice();
  let (start_str, end_str) = split_arrow(s)?;
  let start = parse_timestamp(start_str)?;
  let end = parse_timestamp(end_str)?;
  Ok((start, end))
}

/// Split a timing line on `-->`, trimming whitespace around each part.
#[cfg_attr(not(tarpaulin), inline(always))]
fn split_arrow(s: &str) -> Result<(&str, &str), ParseVttError> {
  let arrow = s.find("-->").ok_or(ParseVttError::InvalidTimingLine)?;
  let start = s[..arrow].trim();
  let end = s[arrow + 3..].trim();
  if start.is_empty() || end.is_empty() {
    return Err(ParseVttError::UnclosedDuration);
  }
  Ok((start, end))
}

/// Parse a timestamp in either long form `HH:MM:SS.mmm` or short form
/// `MM:SS.mmm` using direct byte extraction.
///
/// The logos regex guarantees the format, so we can extract fields by
/// fixed offsets from the end: `mmm`(3) `.`(1) `SS`(2) `:`(1) `MM`(2)
/// = 9 fixed bytes. If there's a `:`(1) before that, everything before
/// it is the hours component.
///
/// # Safety assumption
///
/// Callers **must** ensure that `s` has already been validated by a
/// logos regex (which guarantees digit/separator placement).
/// For loosely-validated input (e.g. unterminated cue-text tags), use
/// [`parse_timestamp_cue`] instead.
#[inline]
pub(crate) fn parse_timestamp(s: &str) -> Result<Timestamp, ParseVttError> {
  let b = s.as_bytes();
  let len = b.len();
  if len < 9 {
    return Err(ParseVttError::InvalidTimestamp(
      TimestampError::InvalidLength,
    ));
  }
  let millis = Millisecond(vtt_digit3(&b[len - 3..]));
  let seconds = Second(vtt_digit2(&b[len - 6..len - 4]));
  let minutes = Minute(vtt_digit2(&b[len - 9..len - 7]));
  let hours = if len > 9 {
    let hour_str = &s[..len - 10];
    parse_vtt_hour_bytes(hour_str.as_bytes())?
  } else {
    Hour::new()
  };
  Ok(Timestamp::from_hmsm(hours, minutes, seconds, millis))
}

/// Parse a timestamp from cue-text, where input is only loosely validated
/// by the regex `<[0-9:]+\.[0-9]{3}>`.
///
/// Unlike [`parse_timestamp`], this validates separators and digit bytes
/// before doing any arithmetic, so it is safe for untrusted input.
#[inline]
pub(crate) fn parse_timestamp_cue(s: &str) -> Result<Timestamp, ParseVttError> {
  let b = s.as_bytes();
  let len = b.len();
  // Short form `MM:SS.mmm` = 9 bytes (0 hour digits).
  // Long form `H+:MM:SS.mmm` = 12..=29 bytes (1..=20 hour digits).
  // Reject lengths outside [9, 29] and the gap [10, 11].
  if !(9..=29).contains(&len) || (len > 9 && len < 12) {
    return Err(ParseVttError::InvalidTimestamp(
      TimestampError::InvalidLength,
    ));
  }
  // Validate separators.
  if b[len - 4] != b'.' || b[len - 7] != b':' {
    return Err(ParseVttError::InvalidTimestamp(
      TimestampError::InvalidFormat,
    ));
  }
  // Validate digit bytes before arithmetic.
  let millis_val = vtt_digit3_checked(&b[len - 3..]).ok_or(ParseVttError::InvalidTimestamp(
    TimestampError::InvalidDigits,
  ))?;
  let seconds_val = vtt_digit2_checked(&b[len - 6..len - 4]).ok_or(
    ParseVttError::InvalidTimestamp(TimestampError::InvalidDigits),
  )?;
  let minutes_val = vtt_digit2_checked(&b[len - 9..len - 7]).ok_or(
    ParseVttError::InvalidTimestamp(TimestampError::InvalidDigits),
  )?;
  let millis = Millisecond::try_with(millis_val).ok_or(ParseVttError::ParseMillisecond(
    ParseMillisecondError::Overflow(millis_val),
  ))?;
  let seconds = Second::try_with(seconds_val).ok_or(ParseVttError::ParseSecond(
    ParseSecondError::Overflow(seconds_val),
  ))?;
  let minutes = Minute::try_with(minutes_val).ok_or(ParseVttError::ParseMinute(
    ParseMinuteError::Overflow(minutes_val),
  ))?;
  let hours = if len >= 12 {
    if b[len - 10] != b':' {
      return Err(ParseVttError::InvalidTimestamp(
        TimestampError::InvalidFormat,
      ));
    }
    let hour_bytes = &b[..len - 10];
    parse_vtt_hour_bytes(hour_bytes)?
  } else {
    Hour::new()
  };
  Ok(Timestamp::from_hmsm(hours, minutes, seconds, millis))
}

/// Parse VTT hour bytes (variable-length, unbounded u64).
///
/// Uses checked arithmetic to prevent overflow on extremely large
/// hour values from untrusted input.
#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_vtt_hour_bytes(b: &[u8]) -> Result<Hour, ParseHourError> {
  let mut val: u64 = 0;
  for &byte in b {
    if !byte.is_ascii_digit() {
      return Err(ParseHourError::NotPadded);
    }
    val = val
      .checked_mul(10)
      .and_then(|v| v.checked_add((byte - b'0') as u64))
      .ok_or(ParseHourError::HourOverflow)?;
  }
  Ok(Hour(val))
}

/// Unchecked 2-digit extraction. Caller must guarantee ASCII digits.
///
/// Only called from [`parse_timestamp`], which requires logos-regex-validated input.
/// Debug assertions catch misuse during development.
#[cfg_attr(not(tarpaulin), inline(always))]
fn vtt_digit2(b: &[u8]) -> u8 {
  debug_assert!(b[0].is_ascii_digit() && b[1].is_ascii_digit());
  (b[0] - b'0') * 10 + (b[1] - b'0')
}

/// Unchecked 3-digit extraction. Caller must guarantee ASCII digits.
///
/// Only called from [`parse_timestamp`], which requires logos-regex-validated input.
/// Debug assertions catch misuse during development.
#[cfg_attr(not(tarpaulin), inline(always))]
fn vtt_digit3(b: &[u8]) -> u16 {
  debug_assert!(b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2].is_ascii_digit());
  (b[0] - b'0') as u16 * 100 + (b[1] - b'0') as u16 * 10 + (b[2] - b'0') as u16
}

/// Checked 2-digit extraction. Returns `None` if any byte is not an ASCII digit.
#[cfg_attr(not(tarpaulin), inline(always))]
fn vtt_digit2_checked(b: &[u8]) -> Option<u8> {
  let d0 = b[0].wrapping_sub(b'0');
  let d1 = b[1].wrapping_sub(b'0');
  if d0 > 9 || d1 > 9 {
    return None;
  }
  Some(d0 * 10 + d1)
}

/// Checked 3-digit extraction. Returns `None` if any byte is not an ASCII digit.
#[cfg_attr(not(tarpaulin), inline(always))]
fn vtt_digit3_checked(b: &[u8]) -> Option<u16> {
  let d0 = b[0].wrapping_sub(b'0');
  let d1 = b[1].wrapping_sub(b'0');
  let d2 = b[2].wrapping_sub(b'0');
  if d0 > 9 || d1 > 9 || d2 > 9 {
    return None;
  }
  Some(d0 as u16 * 100 + d1 as u16 * 10 + d2 as u16)
}

/// Lex the next token from a line. Returns `Ok(None)` when the line
/// produces no tokens.
#[cfg_attr(not(tarpaulin), inline(always))]
fn lex(line: &str) -> Result<Option<(Token, usize)>, ParseVttError> {
  let mut lexer = Token::lexer(line);
  match lexer.next() {
    Some(result) => result.map(|t| Some((t, lexer.span().end))),
    None => Ok(None),
  }
}

/// Returns `true` if the line looks like a WebVTT timing line (contains `-->`).
#[cfg_attr(not(tarpaulin), inline(always))]
fn is_timing_line(line: &str) -> bool {
  line.contains("-->")
}

/// Strip BOM and leading VTT whitespace (space, tab, form feed).
#[cfg_attr(not(tarpaulin), inline(always))]
fn strip_leading(line: &str) -> &str {
  line
    .trim_start_matches('\u{feff}')
    .trim_start_matches([' ', '\t', '\x0c'])
}

/// Parse cue settings from the remainder of a timing line.
fn parse_cue_settings<'a>(s: &'a str) -> CueOptions<'a> {
  let mut settings = CueOptions::default();
  if s.is_empty() {
    return settings;
  }

  for token in s.split([' ', '\t', '\x0c']) {
    if token.is_empty() {
      continue;
    }
    let Some((key, value)) = token.split_once(':') else {
      continue;
    };
    if value.is_empty() {
      // Empty value stops settings parsing per the spec
      break;
    }
    match key {
      "vertical" => match value {
        "rl" => {
          settings.set_vertical(Vertical::Rl);
        }
        "lr" => {
          settings.set_vertical(Vertical::Lr);
        }
        _ => {
          continue;
        }
      },
      "line" => {
        // value can be `N%`, `N`, optionally followed by `,start|center|end`
        if let Some((val_str, align_str)) = value.split_once(',') {
          let alignment = match align_str {
            "start" => LineAlign::Start,
            "center" => LineAlign::Center,
            "end" => LineAlign::End,
            _ => {
              continue;
            }
          };
          if let Some(v) = parse_line_value(val_str) {
            settings.set_line(Line::with_alignment(v, alignment));
          }
        } else if let Some(v) = parse_line_value(value) {
          settings.set_line(Line::new(v));
        }
      }
      "position" => {
        if let Some((val_str, align_str)) = value.split_once(',') {
          let alignment = match align_str {
            "line-left" => PositionAlign::LineLeft,
            "center" => PositionAlign::Center,
            "line-right" => PositionAlign::LineRight,
            "auto" => PositionAlign::Auto,
            _ => {
              continue;
            }
          };
          if let Some(pct) = parse_percentage(val_str) {
            settings.set_position(Position::with_alignment(pct, alignment));
          }
        } else if let Some(pct) = parse_percentage(value) {
          settings.set_position(Position::new(pct));
        }
      }
      "size" => {
        if let Some(pct) = parse_percentage(value) {
          settings.set_size(Size::new(pct));
        }
      }
      "align" => match value {
        "start" => {
          settings.set_align(Align::Start);
        }
        "center" => {
          settings.set_align(Align::Center);
        }
        "end" => {
          settings.set_align(Align::End);
        }
        "left" => {
          settings.set_align(Align::Left);
        }
        "right" => {
          settings.set_align(Align::Right);
        }
        _ => {
          continue;
        }
      },
      "region" => {
        settings.set_region(RegionId::new(value));
      }
      _ => {
        // Unknown settings are ignored per the spec
      }
    }
  }

  settings
}

/// Parse a percentage value like "50%" or "50.5%".
#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_percentage(s: &str) -> Option<Percentage> {
  let s = s.strip_suffix('%')?;
  let n: f64 = s.parse().ok()?;
  Percentage::try_with(n)
}

/// Parse a line value: either `N%` (percentage) or `N` (line number, possibly negative).
#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_line_value(s: &str) -> Option<LineValue> {
  if let Some(pct) = parse_percentage(s) {
    Some(LineValue::Percentage(pct))
  } else {
    s.parse().ok().map(LineValue::Number)
  }
}

/// Parse an anchor value like "50%,100%".
#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_anchor(s: &str) -> Option<Anchor> {
  let (x_str, y_str) = s.split_once(',')?;
  let x = parse_percentage(x_str)?;
  let y = parse_percentage(y_str)?;
  Some(Anchor::new(x, y))
}

/// Parse REGION block settings from the body text.
fn parse_region_settings<'a>(body: &'a str) -> Region<'a> {
  let mut region = Region::default();

  for line in body.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }
    let Some((key, value)) = line.split_once(':') else {
      continue;
    };
    if value.is_empty() {
      continue;
    }
    match key {
      "id" => {
        if !value.contains("-->") {
          region.set_id(RegionId::new(value));
        }
      }
      "width" => {
        if let Some(pct) = parse_percentage(value) {
          region.set_width(pct);
        }
      }
      "lines" => {
        if let Ok(n) = value.parse::<u32>() {
          region.set_lines(n);
        }
      }
      "regionanchor" => {
        if let Some(anchor) = parse_anchor(value) {
          region.set_region_anchor(anchor);
        }
      }
      "viewportanchor" => {
        if let Some(anchor) = parse_anchor(value) {
          region.set_viewport_anchor(anchor);
        }
      }
      "scroll" => {
        if value == "up" {
          region.set_scroll(Scroll::Up);
        }
      }
      _ => {}
    }
  }

  region
}

/// Format cue settings to a string for writing.
#[cfg(feature = "std")]
fn format_cue_settings(settings: &CueOptions<'_>, buf: &mut std::vec::Vec<u8>) {
  use std::io::Write as _;

  if let Some(v) = settings.vertical() {
    let _ = write!(buf, " vertical:{v}");
  }
  if let Some(line) = settings.line() {
    match line.value() {
      LineValue::Percentage(p) => {
        let _ = write!(buf, " line:{p}%");
      }
      LineValue::Number(n) => {
        let _ = write!(buf, " line:{n}");
      }
    }
    if let Some(align) = line.alignment() {
      let _ = write!(buf, ",{align}");
    }
  }
  if let Some(pos) = settings.position() {
    let _ = write!(buf, " position:{}%", pos.value());
    if let Some(align) = pos.alignment() {
      let _ = write!(buf, ",{align}");
    }
  }
  if let Some(size) = settings.size() {
    let _ = write!(buf, " size:{}%", size.value());
  }
  if let Some(align) = settings.align() {
    let _ = write!(buf, " align:{align}");
  }
  if let Some(region) = settings.region() {
    let _ = write!(buf, " region:{region}");
  }
}

/// Format a region definition for writing.
#[cfg(feature = "std")]
fn format_region<W: std::io::Write>(region: &Region<'_>, w: &mut W) -> std::io::Result<()> {
  let id = region.id().as_str();
  if !id.is_empty() {
    writeln!(w, "id:{id}")?;
  }
  let default = Region::default();
  if region.width() != default.width() {
    writeln!(w, "width:{}%", region.width())?;
  }
  if region.lines() != default.lines() {
    writeln!(w, "lines:{}", region.lines())?;
  }
  if region.region_anchor() != default.region_anchor() {
    let a = region.region_anchor();
    writeln!(w, "regionanchor:{}%,{}%", a.x(), a.y())?;
  }
  if region.viewport_anchor() != default.viewport_anchor() {
    let a = region.viewport_anchor();
    writeln!(w, "viewportanchor:{}%,{}%", a.x(), a.y())?;
  }
  if region.scroll() != default.scroll() {
    writeln!(w, "scroll:{}", region.scroll())?;
  }
  Ok(())
}

/// State machine states for the WebVTT parser.
enum State<'a> {
  /// Expecting the WEBVTT signature line.
  Signature,
  /// After signature, collecting optional header text until first blank line.
  Header,
  /// Expecting a block start (cue, NOTE, STYLE, REGION) or blank lines.
  BlockStart,
  /// Collecting cue body text.
  CueBody(CueBodyState<'a>),
  /// Collecting NOTE body text.
  NoteBody(usize, usize),
  /// Collecting STYLE body text.
  StyleBody(usize, usize),
  /// Collecting REGION body text.
  RegionBody(usize, usize),
  /// The iterator is exhausted or encountered an error.
  Done,
}

struct CueBodyState<'a> {
  header: Header<'a>,
  start: usize,
  end: usize,
}

impl<'a> CueBodyState<'a> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn new(header: Header<'a>, start: usize, end: usize) -> Self {
    Self { header, start, end }
  }
}

/// A lazy, zero-copy WebVTT parser that yields blocks as an iterator.
///
/// Created via [`Parser::new`]. Each call to [`Iterator::next`] yields the
/// next parsed [`Block`], or an error if the input is malformed.
///
/// The parser handles file-level structure (signature, timing lines, cue
/// settings, NOTE/STYLE/REGION blocks) per the W3C WebVTT spec.  Cue text
/// content parsing (tags, entities, tree building) is out of scope — cue
/// bodies are returned as raw text slices.
///
/// # Example
///
/// ```
/// # #[cfg(not(any(feature = "std", feature = "alloc")))]
/// # fn main() {}
/// # #[cfg(any(feature = "std", feature = "alloc"))]
/// # fn main() {
/// # use std::vec::Vec;
///
/// use fasrt::vtt::{Parser, Block};
///
/// let vtt = "\
/// WEBVTT
///
/// 00:01.000 --> 00:04.000
/// Hello world!
///
/// 00:05.000 --> 00:08.000
/// Goodbye world!
/// ";
///
/// let blocks: Vec<_> = Parser::new(vtt).collect::<Result<_, _>>().unwrap();
/// assert_eq!(blocks.len(), 2);
/// match &blocks[0] {
///   Block::Cue(cue) => assert_eq!(*cue.body_ref(), "Hello world!"),
///   _ => panic!("expected cue"),
/// }
/// # }
/// ```
pub struct Parser<'a> {
  input: &'a str,
  lines: Lines<'a>,
  state: State<'a>,
  /// Whether we've seen the first cue. STYLE/REGION blocks are only
  /// allowed before the first cue per the spec.
  seen_cue: bool,
  /// A line to be reprocessed in the next iteration (used for error
  /// recovery when `-->` appears in cue body or a timing line fails).
  pending_line: Option<&'a str>,
}

impl<'a> Parser<'a> {
  /// Create a new WebVTT parser for the given input.
  pub fn new(input: &'a str) -> Self {
    Self {
      input,
      lines: Lines::new(input),
      state: State::Signature,
      seen_cue: false,
      pending_line: None,
    }
  }

  /// Get the next line, using pending_line if available.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn next_line(&mut self) -> Option<&'a str> {
    self.pending_line.take().or_else(|| self.lines.next())
  }

  /// Skip lines until a blank line or a line containing `-->`.
  /// If a `-->` line is found, it is stored as pending for reprocessing.
  fn skip_to_block_boundary(&mut self) {
    loop {
      let Some(line) = self.lines.next() else {
        return;
      };
      if line.is_empty() {
        return;
      }
      if is_timing_line(line) {
        self.pending_line = Some(line);
        return;
      }
    }
  }
}

/// Try to lex a timing line. On success, returns the parsed header and
/// the byte offset within `self.input` where the body starts.
fn try_parse_timing<'a>(line: &'a str, input: &'a str) -> Option<(Header<'a>, usize)> {
  let stripped = strip_leading(line);
  if !is_timing_line(stripped) {
    return None;
  }

  match lex(stripped) {
    Ok(Some((Token::TimingLine((start, end)), matched_end))) => {
      let mut header = Header::new(start, end);
      let settings_str = stripped[matched_end..].trim();
      let settings = parse_cue_settings(settings_str);
      if settings != CueOptions::default() {
        header.set_settings(settings);
      }
      let offset = line.as_ptr() as usize - input.as_ptr() as usize + line.len();
      Some((header, offset))
    }
    _ => None,
  }
}

impl<'a> Iterator for Parser<'a> {
  type Item = Result<Block<'a, &'a str>, ParseVttError>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.state {
        State::Done => return None,

        State::Signature => {
          let Some(line) = self.lines.next() else {
            self.state = State::Done;
            return Some(Err(ParseVttError::MissingSignature));
          };

          // Strip optional BOM
          let line = line.trim_start_matches('\u{feff}');

          // Must start with "WEBVTT"
          if !line.starts_with("WEBVTT") {
            self.state = State::Done;
            return Some(Err(ParseVttError::MissingSignature));
          }

          // After "WEBVTT", the next character (if any) must be space, tab, or nothing.
          let rest = &line[6..];
          if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
            self.state = State::Done;
            return Some(Err(ParseVttError::InvalidSignature));
          }

          self.state = State::Header;
        }

        State::Header => {
          // Skip header text until we hit a blank line or a --> line
          let Some(line) = self.lines.next() else {
            self.state = State::Done;
            return None;
          };

          if line.is_empty() {
            self.state = State::BlockStart;
          } else if is_timing_line(line) {
            // Per the JS ref parser: if header contains -->, jump to block processing
            self.pending_line = Some(line);
            self.state = State::BlockStart;
          }
          // Otherwise skip header text lines
        }

        State::BlockStart => {
          let Some(line) = self.next_line() else {
            self.state = State::Done;
            return None;
          };

          let trimmed = line.trim_start_matches('\u{feff}');
          if trimmed.is_empty() {
            continue;
          }

          // Check for NOTE block
          if trimmed == "NOTE" || trimmed.starts_with("NOTE ") || trimmed.starts_with("NOTE\t") {
            let after_note = if trimmed == "NOTE" {
              ""
            } else {
              trimmed[5..].trim_start()
            };

            if after_note.is_empty() {
              let offset = line.as_ptr() as usize - self.input.as_ptr() as usize + line.len();
              self.state = State::NoteBody(offset, offset);
            } else {
              let body_start = after_note.as_ptr() as usize - self.input.as_ptr() as usize;
              let body_end = body_start + after_note.len();
              self.state = State::NoteBody(body_start, body_end);
            }
            continue;
          }

          // Check for STYLE block (only before first cue)
          if (trimmed == "STYLE" || trimmed.starts_with("STYLE ") || trimmed.starts_with("STYLE\t"))
            && !self.seen_cue
          {
            let offset = line.as_ptr() as usize - self.input.as_ptr() as usize + line.len();
            self.state = State::StyleBody(offset, offset);
            continue;
          }

          // Check for REGION block (only before first cue)
          if (trimmed == "REGION"
            || trimmed.starts_with("REGION ")
            || trimmed.starts_with("REGION\t"))
            && !self.seen_cue
          {
            let offset = line.as_ptr() as usize - self.input.as_ptr() as usize + line.len();
            self.state = State::RegionBody(offset, offset);
            continue;
          }

          // Check if this line is a timing line (cue without identifier)
          if is_timing_line(trimmed) {
            if let Some((header, offset)) = try_parse_timing(line, self.input) {
              self.state = State::CueBody(CueBodyState::new(header, offset, offset));
              self.seen_cue = true;
            } else {
              // Timing parse failed — skip to next block boundary
              self.skip_to_block_boundary();
            }
            continue;
          }

          // This line could be a cue identifier. The next line should be a timing line.
          let identifier_line = trimmed;
          let Some(next_line) = self.lines.next() else {
            self.state = State::Done;
            return None;
          };

          if is_timing_line(next_line) {
            if let Some((mut header, offset)) = try_parse_timing(next_line, self.input) {
              header.set_identifier(CueId::new(identifier_line));

              self.state = State::CueBody(CueBodyState::new(header, offset, offset));
              self.seen_cue = true;
            } else {
              // Timing parse failed after identifier — skip to next block boundary
              self.skip_to_block_boundary();
            }
          } else if next_line.is_empty() {
            // Identifier line followed by blank — not a valid cue, skip
            continue;
          } else {
            // Non-timing, non-blank line after identifier.
            // Per the JS ref parser, this line is reprocessed as a new block start.
            self.pending_line = Some(next_line);
            continue;
          }
        }
        State::CueBody(ref mut body) => {
          let CueBodyState { header, start, end } = body;
          let Some(line) = self.lines.next() else {
            let body_text = body_slice(self.input, *start, *end);
            let entry = Cue::new(header.clone(), body_text);
            self.state = State::Done;
            return Some(Ok(Block::Cue(entry)));
          };

          if line.is_empty() {
            let body_text = body_slice(self.input, *start, *end);
            let entry = Cue::new(header.clone(), body_text);
            self.state = State::BlockStart;
            return Some(Ok(Block::Cue(entry)));
          }

          // Per the W3C spec, if a body line contains `-->`, the cue ends
          // and the line is reprocessed as a potential timing line.
          if is_timing_line(line) {
            let body_text = body_slice(self.input, *start, *end);
            let entry = Cue::new(header.clone(), body_text);
            self.pending_line = Some(line);
            self.state = State::BlockStart;
            return Some(Ok(Block::Cue(entry)));
          }

          let line_offset = line.as_ptr() as usize - self.input.as_ptr() as usize;
          if *start == *end {
            *start = line_offset;
          }
          *end = line_offset + line.len();
        }

        State::NoteBody(ref mut start, ref mut end) => {
          let Some(line) = self.lines.next() else {
            let body_text = body_slice(self.input, *start, *end);
            self.state = State::Done;
            return Some(Ok(Block::Note(body_text)));
          };

          if line.is_empty() {
            let body_text = body_slice(self.input, *start, *end);
            self.state = State::BlockStart;
            return Some(Ok(Block::Note(body_text)));
          }

          let line_offset = line.as_ptr() as usize - self.input.as_ptr() as usize;
          if *start == *end {
            *start = line_offset;
          }
          *end = line_offset + line.len();
        }

        State::StyleBody(ref mut start, ref mut end) => {
          let Some(line) = self.lines.next() else {
            let body_text = body_slice(self.input, *start, *end);
            self.state = State::Done;
            return Some(Ok(Block::Style(body_text)));
          };

          if line.is_empty() {
            let body_text = body_slice(self.input, *start, *end);
            self.state = State::BlockStart;
            return Some(Ok(Block::Style(body_text)));
          }

          let line_offset = line.as_ptr() as usize - self.input.as_ptr() as usize;
          if *start == *end {
            *start = line_offset;
          }
          *end = line_offset + line.len();
        }

        State::RegionBody(ref mut start, ref mut end) => {
          let Some(line) = self.lines.next() else {
            let body_text = body_slice(self.input, *start, *end);
            let region = parse_region_settings(body_text);
            self.state = State::Done;
            return Some(Ok(Block::Region(region)));
          };

          if line.is_empty() {
            let body_text = body_slice(self.input, *start, *end);
            let region = parse_region_settings(body_text);
            self.state = State::BlockStart;
            return Some(Ok(Block::Region(region)));
          }

          let line_offset = line.as_ptr() as usize - self.input.as_ptr() as usize;
          if *start == *end {
            *start = line_offset;
          }
          *end = line_offset + line.len();
        }
      }
    }
  }
}

/// Extract a body slice from the input, or empty string if no text lines.
#[cfg_attr(not(tarpaulin), inline(always))]
fn body_slice(input: &str, start: usize, end: usize) -> &str {
  if start >= end { "" } else { &input[start..end] }
}

/// Types that can serve as the body of a WebVTT cue when writing.
///
/// Implemented for [`str`], [`String`], [`CueText`](cue::CueText),
/// and `[`[`Node`](cue::Node)`]` slices.  Used by
/// [`Writer::write_cue`] so you can pass any of these as the cue body.
///
/// # Example
///
/// ```
/// use fasrt::vtt::{Writer, Header, Timestamp, Hour, CueBody};
/// use fasrt::vtt::cue::{CueText, Node, CueStr, TagNode, Tag};
/// use fasrt::types::*;
///
/// let header = Header::new(
///   Timestamp::from_hmsm(Hour::new(), Minute::with(0), Second::with(1), Millisecond::with(0)),
///   Timestamp::from_hmsm(Hour::new(), Minute::with(0), Second::with(4), Millisecond::with(0)),
/// );
///
/// let mut buf = Vec::new();
/// let mut writer = Writer::new(&mut buf);
///
/// // Write with a raw string body
/// writer.write_cue(&header, "Hello world!").unwrap();
///
/// // Write with a CueText DOM body
/// let body = CueText::new(vec![
///   Node::Tag(TagNode::new(Tag::Bold)
///     .with_children(vec![Node::Text(CueStr::borrowed("hello"))])),
/// ]);
/// writer.write_cue(&header, &body).unwrap();
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub trait CueBody: sealed::Sealed {
  /// Write this body's content to the given writer.
  fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()>;

  /// Whether this body is empty (produces no output).
  fn is_empty(&self) -> bool;
}

#[cfg(feature = "std")]
mod sealed {
  use super::cue::*;

  pub trait Sealed {}

  impl Sealed for str {}
  impl Sealed for std::string::String {}
  impl<'a, C: Nodes<'a>> Sealed for CueText<'a, C> {}
  impl Sealed for [Node<'_>] {}
  impl Sealed for Node<'_> {}
  impl<T: Sealed> Sealed for Option<T> {}
  impl Sealed for super::Timestamp {}
  impl Sealed for super::cue::CueStr<'_> {}
  impl<C> Sealed for super::cue::TagNode<'_, C> {}
}

#[cfg(feature = "std")]
const _: () = {
  use cue::{CueText, Node, Nodes};

  impl CueBody for str {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      w.write_all(self.as_bytes())
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      self.is_empty()
    }
  }

  impl CueBody for std::string::String {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      w.write_all(self.as_bytes())
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      self.is_empty()
    }
  }

  impl<'a, C: Nodes<'a>> CueBody for CueText<'a, C> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      write!(w, "{}", self)
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      self.children().as_nodes().is_empty()
    }
  }

  impl CueBody for [Node<'_>] {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      for node in self {
        write!(w, "{}", node)?;
      }
      Ok(())
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      self.is_empty()
    }
  }

  impl CueBody for Node<'_> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      write!(w, "{}", self)
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      false
    }
  }

  impl CueBody for Timestamp {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      w.write_all(self.encode().as_str().as_bytes())
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      false
    }
  }

  impl CueBody for cue::CueStr<'_> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      w.write_all(self.normalize().as_bytes())
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      false
    }
  }

  impl<'a, C> CueBody for cue::TagNode<'a, C>
  where
    C: AsRef<[Node<'a>]>,
  {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      write!(w, "{self}")
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      false
    }
  }

  impl<T: CueBody> CueBody for Option<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn write_body<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
      if let Some(node) = self {
        node.write_body(w)
      } else {
        Ok(())
      }
    }

    #[cfg_attr(not(tarpaulin), inline(always))]
    fn is_empty(&self) -> bool {
      self.as_ref().is_some_and(|t| t.is_empty())
    }
  }
};

/// A WebVTT file writer that writes blocks to a [`std::io::Write`] target.
///
/// # Example
///
/// ```
/// use fasrt::vtt::{Writer, Cue, Header, Timestamp, Block, Hour};
/// use fasrt::types::*;
///
/// let mut buf = Vec::new();
/// let mut writer = Writer::new(&mut buf);
///
/// let header = Header::new(
///   Timestamp::from_hmsm(Hour::new(), Minute::with(0), Second::with(1), Millisecond::with(0)),
///   Timestamp::from_hmsm(Hour::new(), Minute::with(0), Second::with(4), Millisecond::with(0)),
/// );
///
/// writer.write(&Block::Cue(Cue::new(header, "Hello world!"))).unwrap();
///
/// let output = String::from_utf8(buf).unwrap();
/// assert!(output.starts_with("WEBVTT\n"));
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct Writer<W> {
  inner: W,
  has_written_signature: bool,
  has_written_block: bool,
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
const _: () = {
  use std::io::{self, Write};

  impl<W: Write> Writer<W> {
    /// Create a new writer wrapping the given [`std::io::Write`] target.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub const fn new(inner: W) -> Self {
      Self {
        inner,
        has_written_signature: false,
        has_written_block: false,
      }
    }

    /// Write the WEBVTT signature with optional header text.
    ///
    /// This is called automatically on the first `write` call if not
    /// called explicitly.
    pub fn write_header(&mut self, header_text: Option<&str>) -> io::Result<()> {
      if self.has_written_signature {
        return Ok(());
      }
      self.has_written_signature = true;
      self.inner.write_all(b"WEBVTT")?;
      if let Some(text) = header_text
        && !text.is_empty()
      {
        self.inner.write_all(b" ")?;
        self.inner.write_all(text.as_bytes())?;
      }
      self.inner.write_all(b"\n")
    }

    /// Write a single WebVTT block.
    ///
    /// A blank line separator is emitted between blocks. The `WEBVTT`
    /// signature is automatically emitted before the first block.
    pub fn write<T: AsRef<str>>(&mut self, block: &Block<'_, T>) -> io::Result<()> {
      if !self.has_written_signature {
        self.write_header(None)?;
      }

      // Blank line before each block (separator after signature / between blocks)
      self.inner.write_all(b"\n")?;

      match block {
        Block::Cue(cue) => {
          let header = cue.header_ref();

          // Optional cue identifier
          if let Some(id) = header.identifier() {
            self.inner.write_all(id.as_str().as_bytes())?;
            self.inner.write_all(b"\n")?;
          }

          // Timing line
          self
            .inner
            .write_all(header.start().encode().as_str().as_bytes())?;
          self.inner.write_all(b" --> ")?;
          self
            .inner
            .write_all(header.end().encode().as_str().as_bytes())?;

          // Optional cue settings
          if let Some(settings) = header.settings() {
            let mut settings_buf = std::vec::Vec::new();
            format_cue_settings(settings, &mut settings_buf);
            self.inner.write_all(&settings_buf)?;
          }

          self.inner.write_all(b"\n")?;

          // Body
          let body = cue.body_ref().as_ref();
          if !body.is_empty() {
            self.inner.write_all(body.as_bytes())?;
            self.inner.write_all(b"\n")?;
          }
        }
        Block::Note(text) => {
          let text = text.as_ref();
          if text.is_empty() {
            self.inner.write_all(b"NOTE\n")?;
          } else {
            self.inner.write_all(b"NOTE\n")?;
            self.inner.write_all(text.as_bytes())?;
            self.inner.write_all(b"\n")?;
          }
        }
        Block::Style(text) => {
          self.inner.write_all(b"STYLE\n")?;
          let text = text.as_ref();
          if !text.is_empty() {
            self.inner.write_all(text.as_bytes())?;
            self.inner.write_all(b"\n")?;
          }
        }
        Block::Region(region) => {
          self.inner.write_all(b"REGION\n")?;
          format_region(region, &mut self.inner)?;
        }
      }

      self.has_written_block = true;
      Ok(())
    }

    /// Write a cue with any [`CueBody`] type as the body.
    ///
    /// This accepts raw strings, [`CueText`](cue::CueText) DOM trees,
    /// or `[`[`Node`](cue::Node)`]` slices.
    ///
    /// ```
    /// use fasrt::vtt::{Writer, Header, Timestamp, Hour};
    /// use fasrt::vtt::cue::{CueText, Node, CueStr, TagNode, Tag};
    /// use fasrt::types::*;
    ///
    /// let header = Header::new(
    ///   Timestamp::from_hmsm(Hour::new(), Minute::with(0), Second::with(1), Millisecond::with(0)),
    ///   Timestamp::from_hmsm(Hour::new(), Minute::with(0), Second::with(4), Millisecond::with(0)),
    /// );
    ///
    /// let mut buf = Vec::new();
    /// let mut writer = Writer::new(&mut buf);
    ///
    /// // Raw string
    /// writer.write_cue(&header, "Hello world!").unwrap();
    ///
    /// // CueText DOM
    /// let body = CueText::new(vec![
    ///   Node::Tag(TagNode::new(Tag::Lang)
    ///     .with_annotation(Some("en"))
    ///     .with_children(vec![Node::Text(CueStr::borrowed("hello"))])),
    /// ]);
    /// writer.write_cue(&header, &body).unwrap();
    ///
    /// // Node slice
    /// let nodes = vec![
    ///   Node::Tag(TagNode::new(Tag::Ruby).with_children(vec![
    ///     Node::Text(CueStr::borrowed("漢字")),
    ///     Node::Tag(TagNode::new(Tag::RubyText)
    ///       .with_children(vec![Node::Text(CueStr::borrowed("かんじ"))])),
    ///   ])),
    /// ];
    /// writer.write_cue(&header, nodes.as_slice()).unwrap();
    /// ```
    pub fn write_cue<B: CueBody + ?Sized>(
      &mut self,
      header: &Header<'_>,
      body: &B,
    ) -> io::Result<()> {
      if !self.has_written_signature {
        self.write_header(None)?;
      }

      self.inner.write_all(b"\n")?;

      // Optional cue identifier
      if let Some(id) = header.identifier() {
        self.inner.write_all(id.as_str().as_bytes())?;
        self.inner.write_all(b"\n")?;
      }

      // Timing line
      self
        .inner
        .write_all(header.start().encode().as_str().as_bytes())?;
      self.inner.write_all(b" --> ")?;
      self
        .inner
        .write_all(header.end().encode().as_str().as_bytes())?;

      // Optional cue settings
      if let Some(settings) = header.settings() {
        let mut settings_buf = std::vec::Vec::new();
        format_cue_settings(settings, &mut settings_buf);
        self.inner.write_all(&settings_buf)?;
      }

      self.inner.write_all(b"\n")?;

      // Body
      if !body.is_empty() {
        body.write_body(&mut self.inner)?;
        self.inner.write_all(b"\n")?;
      }

      self.has_written_block = true;
      Ok(())
    }

    /// Write all blocks from an iterator.
    pub fn write_all<'b, 'c, T, I>(&mut self, blocks: I) -> io::Result<()>
    where
      T: AsRef<str> + 'b,
      I: IntoIterator<Item = &'b Block<'c, T>>,
      'c: 'b,
    {
      for block in blocks {
        self.write(block)?;
      }
      Ok(())
    }

    /// Flush the underlying writer.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn flush(&mut self) -> io::Result<()> {
      self.inner.flush()
    }

    /// Consume the writer and return the inner [`std::io::Write`] target.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn into_inner(self) -> W {
      self.inner
    }
  }
};
