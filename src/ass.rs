use crate::{error::*, utils::Lines};

pub use types::{
  Block, Event, EventField, EventFormat, EventKind, Fields, Format, Hour, Property, Section,
  StyleRow, Timestamp,
};

mod types;

/// Event text parsing: override-tag tokenization and clean-text extraction.
pub mod text;

/// The error type for parsing ASS/SSA scripts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseAssError {
  /// An error occurred while parsing the hour component of a timestamp.
  #[error(transparent)]
  ParseHour(#[from] ParseHourError),
  /// An error occurred while parsing the minute component of a timestamp.
  #[error(transparent)]
  ParseMinute(#[from] ParseMinuteError),
  /// An error occurred while parsing the second component of a timestamp.
  #[error(transparent)]
  ParseSecond(#[from] ParseSecondError),
  /// An error occurred while parsing the centisecond component of a timestamp.
  #[error(transparent)]
  ParseCentisecond(#[from] ParseCentisecondError),

  /// A timestamp could not be parsed.
  #[error("invalid timestamp: {0}")]
  InvalidTimestamp(TimestampError),

  /// A section header is missing its closing `]`.
  #[error("unclosed section header, missing ']'")]
  UnclosedSection,

  /// An event row appeared before any `Format:` line declared the field
  /// order of the `[Events]` section.
  #[error("event row before the '[Events]' section declared a 'Format:' line")]
  MissingFormat,

  /// An event row carries fewer fields than the `Format:` line declares.
  #[error("event row declares {expected} fields, but only {found} were found")]
  TooFewFields {
    /// The number of fields the `Format:` line declares.
    expected: usize,
    /// The number of fields actually present on the row.
    found: usize,
  },

  /// An event field could not be parsed.
  #[error("invalid value for the '{0}' field")]
  InvalidField(EventField),

  /// A line matched none of the shapes an ASS/SSA script may contain.
  #[error("unexpected line")]
  UnexpectedLine,

  /// An unknown error occurred.
  #[error("unexpected token: {0}")]
  Unknown(&'static str),
}

impl Default for ParseAssError {
  fn default() -> Self {
    Self::Unknown("unknown lexer error")
  }
}

/// Options that control how the ASS/SSA parser handles malformed input.
///
/// By default the parser runs in **strict** mode, where anything that does not
/// match the format is an error.  Use [`Options::lossy`] for a maximally
/// permissive preset, which is what real-world fansub scripts usually need.
///
/// With the `serde` feature, this type implements [`serde::Serialize`] and
/// [`serde::Deserialize`] as a snake_case document of its four flags; a field
/// missing on the way in takes its value from [`Options::default`] (strict),
/// so a caller may declare only the flags they want to override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case", default))]
pub struct Options {
  allow_missing_format: bool,
  allow_short_event: bool,
  allow_malformed_fields: bool,
  ignore_unknown_lines: bool,
}

impl Options {
  /// Strict preset — the default.  Every deviation from the format is an
  /// error.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn strict() -> Self {
    Self {
      allow_missing_format: false,
      allow_short_event: false,
      allow_malformed_fields: false,
      ignore_unknown_lines: false,
    }
  }

  /// Lossy preset — maximally permissive.  A missing `Format:` line falls back
  /// to the ASS v4+ order, short rows and unparsable fields leave fields
  /// absent, and unrecognizable lines are skipped.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn lossy() -> Self {
    Self {
      allow_missing_format: true,
      allow_short_event: true,
      allow_malformed_fields: true,
      ignore_unknown_lines: true,
    }
  }

  /// Returns whether event rows may appear before a `Format:` line.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// assert!(!Options::strict().allow_missing_format());
  /// assert!(Options::lossy().allow_missing_format());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn allow_missing_format(&self) -> bool {
    self.allow_missing_format
  }

  /// Sets whether event rows may appear before a `Format:` line.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let options = Options::strict().with_allow_missing_format(true);
  /// assert!(options.allow_missing_format());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_allow_missing_format(mut self, value: bool) -> Self {
    self.allow_missing_format = value;
    self
  }

  /// Sets whether event rows may appear before a `Format:` line.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let mut options = Options::strict();
  /// options.set_allow_missing_format(true);
  /// assert!(options.allow_missing_format());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_allow_missing_format(&mut self, value: bool) -> &mut Self {
    self.allow_missing_format = value;
    self
  }

  /// Returns whether event rows may carry fewer fields than declared.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// assert!(!Options::strict().allow_short_event());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn allow_short_event(&self) -> bool {
    self.allow_short_event
  }

  /// Sets whether event rows may carry fewer fields than declared.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// assert!(Options::strict().with_allow_short_event(true).allow_short_event());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_allow_short_event(mut self, value: bool) -> Self {
    self.allow_short_event = value;
    self
  }

  /// Sets whether event rows may carry fewer fields than declared.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let mut options = Options::strict();
  /// options.set_allow_short_event(true);
  /// assert!(options.allow_short_event());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_allow_short_event(&mut self, value: bool) -> &mut Self {
    self.allow_short_event = value;
    self
  }

  /// Returns whether an unparsable field is treated as absent.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// assert!(Options::lossy().allow_malformed_fields());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn allow_malformed_fields(&self) -> bool {
    self.allow_malformed_fields
  }

  /// Sets whether an unparsable field is treated as absent.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let options = Options::strict().with_allow_malformed_fields(true);
  /// assert!(options.allow_malformed_fields());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_allow_malformed_fields(mut self, value: bool) -> Self {
    self.allow_malformed_fields = value;
    self
  }

  /// Sets whether an unparsable field is treated as absent.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let mut options = Options::strict();
  /// options.set_allow_malformed_fields(true);
  /// assert!(options.allow_malformed_fields());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_allow_malformed_fields(&mut self, value: bool) -> &mut Self {
    self.allow_malformed_fields = value;
    self
  }

  /// Returns whether unrecognizable lines are skipped.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// assert!(!Options::strict().ignore_unknown_lines());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn ignore_unknown_lines(&self) -> bool {
    self.ignore_unknown_lines
  }

  /// Sets whether unrecognizable lines are skipped.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let options = Options::strict().with_ignore_unknown_lines(true);
  /// assert!(options.ignore_unknown_lines());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_ignore_unknown_lines(mut self, value: bool) -> Self {
    self.ignore_unknown_lines = value;
    self
  }

  /// Sets whether unrecognizable lines are skipped.
  ///
  /// ```rust
  /// use fasrt::ass::Options;
  ///
  /// let mut options = Options::strict();
  /// options.set_ignore_unknown_lines(true);
  /// assert!(options.ignore_unknown_lines());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_ignore_unknown_lines(&mut self, value: bool) -> &mut Self {
    self.ignore_unknown_lines = value;
    self
  }
}

