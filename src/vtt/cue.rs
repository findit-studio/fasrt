//! Cue text parsing per the W3C WebVTT spec (§6.4).
//!
//! Provides a lazy [`CueParser`] iterator that yields [`CueToken`]s from raw
//! cue text, and (with `alloc`/`std`) a [`CueText`] DOM tree built on top.
//!
//! Text normalization (entity decoding, NULL replacement) is **deferred** —
//! the parser only records whether normalization is needed.  Call
//! [`CueStr::normalize`] when you actually need the decoded text.

use derive_more::{Display, IsVariant};
use logos::Logos;

use core::fmt;

pub use tree::*;

mod tree;

/// A recognized WebVTT cue text tag name.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, IsVariant)]
pub enum Tag {
  /// `<b>` — bold.
  #[display("b")]
  Bold,
  /// `<i>` — italic.
  #[display("i")]
  Italic,
  /// `<u>` — underline.
  #[display("u")]
  Underline,
  /// `<c>` — class span.
  #[display("c")]
  Class,
  /// `<ruby>` — ruby annotation container.
  #[display("ruby")]
  Ruby,
  /// `<rt>` — ruby text.
  #[display("rt")]
  RubyText,
  /// `<v>` — voice span.
  #[display("v")]
  Voice,
  /// `<lang>` — language span.
  #[display("lang")]
  Lang,
}

// ── Raw logos lexer (private) ───────────────────────────────────────────────

