use core::num::NonZeroU64;

use logos::{Lexer, Logos};

use crate::error::*;
use crate::types::{Millisecond, Minute, Second};

pub use types::{Entry, Header, Hour, Timestamp};

mod types;

/// Cue-body text parsing and clean-text extraction.
pub mod text;

/// The error type for parsing SRT files.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseSrtError {
  /// An error occurred while parsing the hour component of a timestamp.
  #[error(transparent)]
  ParseMinute(#[from] ParseMinuteError),
  /// An error occurred while parsing the minute component of a timestamp.
  #[error(transparent)]
  ParseSecond(#[from] ParseSecondError),
  /// An error occurred while parsing the second component of a timestamp.
  #[error(transparent)]
  ParseHour(#[from] ParseHourError),
  /// An error occurred while parsing the millisecond component of a timestamp.
  #[error(transparent)]
  ParseMillisecond(#[from] ParseMillisecondError),
  /// An error occurred while parsing the index number of a subtitle.
  #[error(transparent)]
  ParseIndex(#[from] ParseIndexNumberError),

  /// Expected an end timestamp after the arrow token "-->".
  #[error("unclosed duration, missing end timestamp")]
  UnclosedDuration,

  /// Unopened duration, missing start timestamp before the arrow token "-->".
  #[error("unopened duration, missing start timestamp")]
  UnopenedDuration,

  /// Expected a header (timeline) line after the index.
  #[error("expected header line (e.g. '00:00:01,000 --> 00:00:04,000') after index {0}")]
  ExpectedHeader(NonZeroU64),

  /// Index numbers must be monotonically increasing.
  #[error("non-monotonic index: expected > {last}, got {got}")]
  NonMonotonicIndex {
    /// The previous index number.
    last: u64,
    /// The current (bad) index number.
    got: u64,
  },

  /// An unknown lexer error occurred.
  #[error("unexpected token: {0}")]
  Unknown(&'static str),
}

impl Default for ParseSrtError {
  fn default() -> Self {
    Self::Unknown("unknown lexer error")
  }
}

/// Options that control how the SRT parser handles malformed input.
///
/// By default, the parser runs in **strict** mode: monotonic index
/// enforcement is on, and any malformed input causes an error.
/// Use [`Options::lossy`] for a maximally permissive preset.
///
/// With the `serde` feature, this type implements [`serde::Serialize`] and
/// [`serde::Deserialize`] as a snake_case document of its four flags; a field
/// missing on the way in takes its value from [`Options::default`] (strict),
/// so a caller may declare only the flags they want to override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case", default))]
pub struct Options {
  /// Accept entries that have a header line but no preceding index number.
  allow_missing_index: bool,
  /// Silently skip text lines that appear outside of any subtitle entry.
  ignore_orphan_text: bool,
  /// When an index is followed by an invalid header line, skip the broken
  /// block instead of returning an error.
  ignore_broken_header: bool,
  /// Enforce that index numbers are monotonically increasing.
  /// In strict (default) mode this causes an error; in lossy mode
  /// non-monotonic entries are silently skipped.
  monotonic_index: bool,
}

impl Options {
  /// Strict preset — the default. Monotonic index enforcement is on,
  /// and all malformed input causes an error.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn strict() -> Self {
    Self {
      allow_missing_index: false,
      ignore_orphan_text: false,
      ignore_broken_header: false,
      monotonic_index: true,
    }
  }

  /// Lossy preset — maximally permissive. Missing indices, orphan text,
  /// and broken headers are tolerated; monotonic enforcement is off.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn lossy() -> Self {
    Self {
      allow_missing_index: true,
      ignore_orphan_text: true,
      ignore_broken_header: true,
      monotonic_index: false,
    }
  }

  /// Returns whether missing index numbers are allowed.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict();
  /// assert!(!opts.allow_missing_index());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn allow_missing_index(&self) -> bool {
    self.allow_missing_index
  }

  /// Sets whether missing index numbers are allowed.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict().with_allow_missing_index(true);
  /// assert!(opts.allow_missing_index());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_allow_missing_index(mut self, value: bool) -> Self {
    self.set_allow_missing_index(value);
    self
  }

  /// Sets whether missing index numbers are allowed.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let mut opts = Options::strict();
  /// opts.set_allow_missing_index(true);
  /// assert!(opts.allow_missing_index());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_allow_missing_index(&mut self, value: bool) -> &mut Self {
    self.allow_missing_index = value;
    self
  }

  /// Returns whether orphan text lines are silently skipped.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict();
  /// assert!(!opts.ignore_orphan_text());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn ignore_orphan_text(&self) -> bool {
    self.ignore_orphan_text
  }

  /// Sets whether orphan text lines are silently skipped.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict().with_ignore_orphan_text(true);
  /// assert!(opts.ignore_orphan_text());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_ignore_orphan_text(mut self, value: bool) -> Self {
    self.set_ignore_orphan_text(value);
    self
  }

  /// Sets whether orphan text lines are silently skipped.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let mut opts = Options::strict();
  /// opts.set_ignore_orphan_text(true);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_ignore_orphan_text(&mut self, value: bool) -> &mut Self {
    self.ignore_orphan_text = value;
    self
  }

  /// Returns whether broken headers are silently skipped.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict();
  /// assert!(!opts.ignore_broken_header());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn ignore_broken_header(&self) -> bool {
    self.ignore_broken_header
  }

  /// Sets whether broken headers are silently skipped.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict().with_ignore_broken_header(true);
  /// assert!(opts.ignore_broken_header());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_ignore_broken_header(mut self, value: bool) -> Self {
    self.set_ignore_broken_header(value);
    self
  }

  /// Sets whether broken headers are silently skipped.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let mut opts = Options::strict();
  /// opts.set_ignore_broken_header(true);
  /// assert!(opts.ignore_broken_header());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_ignore_broken_header(&mut self, value: bool) -> &mut Self {
    self.ignore_broken_header = value;
    self
  }

  /// Returns whether monotonic index enforcement is on.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict();
  /// assert!(opts.monotonic_index());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn monotonic_index(&self) -> bool {
    self.monotonic_index
  }

  /// Sets whether monotonic index enforcement is on.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let opts = Options::strict().with_monotonic_index(false);
  /// assert!(!opts.monotonic_index());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_monotonic_index(mut self, value: bool) -> Self {
    self.set_monotonic_index(value);
    self
  }

  /// Sets whether monotonic index enforcement is on.
  ///
  /// ```rust
  /// use fasrt::srt::Options;
  ///
  /// let mut opts = Options::strict();
  /// opts.set_monotonic_index(false);
  /// assert!(!opts.monotonic_index());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_monotonic_index(&mut self, value: bool) -> &mut Self {
    self.monotonic_index = value;
    self
  }

  /// Returns `true` if any tolerance option is enabled (i.e. not fully strict).
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn is_tolerant(&self) -> bool {
    self.allow_missing_index || self.ignore_orphan_text || self.ignore_broken_header
  }
}

