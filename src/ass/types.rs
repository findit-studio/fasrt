use core::{str::FromStr, time::Duration};

use derive_more::{Display, From, Into, IsVariant, TryUnwrap, Unwrap};

use crate::{
  error::{
    ParseCentisecondError, ParseHourError, ParseMinuteError, ParseSecondError, TimestampError,
  },
  types::{Buffer, Centisecond, Minute, Second},
  utils::u64_digits,
};

use super::{Options, ParseAssError};

/// The hour component of an ASS/SSA timestamp.
///
/// ASS/SSA hours are written as one or more digits and are **not**
/// zero-padded: the canonical form of zero is `0:00:00.00`.  This wraps a
/// `u64` with no upper bound.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
#[repr(transparent)]
pub struct Hour(pub(crate) u64);

impl FromStr for Hour {
  type Err = ParseHourError;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    parse_hour_bytes(s.as_bytes())
  }
}

impl Hour {
  /// Create a new `Hour` with value 0.
  ///
  /// ```rust
  /// use fasrt::ass::Hour;
  ///
  /// let hour = Hour::new();
  /// assert_eq!(hour.as_u64(), 0);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self(0)
  }

  /// Create a new `Hour` from a `u64`.
  ///
  /// ```rust
  /// use fasrt::ass::Hour;
  ///
  /// let hour = Hour::with(12);
  /// assert_eq!(hour.as_u64(), 12);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with(value: u64) -> Self {
    Self(value)
  }

  /// Returns the inner `u64` value.
  ///
  /// ```rust
  /// use fasrt::ass::Hour;
  ///
  /// assert_eq!(Hour::with(42).as_u64(), 42);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_u64(&self) -> u64 {
    self.0
  }
}

impl core::fmt::Display for Hour {
  /// Writes the hour without zero padding, as ASS/SSA does.
  ///
  /// ```rust
  /// use fasrt::ass::Hour;
  ///
  /// assert_eq!(Hour::new().to_string(), "0");
  /// assert_eq!(Hour::with(12).to_string(), "12");
  /// ```
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Parses ASS/SSA hour digits into an [`Hour`], with checked arithmetic.
#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_hour_bytes(bytes: &[u8]) -> Result<Hour, ParseHourError> {
  if bytes.is_empty() {
    return Err(ParseHourError::NotPadded);
  }
  let mut value: u64 = 0;
  for &byte in bytes {
    if !byte.is_ascii_digit() {
      return Err(ParseHourError::NotPadded);
    }
    value = value
      .checked_mul(10)
      .and_then(|v| v.checked_add((byte - b'0') as u64))
      .ok_or(ParseHourError::HourOverflow)?;
  }
  Ok(Hour(value))
}

/// An ASS/SSA timestamp with centisecond precision.
///
/// The wire form is `H:MM:SS.cc`: an unpadded hour of one or more digits, then
/// two-digit minutes and seconds, then a dot and exactly two digits of
/// centiseconds.
///
/// ```rust
/// use fasrt::ass::Timestamp;
///
/// let ts: Timestamp = "0:01:02.34".parse().unwrap();
/// assert_eq!(ts.encode().as_str(), "0:01:02.34");
/// ```
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{}", self.encode().as_str())]
pub struct Timestamp {
  /// Hours (unbounded, unpadded).
  hours: Hour,
  /// Minutes (0–59).
  minutes: Minute,
  /// Seconds (0–59).
  seconds: Second,
  /// Centiseconds (0–99).
  centis: Centisecond,
}

impl Default for Timestamp {
  /// ```rust
  /// use fasrt::ass::{Hour, Timestamp};
  ///
  /// let ts = Timestamp::default();
  /// assert_eq!(ts.hours(), Hour::new());
  /// assert_eq!(ts.encode().as_str(), "0:00:00.00");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn default() -> Self {
    Self::new()
  }
}

impl From<Duration> for Timestamp {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from(value: Duration) -> Self {
    Self::from_duration(value)
  }
}

impl FromStr for Timestamp {
  type Err = ParseAssError;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::parse(s)
  }
}