/// Low-level token produced by the logos DFA.
///
/// Logos classifies each tag by name at the DFA level so the iterator
/// never needs string-based tag-name lookup.
#[derive(Debug, Logos)]
enum RawCueToken<'a> {
  // ── text ──────────────────────────────────────────────────────────────
  /// A run of text (everything that is not `<`).
  #[regex(r"[^<]+")]
  Text(&'a str),

  // ── end tags (exact tokens — highest priority) ────────────────────────
  #[token("</b>")]
  EndBold,
  #[token("</i>")]
  EndItalic,
  #[token("</u>")]
  EndUnderline,
  #[token("</c>")]
  EndClass,
  #[token("</ruby>")]
  EndRuby,
  #[token("</rt>")]
  EndRubyText,
  #[token("</v>")]
  EndVoice,
  #[token("</lang>")]
  EndLang,

  // ── start tags (with optional `.classes` / ` annotation`) ─────────────
  #[regex(r"<b[. \t\n\x0C][^>]*>|<b>")]
  StartBold(&'a str),
  #[regex(r"<i[. \t\n\x0C][^>]*>|<i>")]
  StartItalic(&'a str),
  #[regex(r"<u[. \t\n\x0C][^>]*>|<u>")]
  StartUnderline(&'a str),
  #[regex(r"<c[. \t\n\x0C][^>]*>|<c>")]
  StartClass(&'a str),
  #[regex(r"<ruby[. \t\n\x0C][^>]*>|<ruby>")]
  StartRuby(&'a str),
  #[regex(r"<rt[. \t\n\x0C][^>]*>|<rt>")]
  StartRubyText(&'a str),
  #[regex(r"<v[. \t\n\x0C][^>]*>|<v>")]
  StartVoice(&'a str),
  #[regex(r"<lang[. \t\n\x0C][^>]*>|<lang>")]
  StartLang(&'a str),

  // ── timestamp tag ─────────────────────────────────────────────────────
  /// `<HH:MM:SS.mmm>` or `<MM:SS.mmm>` — fully validated by the DFA so
  /// the fast unchecked `parse_timestamp` can be used directly.
  #[regex(r"<(?:[0-9]+:)?[0-5][0-9]:[0-5][0-9]\.[0-9]{3}>")]
  Timestamp(&'a str),

  // ── fallbacks (skipped by the iterator) ───────────────────────────────
  /// Any other complete tag.
  #[regex(r"<[^>]*>")]
  UnknownTag,
  /// An unterminated tag: `<…` without a closing `>`.
  #[regex(r"<[^>]*")]
  UnterminatedTag,
}

// ── CueStr ──────────────────────────────────────────────────────────────────

/// A lazily-normalizable cue text string.
///
/// Stores the raw slice from the input and a flag indicating whether
/// normalization is needed (entity decoding, NULL → U+FFFD replacement).
/// Normalization is deferred until explicitly requested via [`normalize`].
///
/// # Zero-copy guarantee
///
/// The parser **never** allocates.  When normalization is not needed,
/// [`normalize`] returns the original borrowed slice.  When it *is* needed,
/// the decoded text is computed once and cached behind a [`core::cell::OnceCell`]
/// (requires `alloc` or `std`).
///
/// [`normalize`]: CueStr::normalize
pub struct CueStr<'a> {
  raw: &'a str,
  requires_normalization: bool,
  #[cfg(any(feature = "alloc", feature = "std"))]
  normalized: core::cell::OnceCell<std::string::String>,
}

impl<'a> CueStr<'a> {
  /// Create a `CueStr` that does **not** need normalization.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::CueStr;
  ///
  /// let s = CueStr::borrowed("hello");
  /// assert_eq!(s.as_raw(), "hello");
  /// assert!(!s.requires_normalization());
  /// ```
  pub const fn borrowed(s: &'a str) -> Self {
    Self {
      raw: s,
      requires_normalization: false,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: core::cell::OnceCell::new(),
    }
  }

  /// Create a `CueStr` that **requires** normalization (contains entities
  /// and/or NULL bytes).
  ///
  /// ```rust
  /// use fasrt::vtt::cue::CueStr;
  ///
  /// let s = CueStr::needs_normalization("a&amp;b");
  /// assert!(s.requires_normalization());
  /// assert_eq!(s.as_raw(), "a&amp;b");
  /// ```
  pub const fn needs_normalization(s: &'a str) -> Self {
    Self {
      raw: s,
      requires_normalization: true,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: core::cell::OnceCell::new(),
    }
  }

  /// Returns the raw string, without any normalization.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::CueStr;
  ///
  /// let s = CueStr::needs_normalization("a&amp;b");
  /// assert_eq!(s.as_raw(), "a&amp;b");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_raw(&self) -> &'a str {
    self.raw
  }

  /// Whether this text requires normalization (entities or NULLs present).
  ///
  /// ```rust
  /// use fasrt::vtt::cue::CueStr;
  ///
  /// assert!(!CueStr::borrowed("hello").requires_normalization());
  /// assert!(CueStr::needs_normalization("&amp;").requires_normalization());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn requires_normalization(&self) -> bool {
    self.requires_normalization
  }

  /// Returns the normalized string (entities decoded, NULLs replaced with
  /// U+FFFD).
  ///
  /// If no normalization is needed, returns the raw slice directly (no
  /// allocation). Otherwise, computes the normalized form once and caches
  /// it.
  ///
  /// On `no_std` without `alloc`, always returns the raw string.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::CueStr;
  ///
  /// let plain = CueStr::borrowed("hello");
  /// assert_eq!(plain.normalize(), "hello");
  ///
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// let entity = CueStr::needs_normalization("a&amp;b");
  /// assert_eq!(entity.normalize(), "a&b");
  /// # }
  /// ```
  pub fn normalize(&self) -> &str {
    if !self.requires_normalization {
      return self.raw;
    }

    #[cfg(any(feature = "alloc", feature = "std"))]
    {
      self.normalized.get_or_init(|| self.decode_char_refs())
    }
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    {
      self.raw
    }
  }

  /// Decode HTML5 character references and replace NULLs with U+FFFD.
  ///
  /// Implements the WHATWG "consume a character reference" algorithm used by the
  /// WebVTT cue text tokenizer (the WebVTT Living Standard delegates to the
  /// HTML spec for character reference processing).
  ///
  /// Handles named entities (with and without trailing `;`), numeric decimal
  /// (`&#32;`), and numeric hexadecimal (`&#x20;`) references.
  #[cfg(any(feature = "alloc", feature = "std"))]
  fn decode_char_refs(&self) -> std::string::String {
    let input = self.as_raw();
    let bytes = input.as_bytes();
    let len = bytes.len();

    // Fast path: no `&` or NUL means nothing to decode.
    #[cfg(all(feature = "memchr", not(miri)))]
    let has_special = memchr::memchr2(b'&', 0, bytes).is_some();
    #[cfg(not(all(feature = "memchr", not(miri))))]
    let has_special = bytes.iter().any(|&b| b == b'&' || b == 0);

    if !has_special {
      return std::string::String::from(input);
    }

    let mut out = std::string::String::with_capacity(len);
    let mut i = 0;

    while i < len {
      if bytes[i] == 0 {
        out.push('\u{FFFD}');
        i += 1;
      } else if bytes[i] == b'&' {
        i += 1; // skip '&'
        if i >= len {
          out.push('&');
          continue;
        }

        if bytes[i] == b'#' {
          // Numeric character reference
          i += 1;
          if i >= len {
            out.push_str("&#");
            continue;
          }
          let hex = bytes[i] == b'x' || bytes[i] == b'X';
          if hex {
            i += 1;
          }
          let start = i;
          if hex {
            while i < len && bytes[i].is_ascii_hexdigit() {
              i += 1;
            }
          } else {
            while i < len && bytes[i].is_ascii_digit() {
              i += 1;
            }
          }
          if start == i {
            // No digits found — output raw
            out.push_str(if hex { "&#x" } else { "&#" });
            continue;
          }
          let digits = &input[start..i];
          let code_point = if hex {
            u32::from_str_radix(digits, 16).unwrap_or(0xFFFD)
          } else {
            digits.parse::<u32>().unwrap_or(0xFFFD)
          };
          // Skip trailing ';' if present
          if i < len && bytes[i] == b';' {
            i += 1;
          }
          if code_point == 0 {
            out.push('\u{FFFD}');
          } else if let Some(c) = char::from_u32(code_point) {
            out.push(c);
          } else {
            out.push('\u{FFFD}');
          }
        } else if bytes[i].is_ascii_alphanumeric() {
          // Named character reference — find longest match in entity table
          let ref_start = i;
          // Collect alphanumeric characters and ';'
          while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b';') {
            i += 1;
            // Stop after ';'
            if bytes[i - 1] == b';' {
              break;
            }
          }
          let candidate = &input[ref_start..i];

          // Find the longest prefix that matches an entity in the table
          match Self::find_longest_entity_match(candidate) {
            Some((matched_len, decoded)) => {
              out.push_str(decoded);
              // Rewind: we consumed `candidate` but only matched `matched_len` chars
              i = ref_start + matched_len;
            }
            None => {
              // No match — output '&' + candidate as literal
              out.push('&');
              i = ref_start; // rewind to re-process as text
            }
          }
        } else {
          // '&' followed by non-alphanumeric, non-'#' — output '&' as literal
          out.push('&');
        }
      } else {
        // Find run of plain text (no '&' or NULL)
        let start = i;
        while i < len && bytes[i] != b'&' && bytes[i] != 0 {
          i += 1;
        }
        out.push_str(&input[start..i]);
      }
    }

    out
  }

  /// Find the longest prefix of `candidate` that matches an HTML5 named entity.
  ///
  /// Returns `(matched_length, decoded_str)` for the longest match, or `None`.
  /// Handles both semicolon-terminated and legacy (no semicolon) entities.
  #[cfg(any(feature = "alloc", feature = "std"))]
  fn find_longest_entity_match(candidate: &str) -> Option<(usize, &'static str)> {
    use super::html5_entities::HTML5_ENTITIES;

    // The longest HTML5 entity name is 32 chars ("CounterClockwiseContourIntegral;").
    const MAX_ENTITY_LEN: usize = 32;

    let mut best: Option<(usize, &'static str)> = None;
    let limit = candidate.len().min(MAX_ENTITY_LEN);

    for end in 1..=limit {
      let prefix = &candidate[..end];
      if let Some(s) = HTML5_ENTITIES.get(prefix) {
        best = Some((end, s));
        // A semicolon-terminated match is always the longest for this name.
        if prefix.ends_with(';') {
          break;
        }
      }
    }

    best
  }
}

impl Clone for CueStr<'_> {
  fn clone(&self) -> Self {
    Self {
      raw: self.raw,
      requires_normalization: self.requires_normalization,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: self.normalized.clone(),
    }
  }
}

impl fmt::Debug for CueStr<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("CueStr")
      .field("raw", &self.raw)
      .field("requires_normalization", &self.requires_normalization)
      .finish()
  }
}

impl PartialEq for CueStr<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.raw == other.raw && self.requires_normalization == other.requires_normalization
  }
}

impl Eq for CueStr<'_> {}

impl fmt::Display for CueStr<'_> {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    #[cfg(any(feature = "alloc", feature = "std"))]
    {
      f.write_str(self.normalize())
    }

    #[cfg(not(any(feature = "alloc", feature = "std")))]
    {
      f.write_str(self.raw)
    }
  }
}