impl Default for Options {
  fn default() -> Self {
    Self::strict()
  }
}

/// A lazy, zero-copy ASS/SSA parser that yields one [`Block`] per meaningful
/// line.
///
/// Created via [`Parser::strict`], [`Parser::lossy`], or
/// [`Parser::with_options`].  The parser never allocates and works on every
/// feature tier, including `no_std` without `alloc`: field positions are held
/// in a fixed-size [`EventFormat`], and every string is borrowed from the
/// input.
///
/// Line handling follows the format as VSFilter and libass implement it:
/// `[Section]` headers, `;` comments, `Format:` declarations, `Style:` rows in
/// a style section, event rows in `[Events]`, and `Key: Value` lines
/// everywhere else.  Blank lines are skipped.  LF, CRLF and CR line endings
/// are all accepted, and a leading UTF-8 BOM is ignored.
///
/// # Errors
///
/// An error ends iteration: the parser yields the error once and then `None`.
///
/// # Example
///
/// ```rust
/// # #[cfg(any(feature = "alloc", feature = "std"))]
/// # {
/// use fasrt::ass::{Block, Parser, Section};
///
/// let script = "\
/// [Script Info]
/// Title: Example
///
/// [Events]
/// Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
/// Dialogue: 0,0:00:01.00,0:00:03.00,Default,Rin,0,0,0,,Hello there
/// ";
///
/// let blocks: Vec<_> = Parser::strict(script).collect::<Result<_, _>>().unwrap();
/// assert_eq!(blocks[0], Block::Section(Section::ScriptInfo));
///
/// match blocks.last().unwrap() {
///   Block::Event(event) => {
///     assert_eq!(event.name(), Some("Rin"));
///     assert_eq!(event.text(), "Hello there");
///   }
///   other => panic!("expected an event, got {other:?}"),
/// }
/// # }
/// ```
pub struct Parser<'a> {
  lines: Lines<'a>,
  options: Options,
  section: Option<Section<'a>>,
  event_format: Option<EventFormat>,
  first_line: bool,
  done: bool,
}

