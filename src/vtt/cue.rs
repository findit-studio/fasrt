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
  #[cfg(any(feature = "alloc", feature = "std"))]
  fn decode_char_refs(&self) -> std::string::String {
    let input = self.as_raw();
    let bytes = input.as_bytes();

    // Fast path: no `&` or NUL means nothing to decode. The data state keeps
    // every other character as it stands, so the input is already the answer.
    #[cfg(all(feature = "memchr", not(miri)))]
    let has_special = memchr::memchr2(b'&', 0, bytes).is_some();
    #[cfg(not(all(feature = "memchr", not(miri))))]
    let has_special = bytes.iter().any(|&b| b == b'&' || b == 0);

    if !has_special {
      return std::string::String::from(input);
    }

    let mut out = std::string::String::with_capacity(bytes.len());
    decode_char_refs_into(input, &mut out);
    out
  }
}

// ── character reference decoding ────────────────────────────────────────────

/// Where [`decode_char_refs_into`] puts the text it decodes.
///
/// §6.4 runs the same character-reference machinery from two of its states —
/// the data state, through *WebVTT HTML character reference in data state*, and
/// the annotation state, through *WebVTT HTML character reference in annotation
/// state*. The two differ in nothing but what becomes of the decoded
/// characters: the data state keeps them, while the annotation state goes on to
/// trim and collapse their whitespace. So there is one decoder and two sinks,
/// rather than a second decoder that could drift from the first.
#[cfg(any(feature = "alloc", feature = "std"))]
trait DecodeSink {
  /// Appends a run of already-decoded text.
  fn append_str(&mut self, text: &str);

  /// Appends one already-decoded character.
  fn append_char(&mut self, c: char);
}