/// A token emitted by the [`CueParser`] iterator.
///
/// This is the low-level, zero-allocation representation of cue text.
/// Users who need a DOM tree can use [`CueText::parse`] (requires `alloc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueToken<'a> {
  /// A run of plain text.
  ///
  /// The text is stored as a [`CueStr`] with lazy normalization — call
  /// [`CueStr::normalize`] to decode entities and replace NULLs.
  Text(CueStr<'a>),
  /// A start tag like `<b>`, `<c.classname>`, or `<v Speaker Name>`.
  StartTag {
    /// The tag name.
    tag: Tag,
    /// The raw dot-separated class list (e.g., `"loud.important"`), empty if
    /// none.
    ///
    /// This is the source text, not §6.4's list of applicable classes, which
    /// excludes the empty strings between adjacent separators. Read it with
    /// [`Classes::new`].
    classes: &'a str,
    /// Annotation text (for `<v>` and `<lang>`), `None` if absent.
    annotation: Option<&'a str>,
  },
  /// An end tag like `</b>`.
  EndTag(Tag),
  /// A timestamp tag like `<00:05.000>`.
  Timestamp(crate::vtt::Timestamp),
}

/// A lazy, zero-copy cue text parser backed by a [`logos`] DFA.
///
/// Yields [`CueToken`]s from raw WebVTT cue text.  The parser **never**
/// allocates — entity decoding and NULL replacement happen lazily inside
/// [`CueStr`] when the caller requests it.
///
/// # Example
///
/// ```rust
/// use fasrt::vtt::cue::{CueParser, CueToken, Tag, CueStr};
///
/// let tokens: Vec<_> = CueParser::new("<b>bold</b>").collect();
/// assert_eq!(tokens.len(), 3);
/// assert!(matches!(&tokens[0], CueToken::StartTag { tag: Tag::Bold, .. }));
/// assert!(matches!(&tokens[1], CueToken::Text(_)));
/// assert!(matches!(&tokens[2], CueToken::EndTag(Tag::Bold)));
/// ```
pub struct CueParser<'a> {
  lexer: logos::Lexer<'a, RawCueToken<'a>>,
}