impl<'a> Parser<'a> {
  /// Create a parser in **strict** mode.
  ///
  /// ```rust
  /// use fasrt::ass::Parser;
  ///
  /// let mut parser = Parser::strict("[Events]\n");
  /// assert!(parser.next().is_some());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn strict(input: &'a str) -> Self {
    Self::with_options(input, Options::strict())
  }

  /// Create a parser in **lossy** mode, with every tolerance enabled.
  ///
  /// ```rust
  /// use fasrt::ass::{Block, Parser};
  ///
  /// // No `Format:` line, so the ASS v4+ order is assumed.
  /// let script = "[Events]\nDialogue: 0,0:00:01.00,0:00:03.00,Default,Rin,0,0,0,,Hi\n";
  /// let mut events = Parser::lossy(script).filter_map(|block| match block {
  ///   Ok(Block::Event(event)) => Some(event),
  ///   _ => None,
  /// });
  /// assert_eq!(events.next().unwrap().name(), Some("Rin"));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn lossy(input: &'a str) -> Self {
    Self::with_options(input, Options::lossy())
  }

  /// Create a parser with explicit [`Options`].
  ///
  /// ```rust
  /// use fasrt::ass::{Options, Parser};
  ///
  /// let options = Options::strict().with_ignore_unknown_lines(true);
  /// let parser = Parser::with_options("garbage line\n", options);
  /// assert_eq!(parser.count(), 0);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_options(input: &'a str, options: Options) -> Self {
    Self {
      lines: Lines::new(input),
      options,
      section: None,
      event_format: None,
      first_line: true,
      done: false,
    }
  }

  /// Returns the section the parser is currently inside, if any.
  ///
  /// ```rust
  /// use fasrt::ass::{Parser, Section};
  ///
  /// let mut parser = Parser::strict("[Events]\n");
  /// assert_eq!(parser.section(), None);
  /// let _ = parser.next();
  /// assert_eq!(parser.section(), Some(Section::Events));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn section(&self) -> Option<Section<'a>> {
    self.section
  }

  /// Returns the event field order declared by the `[Events]` section so far.
  ///
  /// ```rust
  /// use fasrt::ass::{EventFormat, Parser};
  ///
  /// let script = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n";
  /// let mut parser = Parser::strict(script);
  /// while parser.next().is_some() {}
  /// assert_eq!(parser.event_format(), Some(EventFormat::ass()));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn event_format(&self) -> Option<EventFormat> {
    self.event_format
  }

  /// Resolves the field order to use for an event row, honouring
  /// [`Options::allow_missing_format`].
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn resolve_event_format(&self) -> Result<EventFormat, ParseAssError> {
    match self.event_format {
      Some(format) => Ok(format),
      None if self.options.allow_missing_format => Ok(EventFormat::ass()),
      None => Err(ParseAssError::MissingFormat),
    }
  }

  /// Records a terminal error, ending iteration.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn fail(&mut self, error: ParseAssError) -> Option<Result<Block<'a>, ParseAssError>> {
    self.done = true;
    Some(Err(error))
  }
}