impl Timestamp {
  /// Create a new timestamp with all components set to zero.
  ///
  /// ```rust
  /// use fasrt::ass::Timestamp;
  ///
  /// assert_eq!(Timestamp::new().encode().as_str(), "0:00:00.00");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self::from_hmsc(
      Hour::new(),
      Minute::new(),
      Second::new(),
      Centisecond::new(),
    )
  }

  /// Create a timestamp from hours, minutes, seconds and centiseconds.
  ///
  /// ```rust
  /// use fasrt::ass::{Hour, Timestamp};
  /// use fasrt::types::{Centisecond, Minute, Second};
  ///
  /// let ts = Timestamp::from_hmsc(
  ///   Hour::with(1),
  ///   Minute::with(2),
  ///   Second::with(3),
  ///   Centisecond::with(4),
  /// );
  /// assert_eq!(ts.encode().as_str(), "1:02:03.04");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn from_hmsc(
    hours: Hour,
    minutes: Minute,
    seconds: Second,
    centis: Centisecond,
  ) -> Self {
    Self {
      hours,
      minutes,
      seconds,
      centis,
    }
  }

  /// Returns the hours component.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn hours(&self) -> Hour {
    self.hours
  }

  /// Returns the minutes component.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn minutes(&self) -> Minute {
    self.minutes
  }

  /// Returns the seconds component.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn seconds(&self) -> Second {
    self.seconds
  }

  /// Returns the centiseconds component.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn centis(&self) -> Centisecond {
    self.centis
  }

  /// Build a new timestamp with the hours field set to the given value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_hours(mut self, hours: Hour) -> Self {
    self.hours = hours;
    self
  }

  /// Build a new timestamp with the minutes field set to the given value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_minutes(mut self, minutes: Minute) -> Self {
    self.minutes = minutes;
    self
  }

  /// Build a new timestamp with the seconds field set to the given value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_seconds(mut self, seconds: Second) -> Self {
    self.seconds = seconds;
    self
  }

  /// Build a new timestamp with the centiseconds field set to the given value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_centis(mut self, centis: Centisecond) -> Self {
    self.centis = centis;
    self
  }

  /// Set the hours field of this timestamp.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_hours(&mut self, hours: Hour) -> &mut Self {
    self.hours = hours;
    self
  }

  /// Set the minutes field of this timestamp.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_minutes(&mut self, minutes: Minute) -> &mut Self {
    self.minutes = minutes;
    self
  }

  /// Set the seconds field of this timestamp.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_seconds(&mut self, seconds: Second) -> &mut Self {
    self.seconds = seconds;
    self
  }

  /// Set the centiseconds field of this timestamp.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_centis(&mut self, centis: Centisecond) -> &mut Self {
    self.centis = centis;
    self
  }

  /// Parse an ASS/SSA timestamp of the form `H:MM:SS.cc`.
  ///
  /// Every byte is validated before any arithmetic is performed, so this is
  /// safe for untrusted input and never panics.
  ///
  /// ```rust
  /// use fasrt::ass::Timestamp;
  /// use fasrt::types::{Centisecond, Minute, Second};
  ///
  /// let ts = Timestamp::parse("1:23:45.67").unwrap();
  /// assert_eq!(ts.minutes(), Minute::with(23));
  /// assert_eq!(ts.seconds(), Second::with(45));
  /// assert_eq!(ts.centis(), Centisecond::with(67));
  ///
  /// // More than one hour digit is accepted.
  /// assert!(Timestamp::parse("123:00:00.00").is_ok());
  /// // Milliseconds are not the ASS/SSA form.
  /// assert!(Timestamp::parse("0:00:01.000").is_err());
  /// ```
  pub fn parse(s: &str) -> Result<Self, ParseAssError> {
    let bytes = s.as_bytes();
    let len = bytes.len();

    // `H:MM:SS.cc` — nine fixed trailing bytes plus at least one hour digit.
    if len < 10 {
      return Err(ParseAssError::InvalidTimestamp(
        TimestampError::InvalidLength,
      ));
    }
    if bytes[len - 3] != b'.' || bytes[len - 6] != b':' || bytes[len - 9] != b':' {
      return Err(ParseAssError::InvalidTimestamp(
        TimestampError::InvalidFormat,
      ));
    }

    let centis_val = two_digits(&bytes[len - 2..]).ok_or(ParseAssError::InvalidTimestamp(
      TimestampError::InvalidDigits,
    ))?;
    let seconds_val = two_digits(&bytes[len - 5..len - 3]).ok_or(
      ParseAssError::InvalidTimestamp(TimestampError::InvalidDigits),
    )?;
    let minutes_val = two_digits(&bytes[len - 8..len - 6]).ok_or(
      ParseAssError::InvalidTimestamp(TimestampError::InvalidDigits),
    )?;

    let centis = Centisecond::try_with(centis_val).ok_or(ParseAssError::ParseCentisecond(
      ParseCentisecondError::Overflow(centis_val),
    ))?;
    let seconds = Second::try_with(seconds_val).ok_or(ParseAssError::ParseSecond(
      ParseSecondError::Overflow(seconds_val),
    ))?;
    let minutes = Minute::try_with(minutes_val).ok_or(ParseAssError::ParseMinute(
      ParseMinuteError::Overflow(minutes_val),
    ))?;
    let hours = parse_hour_bytes(&bytes[..len - 9])?;

    Ok(Self::from_hmsc(hours, minutes, seconds, centis))
  }

  /// Convert this timestamp to a [`Duration`].
  ///
  /// The seconds component saturates at `u64::MAX`, which is unreachable for
  /// any real subtitle file.
  ///
  /// ```rust
  /// use core::time::Duration;
  /// use fasrt::ass::Timestamp;
  ///
  /// let ts = Timestamp::parse("1:02:03.04").unwrap();
  /// assert_eq!(ts.to_duration(), Duration::from_millis(3_723_040));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn to_duration(&self) -> Duration {
    let secs = self
      .hours
      .0
      .saturating_mul(3_600)
      .saturating_add(self.minutes.0 as u64 * 60)
      .saturating_add(self.seconds.0 as u64);
    Duration::new(secs, self.centis.0 as u32 * 10_000_000)
  }

  /// Create a timestamp from a [`Duration`].
  ///
  /// Sub-centisecond precision is truncated toward zero, because the ASS/SSA
  /// wire form carries only two fractional digits.
  ///
  /// ```rust
  /// use core::time::Duration;
  /// use fasrt::ass::Timestamp;
  ///
  /// let ts = Timestamp::from_duration(Duration::from_millis(3_723_049));
  /// assert_eq!(ts.encode().as_str(), "1:02:03.04");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn from_duration(dur: Duration) -> Self {
    let total_secs = dur.as_secs();
    Self {
      hours: Hour::with(total_secs / 3_600),
      minutes: Minute::with(((total_secs % 3_600) / 60) as u8),
      seconds: Second::with((total_secs % 60) as u8),
      centis: Centisecond::with((dur.subsec_millis() / 10) as u8),
    }
  }

  /// Returns the encoded length of this timestamp.
  ///
  /// ```rust
  /// use fasrt::ass::Timestamp;
  ///
  /// assert_eq!(Timestamp::new().encoded_len(), "0:00:00.00".len());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn encoded_len(&self) -> usize {
    // `H+` + `:MM` + `:SS` + `.cc`
    u64_digits(self.hours.0) + 9
  }

  /// Format this timestamp in the ASS/SSA wire form `H:MM:SS.cc`.
  ///
  /// The hour is never zero-padded, which is what VSFilter, libass and
  /// Aegisub all write.
  ///
  /// ```rust
  /// use fasrt::ass::{Hour, Timestamp};
  /// use fasrt::types::{Centisecond, Minute, Second};
  ///
  /// let ts = Timestamp::new();
  /// assert_eq!(ts.encode().as_str(), "0:00:00.00");
  ///
  /// let ts = Timestamp::from_hmsc(
  ///   Hour::with(10),
  ///   Minute::with(2),
  ///   Second::with(3),
  ///   Centisecond::with(4),
  /// );
  /// assert_eq!(ts.encode().as_str(), "10:02:03.04");
  ///
  /// let ts = Timestamp::new().with_hours(Hour::with(u64::MAX));
  /// assert_eq!(ts.encode().as_str(), "18446744073709551615:00:00.00");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn encode(&self) -> Buffer<30> {
    let mut buffer = Buffer::new();
    buffer.fmt_u64(self.hours.0);
    buffer.write_str(":");
    buffer.write_str(self.minutes.as_str());
    buffer.write_str(":");
    buffer.write_str(self.seconds.as_str());
    buffer.write_str(".");
    buffer.write_str(self.centis.as_str());
    buffer
  }
}

/// Reads exactly two ASCII digits, returning `None` if either byte is not a
/// digit.
#[cfg_attr(not(tarpaulin), inline(always))]
fn two_digits(bytes: &[u8]) -> Option<u8> {
  let high = bytes[0].wrapping_sub(b'0');
  let low = bytes[1].wrapping_sub(b'0');
  if high > 9 || low > 9 {
    return None;
  }
  Some(high * 10 + low)
}