/// The data state's sink: the decoded text, verbatim.
#[cfg(any(feature = "alloc", feature = "std"))]
impl DecodeSink for std::string::String {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn append_str(&mut self, text: &str) {
    self.push_str(text);
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn append_char(&mut self, c: char) {
    self.push(c);
  }
}

/// The annotation state's sink, which performs the last step §6.4 takes before
/// it returns a start tag: *"Remove any leading or trailing ASCII whitespace
/// characters from buffer, and replace any sequence of one or more consecutive
/// ASCII whitespace characters in buffer with a single U+0020 SPACE
/// character."*
///
/// The buffer §6.4 speaks of holds *decoded* characters, so this sits
/// downstream of the decoder rather than over the source text: `a&#x20; b`
/// carries one run of two whitespace characters, not two runs of one, and
/// `&#x20;a` is padded rather than starting with a space.
#[cfg(any(feature = "alloc", feature = "std"))]
struct CollapsingSink<'o> {
  out: &'o mut std::string::String,
  /// Whitespace has arrived since the last character was written. It becomes a
  /// single U+0020 if anything else follows and nothing if not, which is the
  /// trailing trim; the leading trim is the same rule with `out` still empty.
  pending_space: bool,
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl CollapsingSink<'_> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn push(&mut self, c: char) {
    // ASCII whitespace is Infra's five, the set already named for the trim
    // that ends this state — read from the same constant so the two cannot
    // drift apart.
    if ASCII_WHITESPACE.contains(&c) {
      self.pending_space = true;
      return;
    }
    if core::mem::take(&mut self.pending_space) && !self.out.is_empty() {
      self.out.push(' ');
    }
    self.out.push(c);
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl DecodeSink for CollapsingSink<'_> {
  fn append_str(&mut self, text: &str) {
    for c in text.chars() {
      self.push(c);
    }
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn append_char(&mut self, c: char) {
    self.push(c);
  }
}

/// Decode HTML5 character references into `sink` and replace NULLs with U+FFFD.
///
/// Implements the WHATWG "consume a character reference" algorithm used by the
/// WebVTT cue text tokenizer (the WebVTT Living Standard delegates to the HTML
/// spec for character reference processing).
///
/// Handles named entities (with and without trailing `;`), numeric decimal
/// (`&#32;`), and numeric hexadecimal (`&#x20;`) references.
#[cfg(any(feature = "alloc", feature = "std"))]
fn decode_char_refs_into<S: DecodeSink + ?Sized>(input: &str, sink: &mut S) {
  let bytes = input.as_bytes();
  let len = bytes.len();
  let mut i = 0;

  while i < len {
    if bytes[i] == 0 {
      sink.append_char('\u{FFFD}');
      i += 1;
    } else if bytes[i] == b'&' {
      i += 1; // skip '&'
      if i >= len {
        sink.append_char('&');
        continue;
      }

      if bytes[i] == b'#' {
        // Numeric character reference
        i += 1;
        if i >= len {
          sink.append_str("&#");
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
          sink.append_str(if hex { "&#x" } else { "&#" });
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
        // HTML's numeric character reference end state: a NULL, a surrogate
        // and a value past U+10FFFF each become U+FFFD, and a legacy C1 code
        // point is replaced by the character the author meant.
        if code_point == 0 {
          sink.append_char('\u{FFFD}');
        } else if let Some(c) = char::from_u32(replace_legacy_c1(code_point)) {
          sink.append_char(c);
        } else {
          sink.append_char('\u{FFFD}');
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
        match find_longest_entity_match(candidate) {
          Some((matched_len, decoded)) => {
            sink.append_str(decoded);
            // Rewind: we consumed `candidate` but only matched `matched_len` chars
            i = ref_start + matched_len;
          }
          None => {
            // No match — output '&' + candidate as literal
            sink.append_char('&');
            i = ref_start; // rewind to re-process as text
          }
        }
      } else {
        // '&' followed by non-alphanumeric, non-'#' — output '&' as literal
        sink.append_char('&');
      }
    } else {
      // Find run of plain text (no '&' or NULL)
      let start = i;
      while i < len && bytes[i] != b'&' && bytes[i] != 0 {
        i += 1;
      }
      sink.append_str(&input[start..i]);
    }
  }
}

/// Replace a legacy C1 code point as HTML's [numeric character reference end
/// state][end-state] does, and return every other code point unchanged.
///
/// A numeric reference in the 0x80–0x9F range was almost always written by an
/// author who meant the Windows-1252 character at that byte, so the HTML
/// tokenizer — which WebVTT §6.4 consumes references through, in the data state
/// and the annotation state alike — substitutes it rather than yielding a C1
/// control. The five code points in that range the table omits (0x81, 0x8D,
/// 0x8F, 0x90 and 0x9D) have no Windows-1252 character and pass through.
///
/// [end-state]: https://html.spec.whatwg.org/multipage/parsing.html#numeric-character-reference-end-state
#[cfg(any(feature = "alloc", feature = "std"))]
const fn replace_legacy_c1(code_point: u32) -> u32 {
  match code_point {
    0x80 => 0x20AC, // EURO SIGN
    0x82 => 0x201A, // SINGLE LOW-9 QUOTATION MARK
    0x83 => 0x0192, // LATIN SMALL LETTER F WITH HOOK
    0x84 => 0x201E, // DOUBLE LOW-9 QUOTATION MARK
    0x85 => 0x2026, // HORIZONTAL ELLIPSIS
    0x86 => 0x2020, // DAGGER
    0x87 => 0x2021, // DOUBLE DAGGER
    0x88 => 0x02C6, // MODIFIER LETTER CIRCUMFLEX ACCENT
    0x89 => 0x2030, // PER MILLE SIGN
    0x8A => 0x0160, // LATIN CAPITAL LETTER S WITH CARON
    0x8B => 0x2039, // SINGLE LEFT-POINTING ANGLE QUOTATION MARK
    0x8C => 0x0152, // LATIN CAPITAL LIGATURE OE
    0x8E => 0x017D, // LATIN CAPITAL LETTER Z WITH CARON
    0x91 => 0x2018, // LEFT SINGLE QUOTATION MARK
    0x92 => 0x2019, // RIGHT SINGLE QUOTATION MARK
    0x93 => 0x201C, // LEFT DOUBLE QUOTATION MARK
    0x94 => 0x201D, // RIGHT DOUBLE QUOTATION MARK
    0x95 => 0x2022, // BULLET
    0x96 => 0x2013, // EN DASH
    0x97 => 0x2014, // EM DASH
    0x98 => 0x02DC, // SMALL TILDE
    0x99 => 0x2122, // TRADE MARK SIGN
    0x9A => 0x0161, // LATIN SMALL LETTER S WITH CARON
    0x9B => 0x203A, // SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
    0x9C => 0x0153, // LATIN SMALL LIGATURE OE
    0x9E => 0x017E, // LATIN SMALL LETTER Z WITH CARON
    0x9F => 0x0178, // LATIN CAPITAL LETTER Y WITH DIAERESIS
    other => other,
  }
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

// ── Annotation ──────────────────────────────────────────────────────────────

/// A start tag's annotation — `<v>`'s voice, `<lang>`'s language — as W3C
/// WebVTT §6.4's *start tag annotation state* reads it.
///
/// §6.4 accumulates an annotation into a buffer and then, on the `>` that ends
/// the tag, decodes the character references it met along the way, removes
/// leading and trailing [ASCII whitespace][infra] and replaces *"any sequence of
/// one or more consecutive ASCII whitespace characters ... with a single U+0020
/// SPACE character"*. [`normalize`](Self::normalize) is that value;
/// [`as_raw`](Self::as_raw) is the text as the cue spelled it.
///
/// ```rust
/// use fasrt::vtt::cue::Annotation;
///
/// # #[cfg(any(feature = "alloc", feature = "std"))]
/// # {
/// // Character references are decoded, and the decoded text is what the
/// // whitespace rule then sees — `&#x20;` is a space like any other.
/// assert_eq!(Annotation::new("en&#x2D;US").normalize(), "en-US");
/// assert_eq!(Annotation::new("Roger&#x20; Bingham").normalize(), "Roger Bingham");
///
/// // A run of any width, of any of the five, becomes one space.
/// assert_eq!(Annotation::new("Esme\t\n  Vale").normalize(), "Esme Vale");
/// # }
///
/// // The stored text is untouched, on every feature tier.
/// assert_eq!(Annotation::new("en&#x2D;US").as_raw(), "en&#x2D;US");
/// ```
///
/// # Why both faces exist
///
/// [`as_raw`](Self::as_raw) is what a start tag is written back from, and it
/// has to be: the normalized value may hold a U+003E GREATER-THAN SIGN that a
/// `&gt;` stood for, and writing *that* into a start tag would end the tag
/// early — `<v a&gt;b>` would become `<v a>` followed by the text `b>`. The
/// same goes for an ampersand a `&amp;` stood for, which would be read back as
/// the start of a character reference. So the crate keeps the source text,
/// serializes from it, and hands the spec's value to
/// [`normalize`](Self::normalize). For that reason this type deliberately
/// implements no [`Display`](core::fmt::Display): there is no one right answer
/// to give a formatter, and a silent wrong one corrupts a document.
///
/// # Zero-copy guarantee, and the tier without `alloc`
///
/// The parser **never** allocates. An annotation already in §6.4's normal form
/// — no character references, no NULs, and no ASCII whitespace but for single
/// U+0020 SPACEs between other characters — is its own normalized value, so
/// [`normalize`](Self::normalize) hands back the borrowed slice. Only an
/// annotation that is not decodes, once, into a
/// [`OnceCell`](core::cell::OnceCell) (which needs `alloc` or `std`).
///
/// Without `alloc` there is nowhere to put a decoded string, so
/// [`normalize`](Self::normalize) returns the stored text — the same honest
/// degradation [`CueStr::normalize`] already makes for cue text. What is *not*
/// tier-dependent is the shape of the answer: the same annotations are present
/// on every tier, spelled the same way, and
/// [`requires_normalization`](Self::requires_normalization) tells a no-`alloc`
/// caller when the text it is holding is not the spec's value.
///
/// # Presence
///
/// An annotation whose stored text is empty once trimmed is reported as absent
/// rather than as an empty one: `<v>` and `<v   >` both give `None`. §6.4 draws
/// no such line — every start tag carries a buffer, empty or not — so a caller
/// must read `None` and an annotation that normalizes to `""` as the same
/// answer. The one way to reach the latter is an annotation built entirely
/// from character references for whitespace, as in `<v &#x20;>`.
///
/// # Equality
///
/// Two annotations are equal when their stored text is equal. Whether
/// normalization is needed is derived from that text, and the decoded value is
/// a cache, so the stored text is the whole value.
///
/// [infra]: https://infra.spec.whatwg.org/#ascii-whitespace
pub struct Annotation<'a> {
  raw: &'a str,
  requires_normalization: bool,
  #[cfg(any(feature = "alloc", feature = "std"))]
  normalized: core::cell::OnceCell<std::string::String>,
}

impl<'a> Annotation<'a> {
  /// Stores `raw` as an annotation, working out whether §6.4's annotation
  /// state would change it.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::Annotation;
  ///
  /// let plain = Annotation::new("Roger Bingham");
  /// assert_eq!(plain.as_raw(), "Roger Bingham");
  /// assert!(!plain.requires_normalization());
  ///
  /// assert!(Annotation::new("Roger  Bingham").requires_normalization());
  /// assert!(Annotation::new("en&#x2D;US").requires_normalization());
  /// ```
  pub const fn new(raw: &'a str) -> Self {
    Self {
      raw,
      requires_normalization: is_outside_annotation_normal_form(raw),
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: core::cell::OnceCell::new(),
    }
  }

  /// Returns the annotation as the cue spelled it, undecoded and uncollapsed.
  ///
  /// This is the text a start tag is serialized from; see the type's own
  /// documentation for why the normalized value cannot be.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::Annotation;
  ///
  /// assert_eq!(Annotation::new("en&#x2D;US").as_raw(), "en&#x2D;US");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_raw(&self) -> &'a str {
    self.raw
  }

  /// Whether the stored text differs from §6.4's annotation, so that
  /// [`normalize`](Self::normalize) has work to do.
  ///
  /// This is the question a caller without `alloc` asks before trusting
  /// [`as_raw`](Self::as_raw) as the spec's value.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::Annotation;
  ///
  /// assert!(!Annotation::new("Roger Bingham").requires_normalization());
  /// assert!(Annotation::new("Roger\tBingham").requires_normalization());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn requires_normalization(&self) -> bool {
    self.requires_normalization
  }

  /// Returns §6.4's annotation: character references decoded, NULs replaced
  /// with U+FFFD, ASCII whitespace trimmed at both ends and every run of it
  /// within collapsed to a single U+0020 SPACE.
  ///
  /// An annotation already in that form is returned borrowed, with no
  /// allocation; any other is decoded once and cached. Without `alloc` there is
  /// nowhere to cache it, so the stored text is returned instead — ask
  /// [`requires_normalization`](Self::requires_normalization) whether that is
  /// the spec's value.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::Annotation;
  ///
  /// assert_eq!(Annotation::new("en&#x2D;US").normalize(), "en-US");
  /// assert_eq!(Annotation::new("Roger \t Bingham").normalize(), "Roger Bingham");
  /// # }
  /// ```
  pub fn normalize(&self) -> &str {
    if !self.requires_normalization {
      return self.raw;
    }

    #[cfg(any(feature = "alloc", feature = "std"))]
    {
      self.normalized.get_or_init(|| {
        let mut out = std::string::String::with_capacity(self.raw.len());
        let mut sink = CollapsingSink {
          out: &mut out,
          pending_space: false,
        };
        decode_char_refs_into(self.raw, &mut sink);
        out
      })
    }
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    {
      self.raw
    }
  }
}

/// Whether §6.4's annotation state would return something other than `raw`.
///
/// It would if the text carries a character reference to decode (`&`) or a NUL
/// to replace, or if its ASCII whitespace is not already collapsed: anything
/// but U+0020 is rewritten as U+0020, a run of two is rewritten as one, and a
/// space at either end is removed.
///
/// A conservative `true` costs only the allocating path, which produces the
/// same string; a wrong `false` would report un-normalized text as the spec's
/// value, so every check here errs towards `true` — a `&` that turns out to
/// decode to itself is one such case.
const fn is_outside_annotation_normal_form(raw: &str) -> bool {
  let bytes = raw.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'&' | 0 | b'\t' | b'\n' | 0x0C | b'\r' => return true,
      // A space is only in normal form between two other characters.
      b' ' if i == 0 || i + 1 == bytes.len() || bytes[i + 1] == b' ' => return true,
      _ => i += 1,
    }
  }
  false
}