impl<'a> Iterator for Parser<'a> {
  type Item = Result<Block<'a>, ParseAssError>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if self.done {
        return None;
      }

      let mut raw_line = self.lines.next()?;
      if self.first_line {
        self.first_line = false;
        raw_line = raw_line.trim_start_matches('\u{feff}');
      }

      // Leading whitespace is structural noise; trailing whitespace is not,
      // because it can belong to an event's `Text` field. Resource payload
      // lines are kept exactly as they arrived, hence `raw_line`.
      let line = raw_line.trim_start();
      if line.trim_end().is_empty() {
        continue;
      }

      // `[Section]`
      if let Some(rest) = line.strip_prefix('[') {
        let Some(name) = rest.trim_end().strip_suffix(']') else {
          if self.options.ignore_unknown_lines {
            continue;
          }
          return self.fail(ParseAssError::UnclosedSection);
        };
        let section = Section::new(name);
        self.section = Some(section);
        // A fresh `[Events]` section must declare its own field order.
        if section.is_events() {
          self.event_format = None;
        }
        return Some(Ok(Block::Section(section)));
      }

      // Inside `[Fonts]` / `[Graphics]` every line that is not the section's
      // own header is encoded payload, and this must be decided before the
      // comment and `Key: Value` rules below: the payload alphabet spans
      // U+0021..=U+0061, so a payload line can legitimately begin with `;` or
      // contain `:`. Only the exact lowercase header libass looks for is
      // treated as a property.
      if let Some(section) = self.section
        && section.is_resource()
      {
        let header = match section {
          Section::Fonts => "fontname:",
          _ => "filename:",
        };
        if !line.starts_with(header) {
          return Some(Ok(Block::Data(raw_line)));
        }
      }

      // `; comment`
      if let Some(rest) = line.strip_prefix(';') {
        return Some(Ok(Block::Comment(rest)));
      }

      // `Key: Value`
      let Some((key, value)) = line.split_once(':') else {
        if self.options.ignore_unknown_lines {
          continue;
        }
        return self.fail(ParseAssError::UnexpectedLine);
      };
      let key = key.trim_end();
      let value = value.trim_start_matches([' ', '\t']);

      if key.is_empty() {
        if self.options.ignore_unknown_lines {
          continue;
        }
        return self.fail(ParseAssError::UnexpectedLine);
      }

      if key.eq_ignore_ascii_case("Format") {
        let format = Format::new(value);
        if self.section == Some(Section::Events) {
          self.event_format = Some(format.event_format());
        }
        return Some(Ok(Block::Format(format)));
      }

      if key.eq_ignore_ascii_case("Style")
        && self.section.is_some_and(|section| section.is_styles())
      {
        return Some(Ok(Block::Style(StyleRow::new(value))));
      }

      if self.section == Some(Section::Events)
        && let Some(kind) = EventKind::new(key)
      {
        let format = match self.resolve_event_format() {
          Ok(format) => format,
          Err(error) => return self.fail(error),
        };
        return match Event::parse_fields_with(kind, value, &format, &self.options) {
          Ok(event) => Some(Ok(Block::Event(event))),
          Err(error) => self.fail(error),
        };
      }

      return Some(Ok(Block::Property(Property::new(key, value))));
    }
  }
}