/// A section header, e.g. `[Events]`.
///
/// Section names are matched ASCII-case-insensitively.  Anything this crate
/// does not recognize — `[Aegisub Project Garbage]`, for instance — is
/// preserved verbatim as [`Other`](Self::Other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IsVariant, Unwrap, TryUnwrap)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum Section<'a> {
  /// `[Script Info]`
  ScriptInfo,
  /// `[V4 Styles]` — the SSA v4 style section.
  V4Styles,
  /// `[V4+ Styles]` — the ASS v4+ style section.
  V4PlusStyles,
  /// `[Events]`
  Events,
  /// `[Fonts]`
  Fonts,
  /// `[Graphics]`
  Graphics,
  /// Any other section, holding the name as it appeared between the brackets.
  Other(&'a str),
}

impl<'a> Section<'a> {
  /// Recognize a section from the text between the brackets.
  ///
  /// ```rust
  /// use fasrt::ass::Section;
  ///
  /// assert_eq!(Section::new("Events"), Section::Events);
  /// assert_eq!(Section::new("v4+ styles"), Section::V4PlusStyles);
  /// assert_eq!(
  ///   Section::new("Aegisub Project Garbage"),
  ///   Section::Other("Aegisub Project Garbage"),
  /// );
  /// ```
  pub fn new(name: &'a str) -> Self {
    if name.eq_ignore_ascii_case("Script Info") {
      Self::ScriptInfo
    } else if name.eq_ignore_ascii_case("V4 Styles") {
      Self::V4Styles
    } else if name.eq_ignore_ascii_case("V4+ Styles") {
      Self::V4PlusStyles
    } else if name.eq_ignore_ascii_case("Events") {
      Self::Events
    } else if name.eq_ignore_ascii_case("Fonts") {
      Self::Fonts
    } else if name.eq_ignore_ascii_case("Graphics") {
      Self::Graphics
    } else {
      Self::Other(name)
    }
  }

  /// Returns the section name, without the brackets.
  ///
  /// ```rust
  /// use fasrt::ass::Section;
  ///
  /// assert_eq!(Section::Events.as_str(), "Events");
  /// assert_eq!(Section::Other("Fonts+").as_str(), "Fonts+");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'a str {
    match self {
      Self::ScriptInfo => "Script Info",
      Self::V4Styles => "V4 Styles",
      Self::V4PlusStyles => "V4+ Styles",
      Self::Events => "Events",
      Self::Fonts => "Fonts",
      Self::Graphics => "Graphics",
      Self::Other(name) => name,
    }
  }

  /// Whether this section carries an embedded resource: `[Fonts]` and
  /// `[Graphics]`.
  ///
  /// Such a section holds encoded payload lines rather than `Key: Value`
  /// pairs.  Only `fontname:` and `filename:` introduce a resource; every
  /// other line is payload, and the payload alphabet includes `:`, so a
  /// payload line must never be read as a property.
  ///
  /// ```rust
  /// use fasrt::ass::Section;
  ///
  /// assert!(Section::Fonts.is_resource());
  /// assert!(Section::Graphics.is_resource());
  /// assert!(!Section::Events.is_resource());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn is_resource(&self) -> bool {
    matches!(self, Self::Fonts | Self::Graphics)
  }

  /// Whether this section carries `Style:` rows.
  ///
  /// True for the two known style sections, and for any other section whose
  /// name ends with `Styles` — which covers the `[V4++ Styles]` written by
  /// some ASS2 tooling.
  ///
  /// ```rust
  /// use fasrt::ass::Section;
  ///
  /// assert!(Section::V4PlusStyles.is_styles());
  /// assert!(Section::new("V4++ Styles").is_styles());
  /// assert!(!Section::Events.is_styles());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn is_styles(&self) -> bool {
    match self {
      Self::V4Styles | Self::V4PlusStyles => true,
      Self::Other(name) => {
        // Compared as bytes: slicing a `str` at `len - 6` would panic on a
        // name whose tail is not ASCII.
        let bytes = name.trim_end().as_bytes();
        bytes.len() >= 6 && bytes[bytes.len() - 6..].eq_ignore_ascii_case(b"Styles")
      }
      _ => false,
    }
  }
}

impl core::fmt::Display for Section<'_> {
  /// Writes the section header, brackets included.
  ///
  /// ```rust
  /// use fasrt::ass::Section;
  ///
  /// assert_eq!(Section::Events.to_string(), "[Events]");
  /// ```
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "[{}]", self.as_str())
  }
}

/// The kind of an event row: the keyword before the colon.
///
/// These are the six event types defined by the SSA v4.00 specification.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, IsVariant)]
pub enum EventKind {
  /// `Dialogue:` — timed, displayed text.
  #[display("Dialogue")]
  Dialogue,
  /// `Comment:` — a row that is parsed but never displayed.
  #[display("Comment")]
  Comment,
  /// `Picture:` — a timed image.
  #[display("Picture")]
  Picture,
  /// `Sound:` — a timed sound effect.
  #[display("Sound")]
  Sound,
  /// `Movie:` — a timed video clip.
  #[display("Movie")]
  Movie,
  /// `Command:` — a timed command to the host program.
  #[display("Command")]
  Command,
}

impl EventKind {
  /// Recognize an event kind from the keyword before the colon, matched
  /// ASCII-case-insensitively.
  ///
  /// ```rust
  /// use fasrt::ass::EventKind;
  ///
  /// assert_eq!(EventKind::new("Dialogue"), Some(EventKind::Dialogue));
  /// assert_eq!(EventKind::new("comment"), Some(EventKind::Comment));
  /// assert_eq!(EventKind::new("Title"), None);
  /// ```
  pub fn new(keyword: &str) -> Option<Self> {
    if keyword.eq_ignore_ascii_case("Dialogue") {
      Some(Self::Dialogue)
    } else if keyword.eq_ignore_ascii_case("Comment") {
      Some(Self::Comment)
    } else if keyword.eq_ignore_ascii_case("Picture") {
      Some(Self::Picture)
    } else if keyword.eq_ignore_ascii_case("Sound") {
      Some(Self::Sound)
    } else if keyword.eq_ignore_ascii_case("Movie") {
      Some(Self::Movie)
    } else if keyword.eq_ignore_ascii_case("Command") {
      Some(Self::Command)
    } else {
      None
    }
  }