impl Default for Options {
  fn default() -> Self {
    Self::strict()
  }
}

/// Token produced by the lexer.
#[derive(Debug, Logos, PartialEq)]
#[logos(
  error = ParseSrtError,
  extras = Option<Self>,
)]
enum Token {
  /// Header "HH:MM:SS,mmm --> HH:MM:SS,mmm"
  #[regex(
    r"[0-9]{2,3}:[0-5][0-9]:[0-5][0-9],[0-9]{3}[ \t\x0C]+-->[ \t\x0C]+[0-9]{2,3}:[0-5][0-9]:[0-5][0-9],[0-9]{3}",
    parse_header
  )]
  Header(Header),

  /// A number (subtitle index).
  #[regex(r"[0-9]+", parse_number, priority = 3)]
  Number(NonZeroU64),
}

#[inline]
fn parse_number(s: &mut Lexer<'_, Token>) -> Result<NonZeroU64, ParseSrtError> {
  let slice = s.slice().trim();
  if slice.len() > 20 {
    return Err(ParseIndexNumberError::Overflow.into());
  }

  if slice == "0" {
    return Err(ParseIndexNumberError::Zero.into());
  }

  slice
    .parse::<u64>()
    .map_err(|e| ParseIndexNumberError::ParseInt(e).into())
    .and_then(|num| NonZeroU64::new(num).ok_or(ParseIndexNumberError::Zero.into()))
}