/// An ASS/SSA script writer.
///
/// The writer emits the canonical form of every block: fields are rendered
/// from their parsed values, so non-canonical spellings in the input (padded
/// margins such as `0000`, say) are normalized.  The `Text` field, style rows
/// and `Format:` declarations are written back verbatim, so an event's markup
/// survives a round-trip byte for byte.
///
/// Lines are terminated with LF.  Real-world scripts are commonly CRLF; the
/// parser accepts either.
///
/// Event rows are written in the field order the writer currently holds, which
/// starts as [`EventFormat::ass`] and is updated whenever a `Format:` block is
/// written inside an `[Events]` section.  Feeding the writer the blocks a
/// [`Parser`] produced therefore reproduces the original field order.
///
/// # Example
///
/// ```rust
/// use fasrt::ass::{Block, Event, EventKind, Parser, Writer};
///
/// let script = "\
/// [Events]
/// Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
/// Dialogue: 0,0:00:01.00,0:00:03.00,Default,Rin,0,0,0,,Hello there
/// ";
///
/// let blocks: Vec<_> = Parser::strict(script).collect::<Result<_, _>>().unwrap();
///
/// let mut buf = Vec::new();
/// let mut writer = Writer::new(&mut buf);
/// writer.write_all(&blocks).unwrap();
///
/// assert_eq!(String::from_utf8(buf).unwrap(), script);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct Writer<W> {
  inner: W,
  format: EventFormat,
  /// The order to fall back to when an `[Events]` section declares none, kept
  /// so that a second `[Events]` section cannot inherit the first one's
  /// `Format:` line — the parser resets there too.
  default_format: EventFormat,
  in_events: bool,
  has_written: bool,
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
const _: () = {
  use std::io::{self, Write};

  impl<W: Write> Writer<W> {
    /// Create a writer that emits event rows in the ASS v4+ field order.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub const fn new(inner: W) -> Self {
      Self {
        inner,
        format: EventFormat::ass(),
        default_format: EventFormat::ass(),
        in_events: false,
        has_written: false,
      }
    }

    /// Create a writer that emits event rows in the given field order.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventFormat, EventKind, Writer};
    ///
    /// let mut buf = Vec::new();
    /// let mut writer = Writer::with_event_format(&mut buf, EventFormat::matroska());
    /// let event = Event::new(EventKind::Dialogue, "Hi")
    ///   .with_read_order(Some(3))
    ///   .with_style(Some("Default"));
    /// writer.write_event(&event).unwrap();
    ///
    /// assert_eq!(String::from_utf8(buf).unwrap(), "Dialogue: 3,,Default,,,,,,Hi\n");
    /// ```
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub const fn with_event_format(inner: W, format: EventFormat) -> Self {
      Self {
        inner,
        format,
        default_format: format,
        in_events: false,
        has_written: false,
      }
    }

    /// Returns the field order event rows are currently written in.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub const fn event_format(&self) -> EventFormat {
      self.format
    }

    /// Sets the field order event rows are written in.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub const fn set_event_format(&mut self, format: EventFormat) -> &mut Self {
      self.format = format;
      self
    }

    /// Write a single block.
    ///
    /// A blank line is emitted before every section header except the first,
    /// which is the layout every ASS/SSA authoring tool produces and what the
    /// parser skips on the way back in.
    pub fn write(&mut self, block: &Block<'_>) -> io::Result<()> {
      let result = self.write_block(block);
      self.has_written = true;
      result
    }

    /// Writes one block, without updating the "something has been written"
    /// flag.
    fn write_block(&mut self, block: &Block<'_>) -> io::Result<()> {
      match block {
        Block::Section(section) => {
          if self.has_written {
            self.inner.write_all(b"\n")?;
          }
          self.in_events = section.is_events();
          if self.in_events {
            // A fresh `[Events]` section must declare its own field order;
            // carrying the previous section's over would silently re-order
            // every row it contains.
            self.format = self.default_format;
          }
          writeln!(self.inner, "[{}]", section.as_str())
        }
        Block::Comment(text) => writeln!(self.inner, ";{text}"),
        Block::Format(format) => {
          if self.in_events {
            self.format = format.event_format();
          }
          writeln!(self.inner, "Format: {}", format.as_str())
        }
        Block::Style(row) => writeln!(self.inner, "Style: {}", row.as_str()),
        Block::Data(payload) => writeln!(self.inner, "{payload}"),
        Block::Event(event) => self.write_event(event),
        Block::Property(property) => {
          writeln!(self.inner, "{}: {}", property.key(), property.value())
        }
      }
    }

    /// Write every block from an iterator.
    pub fn write_all<'b, 'c, I>(&mut self, blocks: I) -> io::Result<()>
    where
      I: IntoIterator<Item = &'b Block<'c>>,
      'c: 'b,
    {
      for block in blocks {
        self.write(block)?;
      }
      Ok(())
    }

    /// Write a single event row in the writer's current field order.
    ///
    /// Fields that the format declares but the event does not carry are
    /// written as empty, which keeps the commas aligned.  A column the format
    /// declares under a name this crate does not recognize is written back
    /// with the text the parsed row held there, via
    /// [`Event::field`](crate::ass::Event::field), so declaring a vendor or
    /// future field does not lose it.
    ///
    /// ```rust
    /// use fasrt::ass::{Event, EventKind, Timestamp, Writer};
    ///
    /// let event = Event::new(EventKind::Dialogue, "Hello")
    ///   .with_layer(Some(0))
    ///   .with_start(Some(Timestamp::parse("0:00:01.00").unwrap()))
    ///   .with_end(Some(Timestamp::parse("0:00:03.00").unwrap()))
    ///   .with_style(Some("Default"))
    ///   .with_name(Some("Rin"))
    ///   .with_margin_l(Some(0))
    ///   .with_margin_r(Some(0))
    ///   .with_margin_v(Some(0));
    ///
    /// let mut buf = Vec::new();
    /// Writer::new(&mut buf).write_event(&event).unwrap();
    /// assert_eq!(
    ///   String::from_utf8(buf).unwrap(),
    ///   "Dialogue: 0,0:00:01.00,0:00:03.00,Default,Rin,0,0,0,,Hello\n",
    /// );
    /// ```
    pub fn write_event(&mut self, event: &Event<'_>) -> io::Result<()> {
      self.has_written = true;
      write!(self.inner, "{}: ", event.kind().as_str())?;

      for index in 0..self.format.len() {
        if index > 0 {
          self.inner.write_all(b",")?;
        }
        let Some(field) = self.format.field_at(index) else {
          // A column this crate does not recognize: emit whatever the row
          // originally held there, so the declaration is not data loss.
          if let Some(raw) = event.field(index) {
            self.inner.write_all(raw.as_bytes())?;
          }
          continue;
        };
        match field {
          EventField::ReadOrder => write_opt(&mut self.inner, event.read_order())?,
          EventField::Marked => write_opt(&mut self.inner, event.marked())?,
          EventField::Layer => write_opt(&mut self.inner, event.layer())?,
          EventField::Start => write_timestamp(&mut self.inner, event.start())?,
          EventField::End => write_timestamp(&mut self.inner, event.end())?,
          EventField::Style => write_opt(&mut self.inner, event.style())?,
          EventField::Name => write_opt(&mut self.inner, event.name())?,
          EventField::MarginL => write_opt(&mut self.inner, event.margin_l())?,
          EventField::MarginR => write_opt(&mut self.inner, event.margin_r())?,
          EventField::MarginV => write_opt(&mut self.inner, event.margin_v())?,
          EventField::Effect => write_opt(&mut self.inner, event.effect())?,
          EventField::Text => self.inner.write_all(event.text().as_bytes())?,
        }
      }

      self.inner.write_all(b"\n")
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

  /// Writes an optional field, emitting nothing when it is absent.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn write_opt<W: Write, T: core::fmt::Display>(w: &mut W, value: Option<T>) -> io::Result<()> {
    match value {
      Some(value) => write!(w, "{value}"),
      None => Ok(()),
    }
  }

  /// Writes an optional timestamp in the ASS/SSA wire form.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn write_timestamp<W: Write>(w: &mut W, value: Option<Timestamp>) -> io::Result<()> {
    match value {
      Some(value) => w.write_all(value.encode().as_str().as_bytes()),
      None => Ok(()),
    }
  }
};