impl<'a> CueParser<'a> {
  /// Create a new cue text parser for the given raw cue text.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::{CueParser, CueToken, Tag};
  ///
  /// let mut parser = CueParser::new("<b>text</b>");
  /// assert!(matches!(parser.next(), Some(CueToken::StartTag { tag: Tag::Bold, .. })));
  /// ```
  pub fn new(input: &'a str) -> Self {
    Self {
      lexer: RawCueToken::lexer(input),
    }
  }
}

/// The whitespace §6.4 ends a tag name or class list on, sending the tokenizer
/// into its start tag annotation state: TAB, LF, FF and SPACE. All four are
/// one byte, so the index of one is always a `str` boundary.
///
/// Deliberately *not* [`ASCII_WHITESPACE`]: §6.4's state transitions list these
/// four, while the trim that ends the annotation state is over Infra's ASCII
/// whitespace, which also has CR.
const ANNOTATION_DELIMITERS: [char; 4] = ['\t', '\n', '\u{000C}', ' '];

/// [Infra's ASCII whitespace][infra], which §6.4 trims from an annotation:
/// the four delimiters above plus U+000D CARRIAGE RETURN.
///
/// [infra]: https://infra.spec.whatwg.org/#ascii-whitespace
const ASCII_WHITESPACE: [char; 5] = ['\t', '\n', '\u{000C}', '\r', ' '];