#[inline]
fn parse_header(s: &mut Lexer<'_, Token>) -> Result<Header, ParseSrtError> {
  let slice = s.slice();
  // The regex guarantees `HH+:MM:SS,mmm <ws> --> <ws> HH+:MM:SS,mmm`.
  // Find the arrow separator and extract both timestamps via byte arithmetic.
  let arrow = slice.find("-->").unwrap();
  let start_str = slice[..arrow].trim();
  let end_str = slice[arrow + 3..].trim();
  let start = parse_timestamp_bytes(start_str.as_bytes())?;
  let end = parse_timestamp_bytes(end_str.as_bytes())?;
  Ok(Header::new(start, end))
}

/// Parse an SRT timestamp from raw bytes using direct digit extraction.
///
/// Format: `H+:MM:SS,mmm` where H is 2-3 digits (regex-validated),
/// MM and SS are exactly 2 digits, mmm is exactly 3 digits.
/// Layout from the end: `mmm`(3) `,`(1) `SS`(2) `:`(1) `MM`(2) `:`(1) = 10 fixed bytes.
#[inline]
fn parse_timestamp_bytes(b: &[u8]) -> Result<Timestamp, ParseSrtError> {
  let len = b.len();
  let millis = Millisecond(digit3(&b[len - 3..]));
  let seconds = Second(digit2(&b[len - 6..len - 4]));
  let minutes = Minute(digit2(&b[len - 9..len - 7]));
  let hour_len = len - 10;
  let hours = match hour_len {
    2 => Hour(digit2(&b[..2]) as u16),
    3 => Hour(digit3(&b[..3])),
    _ => return Err(ParseHourError::NotPadded.into()),
  };
  Ok(Timestamp::from_hmsm(hours, minutes, seconds, millis))
}

#[cfg_attr(not(tarpaulin), inline(always))]
const fn digit2(b: &[u8]) -> u8 {
  (b[0] - b'0') * 10 + (b[1] - b'0')
}

#[cfg_attr(not(tarpaulin), inline(always))]
const fn digit3(b: &[u8]) -> u16 {
  (b[0] - b'0') as u16 * 100 + (b[1] - b'0') as u16 * 10 + (b[2] - b'0') as u16
}

struct StateBody {
  header: Header,
  start: usize,
  end: usize,
}

impl StateBody {
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn new(header: Header, start: usize, end: usize) -> Self {
    Self { header, start, end }
  }
}

/// State machine states for the SRT parser.
enum State {
  /// Expecting a subtitle index number (or blank lines / BOM to skip).
  Index,
  /// Got an index, expecting the header (timeline) line.
  Header(NonZeroU64),
  /// Got the header, collecting body text lines.
  Body(StateBody),
  /// Lossy recovery: skip lines until the next blank line, then go to `Index`.
  SkipToBlank,
  /// The iterator has encountered an error or is exhausted.
  Done,
}

/// A lazy, zero-copy SRT parser that yields subtitle entries as an iterator.
///
/// Created via [`Parser::strict`], [`Parser::lossy`], or
/// [`Parser::with_options`]. Each call to [`Iterator::next`] yields the
/// next parsed [`Entry`], or an error if the input is malformed. Once
/// an error is returned the iterator is exhausted.
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
/// use fasrt::srt::Parser;
///
/// let srt = "\
/// 1
/// 00:00:01,000 --> 00:00:04,000
/// Hello world!
///
/// 2
/// 00:00:05,000 --> 00:00:08,000
/// Goodbye world!
/// ";
///
/// let subs: Vec<_> = Parser::strict(srt).collect::<Result<_, _>>().unwrap();
/// assert_eq!(subs.len(), 2);
/// assert_eq!(*subs[0].body_ref(), "Hello world!");
/// assert_eq!(*subs[1].body_ref(), "Goodbye world!");
/// # }
/// ```
pub struct Parser<'a> {
  input: &'a str,
  lines: Lines<'a>,
  state: State,
  opts: Options,
  /// Tracks the last yielded index for monotonic-index enforcement.
  last_index: u64,
}

impl<'a> Parser<'a> {
  /// Create a parser in **strict** mode.
  ///
  /// Any malformed input — including non-monotonic indices — causes the
  /// iterator to yield an error and stop.
  pub fn strict(input: &'a str) -> Self {
    Self::with_options(input, Options::strict())
  }