  /// Returns the canonical keyword for this event kind.
  ///
  /// ```rust
  /// use fasrt::ass::EventKind;
  ///
  /// assert_eq!(EventKind::Dialogue.as_str(), "Dialogue");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'static str {
    match self {
      Self::Dialogue => "Dialogue",
      Self::Comment => "Comment",
      Self::Picture => "Picture",
      Self::Sound => "Sound",
      Self::Movie => "Movie",
      Self::Command => "Command",
    }
  }
}

/// A field an event row may carry, as named by an `[Events]` `Format:` line.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, IsVariant)]
pub enum EventField {
  /// `ReadOrder` — the original row order, present only in Matroska
  /// `S_TEXT/ASS` packets.
  #[display("ReadOrder")]
  ReadOrder,
  /// `Marked` — the SSA v4 marked flag, written as `Marked=0`.
  #[display("Marked")]
  Marked,
  /// `Layer` — the ASS v4+ z-order.
  #[display("Layer")]
  Layer,
  /// `Start` — the start timestamp.
  #[display("Start")]
  Start,
  /// `End` — the end timestamp.
  #[display("End")]
  End,
  /// `Style` — the name of the style to render with.
  #[display("Style")]
  Style,
  /// `Name` — the authored speaker name, also spelled `Actor`.
  #[display("Name")]
  Name,
  /// `MarginL` — the left margin override.
  #[display("MarginL")]
  MarginL,
  /// `MarginR` — the right margin override.
  #[display("MarginR")]
  MarginR,
  /// `MarginV` — the vertical margin override.
  #[display("MarginV")]
  MarginV,
  /// `Effect` — the effect name.
  #[display("Effect")]
  Effect,
  /// `Text` — the event text, always the final field.
  #[display("Text")]
  Text,
}

impl EventField {
  /// Every field, in declaration order.
  const ALL: [Self; 12] = [
    Self::ReadOrder,
    Self::Marked,
    Self::Layer,
    Self::Start,
    Self::End,
    Self::Style,
    Self::Name,
    Self::MarginL,
    Self::MarginR,
    Self::MarginV,
    Self::Effect,
    Self::Text,
  ];

  /// Recognize a field from a `Format:` declaration, matched
  /// ASCII-case-insensitively.
  ///
  /// `Actor` is accepted as a spelling of [`Name`](Self::Name): Aegisub labels
  /// the column "Actor" in its UI and some tools write that spelling into the
  /// `Format:` line.
  ///
  /// ```rust
  /// use fasrt::ass::EventField;
  ///
  /// assert_eq!(EventField::new("Layer"), Some(EventField::Layer));
  /// assert_eq!(EventField::new("actor"), Some(EventField::Name));
  /// assert_eq!(EventField::new("Nonsense"), None);
  /// ```
  pub fn new(name: &str) -> Option<Self> {
    if name.eq_ignore_ascii_case("Name") || name.eq_ignore_ascii_case("Actor") {
      return Some(Self::Name);
    }
    Self::ALL
      .into_iter()
      .find(|field| name.eq_ignore_ascii_case(field.as_str()))
  }

  /// Returns the canonical spelling of this field.
  ///
  /// ```rust
  /// use fasrt::ass::EventField;
  ///
  /// assert_eq!(EventField::MarginL.as_str(), "MarginL");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'static str {
    match self {
      Self::ReadOrder => "ReadOrder",
      Self::Marked => "Marked",
      Self::Layer => "Layer",
      Self::Start => "Start",
      Self::End => "End",
      Self::Style => "Style",
      Self::Name => "Name",
      Self::MarginL => "MarginL",
      Self::MarginR => "MarginR",
      Self::MarginV => "MarginV",
      Self::Effect => "Effect",
      Self::Text => "Text",
    }
  }

  /// The field's slot in an [`EventFormat`]'s index table.
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn slot(&self) -> usize {
    match self {
      Self::ReadOrder => 0,
      Self::Marked => 1,
      Self::Layer => 2,
      Self::Start => 3,
      Self::End => 4,
      Self::Style => 5,
      Self::Name => 6,
      Self::MarginL => 7,
      Self::MarginR => 8,
      Self::MarginV => 9,
      Self::Effect => 10,
      Self::Text => 11,
    }
  }
}

/// The declared field order of event rows, as given by an `[Events]`
/// `Format:` line.
///
/// An `EventFormat` stores only field positions, so it is `Copy` and needs no
/// allocation — the whole ASS/SSA parser works on `no_std` without `alloc`.
///
/// The presets cover the three orders that occur in practice: [`ass`],
/// [`ssa`] and [`matroska`].
///
/// [`ass`]: EventFormat::ass
/// [`ssa`]: EventFormat::ssa
/// [`matroska`]: EventFormat::matroska
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventFormat {
  slots: [Option<u8>; 12],
  fields: u8,
}

impl EventFormat {
  /// The largest number of fields a `Format:` line may declare.
  ///
  /// Declarations beyond this are ignored rather than causing an error.
  pub const MAX_FIELDS: usize = u8::MAX as usize;