/// Extract classes and annotation from the portion of a start-tag slice
/// that follows the tag name (i.e. everything between `<tagname` and `>`).
///
/// For `<b.loud.important>` the input is `".loud.important"`.
/// For `<v Roger Bingham>` the input is `" Roger Bingham"`.
#[cfg_attr(not(tarpaulin), inline(always))]
fn parse_tag_attrs(after_name: &str) -> (&str, Option<&str>) {
  if after_name.is_empty() {
    return ("", None);
  }

  let (tag_rest, annotation) = match after_name.find(ANNOTATION_DELIMITERS) {
    Some(idx) => {
      // §6.4 ends the annotation state by removing leading and trailing ASCII
      // whitespace from the buffer — Infra's five, not Unicode's set. It also
      // collapses internal runs to a single space, which this parser cannot do
      // while the annotation is a slice borrowed from the cue.
      let ann = after_name[idx + 1..].trim_matches(ASCII_WHITESPACE);
      (
        &after_name[..idx],
        if ann.is_empty() { None } else { Some(ann) },
      )
    }
    None => (after_name, None),
  };

  let classes = tag_rest.strip_prefix('.').unwrap_or("");
  (classes, annotation)
}

/// Build a [`CueToken::StartTag`] from the raw logos slice, stripping the
/// outer `<` / `>` and the tag name of the given byte length.
#[cfg_attr(not(tarpaulin), inline(always))]
fn make_start_tag<'a>(tag: Tag, slice: &'a str, name_len: usize) -> CueToken<'a> {
  // slice = "<tagname…>" — strip `<` (1 byte) + tag name + `>` (1 byte)
  let inner = &slice[1 + name_len..slice.len() - 1];
  let (classes, annotation) = parse_tag_attrs(inner);
  CueToken::StartTag {
    tag,
    classes,
    annotation,
  }
}

impl<'a> Iterator for CueParser<'a> {
  type Item = CueToken<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      let token = self.lexer.next()?;
      match token {
        // ── text ──
        Ok(RawCueToken::Text(text)) => {
          let needs_norm = text.as_bytes().iter().any(|&b| b == b'&' || b == 0);
          return Some(CueToken::Text(if needs_norm {
            CueStr::needs_normalization(text)
          } else {
            CueStr::borrowed(text)
          }));
        }

        // ── end tags ──
        Ok(RawCueToken::EndBold) => return Some(CueToken::EndTag(Tag::Bold)),
        Ok(RawCueToken::EndItalic) => return Some(CueToken::EndTag(Tag::Italic)),
        Ok(RawCueToken::EndUnderline) => return Some(CueToken::EndTag(Tag::Underline)),
        Ok(RawCueToken::EndClass) => return Some(CueToken::EndTag(Tag::Class)),
        Ok(RawCueToken::EndRuby) => return Some(CueToken::EndTag(Tag::Ruby)),
        Ok(RawCueToken::EndRubyText) => return Some(CueToken::EndTag(Tag::RubyText)),
        Ok(RawCueToken::EndVoice) => return Some(CueToken::EndTag(Tag::Voice)),
        Ok(RawCueToken::EndLang) => return Some(CueToken::EndTag(Tag::Lang)),

        // ── start tags ──
        Ok(RawCueToken::StartBold(s)) => return Some(make_start_tag(Tag::Bold, s, 1)),
        Ok(RawCueToken::StartItalic(s)) => return Some(make_start_tag(Tag::Italic, s, 1)),
        Ok(RawCueToken::StartUnderline(s)) => {
          return Some(make_start_tag(Tag::Underline, s, 1));
        }
        Ok(RawCueToken::StartClass(s)) => return Some(make_start_tag(Tag::Class, s, 1)),
        Ok(RawCueToken::StartRuby(s)) => return Some(make_start_tag(Tag::Ruby, s, 4)),
        Ok(RawCueToken::StartRubyText(s)) => {
          return Some(make_start_tag(Tag::RubyText, s, 2));
        }
        Ok(RawCueToken::StartVoice(s)) => return Some(make_start_tag(Tag::Voice, s, 1)),
        Ok(RawCueToken::StartLang(s)) => return Some(make_start_tag(Tag::Lang, s, 4)),

        // ── timestamp ──
        Ok(RawCueToken::Timestamp(s)) => {
          let content = &s[1..s.len() - 1]; // strip `<` and `>`
          // Regex already validates format; use unchecked fast path.
          if let Ok(ts) = super::parse_timestamp(content) {
            return Some(CueToken::Timestamp(ts));
          }
          // Malformed — skip
        }

        // ── unknown complete tags — skip ──
        Ok(RawCueToken::UnknownTag) | Err(()) => {}

        // ── unterminated tags — try to parse as known start tag or timestamp ──
        Ok(RawCueToken::UnterminatedTag) => {
          let s = self.lexer.slice();
          if let Some(token) = try_parse_unterminated(s) {
            return Some(token);
          }
        }
      }
    }
  }
}