  /// Create a parser in **lossy** mode with all tolerances enabled.
  ///
  /// Silently skips malformed entries instead of returning errors.
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
  /// use fasrt::srt::Parser;
  ///
  /// let srt = "\
  /// 1
  /// 00:00:01,000 --> 00:00:04,000
  /// Good entry
  ///
  /// garbage line
  ///
  /// 00:00:05,000 --> 00:00:08,000
  /// Missing index entry
  /// ";
  ///
  /// let subs: Vec<_> = Parser::lossy(srt).collect::<Result<_, _>>().unwrap();
  /// assert_eq!(subs.len(), 2);
  /// assert_eq!(*subs[0].body_ref(), "Good entry");
  /// assert_eq!(*subs[1].body_ref(), "Missing index entry");
  /// # }
  /// ```
  pub fn lossy(input: &'a str) -> Self {
    Self::with_options(input, Options::lossy())
  }

  /// Create a parser with the given [`Options`].
  pub fn with_options(input: &'a str, opts: Options) -> Self {
    Self {
      input,
      lines: Lines::new(input),
      state: State::Index,
      opts,
      last_index: 0,
    }
  }
}

impl<'a> Iterator for Parser<'a> {
  type Item = Result<Entry<&'a str>, ParseSrtError>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.state {
        State::Done => return None,
        State::SkipToBlank => {
          let Some(line) = self.lines.next() else {
            self.state = State::Done;
            return None;
          };
          if line.trim_start_matches('\u{feff}').is_empty() {
            self.state = State::Index;
          }
        }
        State::Index => {
          let Some(line) = self.lines.next() else {
            self.state = State::Done;
            return None;
          };

          let trimmed = line.trim_start_matches('\u{feff}');
          if trimmed.is_empty() {
            continue;
          }

          match lex(trimmed) {
            // 1. Index number (normal SRT flow).
            Ok(Some(Token::Number(index))) => {
              if self.opts.monotonic_index && index.get() <= self.last_index {
                if self.opts.is_tolerant() {
                  self.state = State::SkipToBlank;
                  continue;
                }
                self.state = State::Done;
                return Some(Err(ParseSrtError::NonMonotonicIndex {
                  last: self.last_index,
                  got: index.get(),
                }));
              }
              self.state = State::Header(index);
            }
            // 2. Header line without index (missing index).
            Ok(Some(Token::Header(header))) if self.opts.allow_missing_index => {
              let offset = line.as_ptr() as usize - self.input.as_ptr() as usize + line.len();
              self.state = State::Body(StateBody::new(header, offset, offset));
            }
            // 3. Orphan text / no token / unrecognised line.
            Ok(_) | Err(_) if self.opts.ignore_orphan_text => {
              continue;
            }
            Ok(Some(_)) | Ok(None) => {
              self.state = State::Done;
              return Some(Err(ParseSrtError::Unknown(
                "expected subtitle index number",
              )));
            }
            Err(e) => {
              self.state = State::Done;
              return Some(Err(e));
            }
          }
        }
        State::Header(index) => {
          let Some(line) = self.lines.next() else {
            self.state = State::Done;
            return if self.opts.ignore_broken_header {
              None
            } else {
              Some(Err(ParseSrtError::ExpectedHeader(index)))
            };
          };

          let trimmed = line.trim_start_matches('\u{feff}');
          match lex(trimmed) {
            Ok(Some(Token::Header(mut header))) => {
              header.set_index(index);
              let offset = line.as_ptr() as usize - self.input.as_ptr() as usize + line.len();
              self.state = State::Body(StateBody::new(header, offset, offset));
            }
            _ if self.opts.ignore_broken_header => {
              if trimmed.is_empty() {
                self.state = State::Index;
              } else {
                self.state = State::SkipToBlank;
              }
            }
            Ok(_) => {
              self.state = State::Done;
              return Some(Err(ParseSrtError::ExpectedHeader(index)));
            }
            Err(e) => {
              self.state = State::Done;
              return Some(Err(e));
            }
          }
        }
        State::Body(ref mut body) => {
          let StateBody { header, start, end } = body;
          let Some(line) = self.lines.next() else {
            let body = body_slice(self.input, *start, *end);
            let entry = Entry::new(header.clone(), body);
            if let Some(idx) = header.index() {
              self.last_index = idx.get();
            }
            self.state = State::Done;
            return Some(Ok(entry));
          };

          let trimmed = line.trim_start_matches('\u{feff}');
          if trimmed.is_empty() {
            let body = body_slice(self.input, *start, *end);
            let entry = Entry::new(header.clone(), body);
            if let Some(idx) = header.index() {
              self.last_index = idx.get();
            }
            self.state = State::Index;
            return Some(Ok(entry));
          }

          // Accumulate text line
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

/// Extract the body slice from the input, or empty string if no text lines.
#[cfg_attr(not(tarpaulin), inline(always))]
fn body_slice(input: &str, start: usize, end: usize) -> &str {
  if start >= end { "" } else { &input[start..end] }
}

/// Lex the next token from a line. Returns `Ok(None)` when the line
/// produces no tokens.
fn lex(line: &str) -> Result<Option<Token>, ParseSrtError> {
  match Token::lexer(line).next() {
    Some(result) => result.map(Some),
    None => Ok(None),
  }
}

/// An SRT file writer that writes subtitle entries to a [`std::io::Write`] target.
///
/// # Example
///
/// ```
/// use fasrt::srt::{Writer, Entry, Header, Hour, Timestamp};
/// use fasrt::types::*;
///
/// let mut buf = Vec::new();
/// let mut writer = Writer::new(&mut buf);
///
/// let header = Header::new(
///   Timestamp::from_hmsm(Hour::with(0), Minute::with(0), Second::with(1), Millisecond::with(0)),
///   Timestamp::from_hmsm(Hour::with(0), Minute::with(0), Second::with(4), Millisecond::with(0)),
/// ).with_index(std::num::NonZeroU64::new(1).unwrap());
///
/// writer.write(&Entry::new(header, "Hello world!")).unwrap();
///
/// let output = String::from_utf8(buf).unwrap();
/// assert_eq!(output, "1\n00:00:01,000 --> 00:00:04,000\nHello world!\n");
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct Writer<W> {
  inner: W,
  has_written: bool,
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
        has_written: false,
      }
    }