  /// An empty format, declaring no fields.
  ///
  /// ```rust
  /// use fasrt::ass::EventFormat;
  ///
  /// assert!(EventFormat::empty().is_empty());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn empty() -> Self {
    Self {
      slots: [None; 12],
      fields: 0,
    }
  }

  /// The ASS v4+ order:
  /// `Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text`.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, EventFormat};
  ///
  /// let format = EventFormat::ass();
  /// assert_eq!(format.len(), 10);
  /// assert_eq!(format.index_of(EventField::Layer), Some(0));
  /// assert_eq!(format.index_of(EventField::Text), Some(9));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn ass() -> Self {
    Self::from_order(&[
      EventField::Layer,
      EventField::Start,
      EventField::End,
      EventField::Style,
      EventField::Name,
      EventField::MarginL,
      EventField::MarginR,
      EventField::MarginV,
      EventField::Effect,
      EventField::Text,
    ])
  }

  /// The SSA v4 order:
  /// `Marked, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text`.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, EventFormat};
  ///
  /// let format = EventFormat::ssa();
  /// assert_eq!(format.index_of(EventField::Marked), Some(0));
  /// assert_eq!(format.index_of(EventField::Layer), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn ssa() -> Self {
    Self::from_order(&[
      EventField::Marked,
      EventField::Start,
      EventField::End,
      EventField::Style,
      EventField::Name,
      EventField::MarginL,
      EventField::MarginR,
      EventField::MarginV,
      EventField::Effect,
      EventField::Text,
    ])
  }

  /// The order used by Matroska `S_TEXT/ASS` packets:
  /// `ReadOrder, Layer, Style, Name, MarginL, MarginR, MarginV, Effect, Text`.
  ///
  /// Such a packet carries one event, without the `Dialogue:` keyword and
  /// **without** `Start`/`End` — the container's timestamp and duration are
  /// authoritative.  Pair this preset with [`Event::parse_fields`] to read one
  /// packet at a time.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, EventFormat};
  ///
  /// let format = EventFormat::matroska();
  /// assert_eq!(format.len(), 9);
  /// assert_eq!(format.index_of(EventField::ReadOrder), Some(0));
  /// assert_eq!(format.index_of(EventField::Start), None);
  /// assert_eq!(format.index_of(EventField::End), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn matroska() -> Self {
    Self::from_order(&[
      EventField::ReadOrder,
      EventField::Layer,
      EventField::Style,
      EventField::Name,
      EventField::MarginL,
      EventField::MarginR,
      EventField::MarginV,
      EventField::Effect,
      EventField::Text,
    ])
  }

  /// Builds a format from an explicit field order.
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn from_order(order: &[EventField]) -> Self {
    let mut slots = [None; 12];
    let mut i = 0;
    while i < order.len() {
      slots[order[i].slot()] = Some(i as u8);
      i += 1;
    }
    Self {
      slots,
      fields: order.len() as u8,
    }
  }

  /// Parse the value of an `[Events]` `Format:` line, e.g.
  /// `Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text`.
  ///
  /// Unrecognized names still occupy a position, so the fields after them stay
  /// correctly aligned.  When a field is declared more than once the **last**
  /// occurrence wins, which is how libass resolves it: it walks the
  /// declaration in order and each assignment overwrites the previous one.  At
  /// most [`MAX_FIELDS`](Self::MAX_FIELDS) declarations are honoured.
  ///
  /// An empty or whitespace-only declaration yields [`empty`](Self::empty),
  /// which no event row can be parsed against.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, EventFormat};
  ///
  /// let format = EventFormat::new("Layer, Start, End, Style, Name, Whatever, Text");
  /// assert_eq!(format.len(), 7);
  /// assert_eq!(format.index_of(EventField::Name), Some(4));
  /// assert_eq!(format.index_of(EventField::Text), Some(6));
  /// assert_eq!(format.index_of(EventField::MarginL), None);
  ///
  /// assert!(EventFormat::new("").is_empty());
  /// assert!(EventFormat::new("   ").is_empty());
  /// ```
  pub fn new(declaration: &str) -> Self {
    // `"".split(',')` yields one empty item, which would otherwise be counted
    // as a single unnamed column.
    if declaration.trim().is_empty() {
      return Self::empty();
    }

    let mut slots: [Option<u8>; 12] = [None; 12];
    let mut fields = 0usize;

    for name in declaration.split(',') {
      if fields >= Self::MAX_FIELDS {
        break;
      }
      if let Some(field) = EventField::new(name.trim()) {
        // A repeated field resolves to its *last* position: libass walks the
        // declaration in order and each assignment overwrites the previous.
        slots[field.slot()] = Some(fields as u8);
      }
      fields += 1;
    }

    Self {
      slots,
      fields: fields as u8,
    }
  }

  /// Returns the number of declared fields.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn len(&self) -> usize {
    self.fields as usize
  }

  /// Returns `true` when no fields are declared.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn is_empty(&self) -> bool {
    self.fields == 0
  }

  /// Returns the position of the given field, if it is declared.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, EventFormat};
  ///
  /// assert_eq!(EventFormat::ass().index_of(EventField::Style), Some(3));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn index_of(&self, field: EventField) -> Option<usize> {
    match self.slots[field.slot()] {
      Some(index) => Some(index as usize),
      None => None,
    }
  }

  /// Returns the field declared at the given position, if any.
  ///
  /// Positions holding an unrecognized name yield `None`.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, EventFormat};
  ///
  /// assert_eq!(EventFormat::ass().field_at(0), Some(EventField::Layer));
  /// assert_eq!(EventFormat::ass().field_at(99), None);
  /// ```
  pub const fn field_at(&self, index: usize) -> Option<EventField> {
    if index > u8::MAX as usize {
      return None;
    }
    let mut slot = 0;
    while slot < 12 {
      if let Some(declared) = self.slots[slot] {
        if declared as usize == index {
          return Some(EventField::ALL[slot]);
        }
      }
      slot += 1;
    }
    None
  }
}

impl Default for EventFormat {
  /// The ASS v4+ order.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn default() -> Self {
    Self::ass()
  }
}

/// A single event row: a `Dialogue:`, `Comment:` or other event line.
///
/// Every borrowed field points into the parsed input, so an `Event` never
/// allocates.  Fields are `Option` because an event only carries what its
/// [`EventFormat`] declares — a Matroska packet, for instance, has no
/// `Start`/`End`, and an empty field value is reported as absent.  `Text` is
/// the exception: it is always present, possibly empty.
///
/// A parsed event also keeps the row it came from, so that columns this crate
/// does not recognize survive a write — see [`field`](Self::field).  That row
/// is part of an event's identity: it is observable through
/// [`field`](Self::field) and through what
/// [`Writer`](crate::ass::Writer) emits, so two events whose rows differ are
/// not equal even when every typed field matches.  A row written
/// non-canonically — `0000` where the writer emits `0` — therefore compares
/// unequal to the same event after a round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<'a> {
  kind: EventKind,
  read_order: Option<u32>,
  marked: Option<&'a str>,
  layer: Option<i32>,
  start: Option<Timestamp>,
  end: Option<Timestamp>,
  style: Option<&'a str>,
  name: Option<&'a str>,
  margin_l: Option<u32>,
  margin_r: Option<u32>,
  margin_v: Option<u32>,
  effect: Option<&'a str>,
  text: &'a str,
  /// The row this event was parsed from, empty when it was constructed.
  row: &'a str,
  /// The field count the row was split with, so [`Event::field`] reproduces
  /// the parser's split exactly.
  columns: u8,
}