/// Try to parse an unterminated tag (`<…` without `>`) as a known start tag
/// or timestamp.
///
/// Per the W3C spec, unterminated tags at end-of-input are still recognized
/// if they match a known tag name pattern.
fn try_parse_unterminated<'a>(slice: &'a str) -> Option<CueToken<'a>> {
  let inner = &slice[1..]; // strip leading `<`
  if inner.is_empty() {
    return None;
  }

  // Try timestamp: digits/colons + "." + 3 digits
  if inner.as_bytes()[0].is_ascii_digit() {
    if let Ok(ts) = super::parse_timestamp_cue(inner) {
      return Some(CueToken::Timestamp(ts));
    }
    return None;
  }

  // Try known start tags. The byte that may follow the name is the class
  // separator or one of §6.4's four annotation delimiters — the same set the
  // DFA accepts for a terminated tag.
  const DELIM: [u8; 5] = [b'.', b'\t', b'\n', 0x0C, b' '];
  let follows_name = |byte: u8| DELIM.contains(&byte);

  let (tag, name_len) = match inner.as_bytes() {
    [b'b', next, ..] if follows_name(*next) => (Tag::Bold, 1),
    [b'b'] => (Tag::Bold, 1),
    [b'i', next, ..] if follows_name(*next) => (Tag::Italic, 1),
    [b'i'] => (Tag::Italic, 1),
    [b'u', next, ..] if follows_name(*next) => (Tag::Underline, 1),
    [b'u'] => (Tag::Underline, 1),
    [b'c', next, ..] if follows_name(*next) => (Tag::Class, 1),
    [b'c'] => (Tag::Class, 1),
    [b'v', next, ..] if follows_name(*next) => (Tag::Voice, 1),
    [b'v'] => (Tag::Voice, 1),
    _ if inner.starts_with("ruby") => {
      if inner.len() == 4 || follows_name(inner.as_bytes()[4]) {
        (Tag::Ruby, 4)
      } else {
        return None;
      }
    }
    _ if inner.starts_with("rt") => {
      if inner.len() == 2 || follows_name(inner.as_bytes()[2]) {
        (Tag::RubyText, 2)
      } else {
        return None;
      }
    }
    _ if inner.starts_with("lang") && (inner.len() == 4 || follows_name(inner.as_bytes()[4])) => {
      (Tag::Lang, 4)
    }
    _ => return None,
  };

  let after_name = &inner[name_len..];
  let (classes, annotation) = parse_tag_attrs(after_name);
  Some(CueToken::StartTag {
    tag,
    classes,
    annotation,
  })
}