    /// Write a single subtitle entry.
    ///
    /// A blank line separator is emitted **between** entries, not after the
    /// last one, matching canonical SRT format.
    pub fn write<T: AsRef<str>>(&mut self, entry: &Entry<T>) -> io::Result<()> {
      if self.has_written {
        self.inner.write_all(b"\n")?;
      }
      self.has_written = true;
      let header = entry.header_ref();
      self.inner.write_all(header.encode().as_str().as_bytes())?;
      let body = entry.body_ref().as_ref();
      if !body.is_empty() {
        self.inner.write_all(body.as_bytes())?;
      }
      self.inner.write_all(b"\n")
    }

    /// Write all entries from an iterator.
    pub fn write_all<'a, T, I>(&mut self, entries: I) -> io::Result<()>
    where
      T: AsRef<str> + 'a,
      I: IntoIterator<Item = &'a Entry<T>>,
    {
      for entry in entries {
        self.write(entry)?;
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

/// A line iterator that yields lines without the line terminator,
/// but preserves the ability to compute offsets into the original input.
struct Lines<'a> {
  input: &'a str,
  pos: usize,
}

impl<'a> Lines<'a> {
  fn new(input: &'a str) -> Self {
    Self { input, pos: 0 }
  }
}

impl<'a> Iterator for Lines<'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<Self::Item> {
    if self.pos >= self.input.len() {
      return None;
    }

    let bytes = &self.input.as_bytes()[self.pos..];

    #[cfg(all(feature = "memchr", not(miri)))]
    let found = memchr::memchr(b'\n', bytes);
    #[cfg(not(all(feature = "memchr", not(miri))))]
    let found = bytes.iter().position(|&b| b == b'\n');

    let line_end = found
      .map(|i| {
        let end = if i > 0 && bytes[i - 1] == b'\r' {
          i - 1
        } else {
          i
        };
        (end, i + 1)
      })
      .unwrap_or((bytes.len(), bytes.len()));

    let line = &self.input[self.pos..self.pos + line_end.0];
    self.pos += line_end.1;
    Some(line)
  }
}