macro_rules! event_accessors {
  ($(
    $(#[$meta:meta])*
    $field:ident: $ty:ty, $with:ident, $set:ident;
  )*) => {
    $(
      $(#[$meta])*
      #[cfg_attr(not(tarpaulin), inline(always))]
      pub const fn $field(&self) -> Option<$ty> {
        self.$field
      }

      #[doc = concat!("Sets the `", stringify!($field), "` field (builder pattern).")]
      ///
      /// Presence is meaningful, so the value is an `Option`: `None` writes
      /// the field back as empty.
      #[cfg_attr(not(tarpaulin), inline(always))]
      pub const fn $with(mut self, value: Option<$ty>) -> Self {
        self.$field = value;
        self
      }

      #[doc = concat!("Sets the `", stringify!($field), "` field.")]
      #[cfg_attr(not(tarpaulin), inline(always))]
      pub const fn $set(&mut self, value: Option<$ty>) -> &mut Self {
        self.$field = value;
        self
      }
    )*
  };
}

impl<'a> Event<'a> {
  /// Create an event of the given kind carrying the given raw text, with
  /// every optional field absent.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventKind};
  ///
  /// let event = Event::new(EventKind::Dialogue, "Hello");
  /// assert_eq!(event.kind(), EventKind::Dialogue);
  /// assert_eq!(event.text(), "Hello");
  /// assert_eq!(event.name(), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(kind: EventKind, text: &'a str) -> Self {
    Self {
      kind,
      read_order: None,
      marked: None,
      layer: None,
      start: None,
      end: None,
      style: None,
      name: None,
      margin_l: None,
      margin_r: None,
      margin_v: None,
      effect: None,
      text,
      row: "",
      columns: 0,
    }
  }

  /// Returns the raw value of the column at the given position, as it
  /// appeared in the parsed row.
  ///
  /// This is how columns that this crate does not recognize survive: an
  /// `[Events]` `Format:` line may declare vendor or future fields, and
  /// [`Writer::write_event`](crate::ass::Writer::write_event) emits their
  /// original text unchanged rather than dropping them.
  ///
  /// Returns `None` for an event built with [`new`](Self::new) rather than
  /// parsed, and for a position the row does not reach.  Values are returned
  /// untrimmed; the final declared column absorbs the remainder of the row,
  /// exactly as parsing does.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventFormat, EventKind};
  ///
  /// let format = EventFormat::new("Layer, Unrecognized, Text");
  /// let event = Event::parse("Dialogue: 0,keep me,hello", &format).unwrap();
  ///
  /// assert_eq!(event.layer(), Some(0));
  /// assert_eq!(event.field(1), Some("keep me"));
  /// assert_eq!(event.text(), "hello");
  ///
  /// assert_eq!(Event::new(EventKind::Dialogue, "x").field(0), None);
  /// ```
  pub fn field(&self, index: usize) -> Option<&'a str> {
    if self.columns == 0 {
      return None;
    }
    self.row.splitn(self.columns as usize, ',').nth(index)
  }

  /// Returns the event kind.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventKind};
  ///
  /// assert_eq!(Event::new(EventKind::Comment, "").kind(), EventKind::Comment);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn kind(&self) -> EventKind {
    self.kind
  }

  /// Sets the event kind (builder pattern).
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_kind(mut self, kind: EventKind) -> Self {
    self.kind = kind;
    self
  }

  /// Sets the event kind.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_kind(&mut self, kind: EventKind) -> &mut Self {
    self.kind = kind;
    self
  }

  /// Returns the raw `Text` field, exactly as it appeared.
  ///
  /// This still contains override blocks and escapes; wrap it in
  /// [`PlainText`](crate::ass::text::PlainText) for the cleaned form.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventKind};
  ///
  /// let event = Event::new(EventKind::Dialogue, "{\\i1}Hi");
  /// assert_eq!(event.text(), "{\\i1}Hi");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn text(&self) -> &'a str {
    self.text
  }

  /// Sets the raw `Text` field (builder pattern).
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_text(mut self, text: &'a str) -> Self {
    self.text = text;
    self
  }

  /// Sets the raw `Text` field.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_text(&mut self, text: &'a str) -> &mut Self {
    self.text = text;
    self
  }

  /// Returns the cleaned text of this event.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::ass::{Event, EventKind};
  ///
  /// let event = Event::new(EventKind::Dialogue, "{\\i1}Hello{\\i0}\\NWorld");
  /// assert_eq!(event.plain_text().normalize(), "Hello\nWorld");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn plain_text(&self) -> super::text::PlainText<'a> {
    super::text::PlainText::new(self.text)
  }

  event_accessors! {
    /// Returns the `ReadOrder` field — the original row index, carried by
    /// Matroska `S_TEXT/ASS` packets so that overlapping events can be
    /// restored to their authored order.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_read_order(Some(7));
    /// assert_eq!(event.read_order(), Some(7));
    /// ```
    read_order: u32, with_read_order, set_read_order;

    /// Returns the SSA v4 `Marked` field, verbatim (typically `Marked=0`).
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_marked(Some("Marked=0"));
    /// assert_eq!(event.marked(), Some("Marked=0"));
    /// ```
    marked: &'a str, with_marked, set_marked;

    /// Returns the ASS v4+ `Layer` field — the z-order of this event.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_layer(Some(2));
    /// assert_eq!(event.layer(), Some(2));
    /// ```
    layer: i32, with_layer, set_layer;

    /// Returns the `Start` timestamp.
    ///
    /// This is `None` for an event that came from a Matroska packet, where
    /// the container timestamp is authoritative.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind, Timestamp};
    ///
    /// let start = Timestamp::parse("0:00:01.00").unwrap();
    /// let event = Event::new(EventKind::Dialogue, "").with_start(Some(start));
    /// assert_eq!(event.start(), Some(start));
    /// ```
    start: Timestamp, with_start, set_start;

    /// Returns the `End` timestamp.
    ///
    /// This is `None` for an event that came from a Matroska packet, where
    /// the container duration is authoritative.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind, Timestamp};
    ///
    /// let end = Timestamp::parse("0:00:03.00").unwrap();
    /// let event = Event::new(EventKind::Dialogue, "").with_end(Some(end));
    /// assert_eq!(event.end(), Some(end));
    /// ```
    end: Timestamp, with_end, set_end;

    /// Returns the `Style` field — the name of the style to render with.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_style(Some("Default"));
    /// assert_eq!(event.style(), Some("Default"));
    /// ```
    style: &'a str, with_style, set_style;

    /// Returns the `Name` field — the speaker name authored into the script.
    ///
    /// ASS/SSA carries this natively, which makes it person-name observation
    /// material that no other subtitle format in this crate provides.  It is
    /// `None` when the column is absent or empty.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventFormat, EventKind};
    ///
    /// let line = "Dialogue: 0,0:00:01.00,0:00:03.00,Default,Rin,0,0,0,,Hello";
    /// let event = Event::parse(line, &EventFormat::ass()).unwrap();
    /// assert_eq!(event.name(), Some("Rin"));
    /// ```
    name: &'a str, with_name, set_name;

    /// Returns the `MarginL` field.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_margin_l(Some(10));
    /// assert_eq!(event.margin_l(), Some(10));
    /// ```
    margin_l: u32, with_margin_l, set_margin_l;

    /// Returns the `MarginR` field.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_margin_r(Some(20));
    /// assert_eq!(event.margin_r(), Some(20));
    /// ```
    margin_r: u32, with_margin_r, set_margin_r;

    /// Returns the `MarginV` field.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_margin_v(Some(30));
    /// assert_eq!(event.margin_v(), Some(30));
    /// ```
    margin_v: u32, with_margin_v, set_margin_v;

    /// Returns the `Effect` field.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind};
    ///
    /// let event = Event::new(EventKind::Dialogue, "").with_effect(Some("Karaoke"));
    /// assert_eq!(event.effect(), Some("Karaoke"));
    /// ```
    effect: &'a str, with_effect, set_effect;
  }

  /// Parse a complete event line, including the `Dialogue:` keyword.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventFormat, EventKind, Timestamp};
  ///
  /// let line = "Dialogue: 0,0:00:01.00,0:00:03.00,Default,Rin,0,0,0,,Hello there";
  /// let event = Event::parse(line, &EventFormat::ass()).unwrap();
  ///
  /// assert_eq!(event.kind(), EventKind::Dialogue);
  /// assert_eq!(event.layer(), Some(0));
  /// assert_eq!(event.start(), Some(Timestamp::parse("0:00:01.00").unwrap()));
  /// assert_eq!(event.style(), Some("Default"));
  /// assert_eq!(event.name(), Some("Rin"));
  /// assert_eq!(event.effect(), None);
  /// assert_eq!(event.text(), "Hello there");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn parse(line: &'a str, format: &EventFormat) -> Result<Self, ParseAssError> {
    Self::parse_with(line, format, &Options::strict())
  }

  /// Parse a complete event line under the given [`Options`].
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventFormat, Options};
  ///
  /// // A row that stops early is an error in strict mode…
  /// let short = "Dialogue: 0,0:00:01.00,0:00:03.00";
  /// assert!(Event::parse(short, &EventFormat::ass()).is_err());
  ///
  /// // …and tolerated in lossy mode, where the missing fields are absent.
  /// let event = Event::parse_with(short, &EventFormat::ass(), &Options::lossy()).unwrap();
  /// assert_eq!(event.style(), None);
  /// assert_eq!(event.text(), "");
  /// ```
  pub fn parse_with(
    line: &'a str,
    format: &EventFormat,
    options: &Options,
  ) -> Result<Self, ParseAssError> {
    let (keyword, rest) = line.split_once(':').ok_or(ParseAssError::UnexpectedLine)?;
    let kind = EventKind::new(keyword.trim()).ok_or(ParseAssError::UnexpectedLine)?;
    Self::parse_fields_with(kind, rest.trim_start_matches([' ', '\t']), format, options)
  }

  /// Parse an event's comma-separated field list, without a leading keyword.
  ///
  /// This is the form carried by a Matroska `S_TEXT/ASS` packet, one event per
  /// packet.  Pair it with [`EventFormat::matroska`]; the container's
  /// timestamp and duration supply the timing that the packet omits.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventFormat, EventKind};
  ///
  /// // Exactly the bytes a Matroska ASS packet carries.
  /// let packet = "12,0,Default,Rin,0,0,0,,{\\i1}Hello{\\i0}";
  /// let event = Event::parse_fields(EventKind::Dialogue, packet, &EventFormat::matroska())
  ///   .unwrap();
  ///
  /// assert_eq!(event.read_order(), Some(12));
  /// assert_eq!(event.name(), Some("Rin"));
  /// assert_eq!(event.start(), None);
  /// assert_eq!(event.text(), "{\\i1}Hello{\\i0}");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn parse_fields(
    kind: EventKind,
    fields: &'a str,
    format: &EventFormat,
  ) -> Result<Self, ParseAssError> {
    Self::parse_fields_with(kind, fields, format, &Options::strict())
  }

  /// Parse an event's comma-separated field list under the given [`Options`].
  ///
  /// The final declared field absorbs the remainder of the input, so a `Text`
  /// field containing commas — which is common — is preserved intact.  ASS/SSA
  /// requires `Text` to be the last declared field for exactly this reason.
  ///
  /// ```rust
  /// use fasrt::ass::{Event, EventFormat, EventKind};
  ///
  /// let fields = "0,0:00:01.00,0:00:03.00,Default,,0,0,0,,One, two, three";
  /// let event = Event::parse_fields(EventKind::Dialogue, fields, &EventFormat::ass())
  ///   .unwrap();
  /// assert_eq!(event.text(), "One, two, three");
  /// ```
  pub fn parse_fields_with(
    kind: EventKind,
    fields: &'a str,
    format: &EventFormat,
    options: &Options,
  ) -> Result<Self, ParseAssError> {
    let declared = format.len();
    if declared == 0 {
      return Err(ParseAssError::MissingFormat);
    }

    let mut event = Self::new(kind, "");
    event.row = fields;
    event.columns = declared as u8;
    let mut found = 0usize;

    for (index, value) in fields.splitn(declared, ',').enumerate() {
      found = index + 1;
      let Some(field) = format.field_at(index) else {
        continue;
      };

      // `Text` keeps its bytes exactly; every other field is trimmed, and an
      // empty value means the field is absent.
      if matches!(field, EventField::Text) {
        event.text = value;
        continue;
      }

      let value = value.trim();
      if value.is_empty() {
        continue;
      }

      match field {
        EventField::ReadOrder => {
          event.read_order = parse_number(value, field, options)?;
        }
        EventField::Marked => event.marked = Some(value),
        EventField::Layer => {
          event.layer = parse_number(value, field, options)?;
        }
        EventField::Start => {
          event.start = parse_timestamp(value, options)?;
        }
        EventField::End => {
          event.end = parse_timestamp(value, options)?;
        }
        EventField::Style => event.style = Some(value),
        EventField::Name => event.name = Some(value),
        EventField::MarginL => {
          event.margin_l = parse_number(value, field, options)?;
        }
        EventField::MarginR => {
          event.margin_r = parse_number(value, field, options)?;
        }
        EventField::MarginV => {
          event.margin_v = parse_number(value, field, options)?;
        }
        EventField::Effect => event.effect = Some(value),
        EventField::Text => unreachable!("handled above"),
      }
    }

    if found < declared && !options.allow_short_event() {
      return Err(ParseAssError::TooFewFields {
        expected: declared,
        found,
      });
    }

    Ok(event)
  }
}