impl Clone for Annotation<'_> {
  fn clone(&self) -> Self {
    Self {
      raw: self.raw,
      requires_normalization: self.requires_normalization,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: self.normalized.clone(),
    }
  }
}

impl fmt::Debug for Annotation<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Annotation")
      .field("raw", &self.raw)
      .field("requires_normalization", &self.requires_normalization)
      .finish()
  }
}

impl PartialEq for Annotation<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.raw == other.raw
  }
}

impl Eq for Annotation<'_> {}

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
    /// Annotation text (for `<v>` and `<lang>`), `None` if the tag declared
    /// none.
    ///
    /// [`Annotation::normalize`] is §6.4's value for it — character references
    /// decoded, whitespace runs collapsed — and [`Annotation::as_raw`] the text
    /// the cue spelled.
    annotation: Option<Annotation<'a>>,
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
fn parse_tag_attrs(after_name: &str) -> (&str, Option<Annotation<'_>>) {
  if after_name.is_empty() {
    return ("", None);
  }

  let (tag_rest, annotation) = match after_name.find(ANNOTATION_DELIMITERS) {
    Some(idx) => {
      // §6.4 ends the annotation state by removing leading and trailing ASCII
      // whitespace from the buffer — Infra's five, not Unicode's set. Trimming
      // the source text here is the same operation for every annotation whose
      // ends are literal whitespace, and it decides presence without needing to
      // decode; the ends that only *become* whitespace once a character
      // reference is decoded are trimmed by `Annotation::normalize`, along with
      // the runs within.
      let ann = after_name[idx + 1..].trim_matches(ASCII_WHITESPACE);
      (
        &after_name[..idx],
        if ann.is_empty() {
          None
        } else {
          Some(Annotation::new(ann))
        },
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