/// Parses a numeric event field, honouring [`Options::allow_malformed_fields`].
fn parse_number<T: FromStr>(
  value: &str,
  field: EventField,
  options: &Options,
) -> Result<Option<T>, ParseAssError> {
  match value.parse::<T>() {
    Ok(parsed) => Ok(Some(parsed)),
    Err(_) if options.allow_malformed_fields() => Ok(None),
    Err(_) => Err(ParseAssError::InvalidField(field)),
  }
}

/// Parses a timestamp event field, honouring [`Options::allow_malformed_fields`].
///
/// The error from [`Timestamp::parse`] names the component at fault, so it is
/// propagated rather than flattened into [`ParseAssError::InvalidField`].
fn parse_timestamp(value: &str, options: &Options) -> Result<Option<Timestamp>, ParseAssError> {
  match Timestamp::parse(value) {
    Ok(parsed) => Ok(Some(parsed)),
    Err(_) if options.allow_malformed_fields() => Ok(None),
    Err(err) => Err(err),
  }
}

/// A `Key: Value` line, such as a `[Script Info]` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Property<'a> {
  key: &'a str,
  value: &'a str,
}

impl<'a> Property<'a> {
  /// Create a property from its key and value.
  ///
  /// ```rust
  /// use fasrt::ass::Property;
  ///
  /// let property = Property::new("Title", "Example");
  /// assert_eq!(property.key(), "Title");
  /// assert_eq!(property.value(), "Example");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(key: &'a str, value: &'a str) -> Self {
    Self { key, value }
  }

  /// Returns the key, without the trailing colon.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn key(&self) -> &'a str {
    self.key
  }

  /// Returns the value, with the whitespace after the colon removed.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn value(&self) -> &'a str {
    self.value
  }
}

impl core::fmt::Display for Property<'_> {
  /// Writes the property back in its canonical `Key: Value` form.
  ///
  /// ```rust
  /// use fasrt::ass::Property;
  ///
  /// assert_eq!(Property::new("Title", "Example").to_string(), "Title: Example");
  /// ```
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}: {}", self.key, self.value)
  }
}

/// A `Format:` declaration line.
///
/// The raw declaration is kept so a round-trip writes back exactly what was
/// read; [`event_format`](Self::event_format) resolves it to field positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Format<'a> {
  raw: &'a str,
}

impl<'a> Format<'a> {
  /// Create a `Format` from the raw value of a `Format:` line.
  ///
  /// ```rust
  /// use fasrt::ass::Format;
  ///
  /// let format = Format::new("Layer, Start, End, Style, Name, Text");
  /// assert_eq!(format.as_str(), "Layer, Start, End, Style, Name, Text");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(raw: &'a str) -> Self {
    Self { raw }
  }

  /// Returns the raw declaration, exactly as it appeared.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'a str {
    self.raw
  }

  /// Returns a lazy iterator over the declared field names, each trimmed.
  ///
  /// ```rust
  /// use fasrt::ass::Format;
  ///
  /// let format = Format::new("Layer, Start, End");
  /// let names: Vec<_> = format.fields().collect();
  /// assert_eq!(names, ["Layer", "Start", "End"]);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn fields(&self) -> Fields<'a> {
    Fields {
      inner: self.raw.split(','),
    }
  }

  /// Resolves this declaration to event field positions.
  ///
  /// ```rust
  /// use fasrt::ass::{EventField, Format};
  ///
  /// let format = Format::new("Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text");
  /// assert_eq!(format.event_format(), fasrt::ass::EventFormat::ass());
  /// assert_eq!(format.event_format().index_of(EventField::Text), Some(9));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn event_format(&self) -> EventFormat {
    EventFormat::new(self.raw)
  }
}

/// A `Style:` row from a style section.
///
/// Style *rendering* semantics are out of scope for this crate, so the row is
/// exposed as its raw comma-separated fields.  Use the section's `Format:`
/// line to learn what each position means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StyleRow<'a> {
  raw: &'a str,
}

impl<'a> StyleRow<'a> {
  /// Create a style row from the raw value of a `Style:` line.
  ///
  /// ```rust
  /// use fasrt::ass::StyleRow;
  ///
  /// let row = StyleRow::new("Default,Arial,20");
  /// assert_eq!(row.as_str(), "Default,Arial,20");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(raw: &'a str) -> Self {
    Self { raw }
  }

  /// Returns the raw row, exactly as it appeared.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'a str {
    self.raw
  }

  /// Returns a lazy iterator over the row's fields, each trimmed.
  ///
  /// ```rust
  /// use fasrt::ass::StyleRow;
  ///
  /// let row = StyleRow::new("Default, Arial, 20");
  /// let fields: Vec<_> = row.fields().collect();
  /// assert_eq!(fields, ["Default", "Arial", "20"]);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn fields(&self) -> Fields<'a> {
    Fields {
      inner: self.raw.split(','),
    }
  }

  /// Returns the field at the given position, if the row has one.
  ///
  /// ```rust
  /// use fasrt::ass::StyleRow;
  ///
  /// let row = StyleRow::new("Default,Arial,20");
  /// assert_eq!(row.field(0), Some("Default"));
  /// assert_eq!(row.field(3), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn field(&self, index: usize) -> Option<&'a str> {
    self.fields().nth(index)
  }
}

/// A lazy iterator over trimmed, comma-separated fields.
///
/// Created by [`Format::fields`] and [`StyleRow::fields`].
#[derive(Debug, Clone)]
pub struct Fields<'a> {
  inner: core::str::Split<'a, char>,
}

impl<'a> Iterator for Fields<'a> {
  type Item = &'a str;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next().map(str::trim)
  }
}

impl DoubleEndedIterator for Fields<'_> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn next_back(&mut self) -> Option<Self::Item> {
    self.inner.next_back().map(str::trim)
  }
}

/// A single parsed line of an ASS/SSA script.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum Block<'a> {
  /// A section header, e.g. `[Events]`.
  Section(Section<'a>),
  /// A comment line, holding everything after the leading `;`.
  Comment(&'a str),
  /// A `Format:` declaration.
  Format(Format<'a>),
  /// A `Style:` row from a style section.
  Style(StyleRow<'a>),
  /// An event row from the `[Events]` section.
  Event(Event<'a>),
  /// A payload line of an embedded resource, held verbatim.
  ///
  /// Produced only inside a section for which [`Section::is_resource`] is
  /// true, where lines are encoded font or image data rather than properties.
  Data(&'a str),
  /// Any other `Key: Value` line.
  Property(Property<'a>),
}
