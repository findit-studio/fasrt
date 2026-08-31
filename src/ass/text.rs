//! Event text parsing: override-tag tokenization and clean-text extraction.
//!
//! This is the ASS/SSA counterpart of [`crate::vtt::cue`], and it has the same
//! two-layer shape:
//!
//! 1. [`TextParser`] — a lazy, zero-allocation [`logos`] DFA token stream over
//!    the raw `Text` field of an event.  Usable on every feature tier,
//!    including `no_std` without `alloc`.
//! 2. [`PlainText`] — the clean-text layer.  Override blocks are dropped,
//!    `\N` becomes a line break, drawing-mode payloads are skipped, and
//!    normalization is **deferred**: a field that contains no `{` and no `\`
//!    never allocates and [`PlainText::normalize`] returns the borrowed input
//!    unchanged.
//!
//! Both layers take a plain `&str`, so an embedded Matroska `S_TEXT/ASS` track
//! — which arrives one event per packet — can be processed without ever
//! building a whole-file document.
//!
//! # Reference
//!
//! ASS/SSA has no formal specification.  The behaviour implemented here
//! follows the SSA v4.00 specification document together with the
//! widely-deployed VSFilter/libass renderers, and is documented per item where
//! those differ from a naive reading.
//!
//! # Non-goals
//!
//! Tag *semantics* are out of scope: this module tokenizes `\pos(…)`, `\k…`
//! and friends but never interprets them.  Drawing-mode payloads are
//! recognized and skipped, never parsed as geometry.

use logos::Logos;

use core::fmt;

/// Override tag names recognized by [`OverrideTag::is_known`], ordered
/// longest-first so that prefix matching yields the longest match (`\fscx`
/// before `\fs`, `\iclip` before `\i`).
///
/// Names are matched **case-sensitively**, as libass does; `\K` is therefore a
/// distinct tag from `\k`.
static KNOWN_TAGS: &[&str] = &[
  // 5 characters
  "alpha", "iclip", "xbord", "xshad", "ybord", "yshad", //
  // 4 characters
  "blur", "bord", "clip", "fade", "fscx", "fscy", "move", "shad", //
  // 3 characters
  "fad", "fax", "fay", "frx", "fry", "frz", "fsc", "fsp", "org", "pbo", "pos", //
  // 2 characters
  "1a", "1c", "2a", "2c", "3a", "3c", "4a", "4c", "an", "be", "fe", "fn", "fr", "fs", "kf", "ko",
  "kt", //
  // 1 character
  "K", "a", "b", "c", "i", "k", "p", "q", "r", "s", "t", "u",
];

/// A run of literal text, an escape, an override block, or a drawing payload.
///
/// Produced by [`TextParser`].  Every payload borrows directly from the input,
/// so the token stream never allocates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextToken<'a> {
  /// A run of literal text.
  ///
  /// A `\` that begins none of `\N`, `\n`, `\h`, `\{` or `\}` is literal text
  /// and is reported here **on its own**: libass emits the backslash and
  /// advances a single byte, so the character after it is examined afresh.
  /// That is why `\\N` is a literal backslash followed by a hard break, and
  /// not the three literal characters.
  ///
  /// A `{` that opens no block is likewise reported here, so a run of literal
  /// text may arrive as several consecutive `Text` tokens.
  Text(&'a str),
  /// A `\{` or `\}` escape.
  ///
  /// The payload is the brace alone — the character that is rendered — so
  /// `\{` yields `"{"`.
  EscapedBrace(&'a str),
  /// A hard line break (`\N`).
  HardBreak,
  /// A soft line break (`\n`).
  ///
  /// Whether a renderer draws this as a break or as a space depends on the
  /// wrap style (`\q`), which is a rendering concern and out of scope here;
  /// the token is kept distinct from [`HardBreak`](Self::HardBreak) so callers
  /// can apply their own policy.
  SoftBreak,
  /// A non-breaking space (`\h`).
  HardSpace,
  /// A brace-delimited override block, e.g. `{\i1\pos(10,20)}`.
  Override(Override<'a>),
  /// A payload emitted while vector-drawing mode is active, i.e. after a
  /// `{\p<n>}` with `n > 0` and before the matching `{\p0}`.
  ///
  /// The payload is returned verbatim and is never interpreted as geometry.
  /// A single drawing run may be reported as several consecutive `Drawing`
  /// tokens.
  Drawing(&'a str),
}

/// A brace-delimited override block, e.g. `{\i1\pos(10,20)}`.
///
/// The stored slice is the block's *content*, with the surrounding braces
/// stripped.  A `{` with no `}` after it never forms a block — see
/// [`TextParser`] — so an `Override` is always terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Override<'a> {
  raw: &'a str,
}

impl<'a> Override<'a> {
  /// Create an override block from its content, without the braces.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// let block = Override::new("\\i1");
  /// assert_eq!(block.as_str(), "\\i1");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(raw: &'a str) -> Self {
    Self { raw }
  }

  /// Returns the block's content, without the surrounding braces.
  ///
  /// ```rust
  /// use fasrt::ass::text::{Override, TextParser, TextToken};
  ///
  /// let tokens: Vec<_> = TextParser::new("{\\b1}bold").collect();
  /// assert_eq!(tokens[0], TextToken::Override(Override::new("\\b1")));
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'a str {
    self.raw
  }

  /// Returns a lazy iterator over the individual override tags in this block.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// let block = Override::new("\\i1\\pos(10,20)");
  /// let tags: Vec<_> = block.tags().map(|t| t.name()).collect();
  /// assert_eq!(tags, ["i", "pos"]);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn tags(&self) -> OverrideTags<'a> {
    OverrideTags { rest: self.raw }
  }

  /// Returns the drawing scale set by this block, if it contains a `\p` tag.
  ///
  /// `Some(0)` means drawing mode is switched **off**, `Some(n)` with `n > 0`
  /// switches it on.  When a block contains several `\p` tags the last one
  /// wins, matching left-to-right evaluation.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// assert_eq!(Override::new("\\p1").drawing_scale(), Some(1));
  /// assert_eq!(Override::new("\\p0").drawing_scale(), Some(0));
  /// assert_eq!(Override::new("\\pos(1,2)").drawing_scale(), None);
  /// assert_eq!(Override::new("\\i1").drawing_scale(), None);
  /// ```
  pub fn drawing_scale(&self) -> Option<u32> {
    let mut scale = None;
    for tag in self.tags() {
      if tag.name() == "p" {
        scale = Some(tag.args().trim().parse::<u32>().unwrap_or(0));
      }
    }
    scale
  }
}

impl fmt::Display for Override<'_> {
  /// Serializes the block back to ASS markup, restoring the braces.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// assert_eq!(Override::new("\\i1").to_string(), "{\\i1}");
  /// ```
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("{")?;
    f.write_str(self.raw)?;
    f.write_str("}")
  }
}

/// A single override tag inside an [`Override`] block, e.g. `\pos(10,20)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverrideTag<'a> {
  name: &'a str,
  args: &'a str,
  known: bool,
}

impl<'a> OverrideTag<'a> {
  /// Returns the tag name, without the leading backslash.
  ///
  /// For a recognized tag this is the longest matching known name, so `\i1`
  /// yields `"i"` and `\fscx200` yields `"fscx"`.  For an unrecognized tag it
  /// is the leading run of ASCII alphabetic characters, which may be empty.
  ///
  /// Spaces and tabs between the backslash and the name are skipped, as libass
  /// skips them, so `\ p1` is the `p` tag.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// let tags: Vec<_> = Override::new("\\fscx200\\zz9").tags().collect();
  /// assert_eq!(tags[0].name(), "fscx");
  /// assert_eq!(tags[1].name(), "zz");
  ///
  /// let tags: Vec<_> = Override::new("\\ p1").tags().collect();
  /// assert_eq!(tags[0].name(), "p");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn name(&self) -> &'a str {
    self.name
  }

  /// Returns the tag arguments: everything between the end of the name and
  /// the next tag in the block.
  ///
  /// Arguments are returned verbatim and are never interpreted.  A `\` inside
  /// a parenthesized argument list belongs to the enclosing tag, so
  /// `\t(0,500,\frz360)` is a single tag whose arguments are
  /// `"(0,500,\frz360)"`.  The list ends at the **first** `)`, as libass does
  /// — it does not track nesting.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// let tags: Vec<_> = Override::new("\\t(0,500,\\frz360)\\i1").tags().collect();
  /// assert_eq!(tags.len(), 2);
  /// assert_eq!(tags[0].name(), "t");
  /// assert_eq!(tags[0].args(), "(0,500,\\frz360)");
  /// assert_eq!(tags[1].name(), "i");
  /// assert_eq!(tags[1].args(), "1");
  ///
  /// // The inner `)` closes the argument list, so `\p0` is a following tag.
  /// let tags: Vec<_> = Override::new("\\t(0,500,\\clip(0,0,1,1)\\p0").tags().collect();
  /// assert_eq!(tags.len(), 2);
  /// assert_eq!(tags[1].name(), "p");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn args(&self) -> &'a str {
    self.args
  }

  /// Whether the name matched an override tag this crate recognizes.
  ///
  /// Unrecognized tags are still tokenized — an ASS renderer ignores them and
  /// so does the clean-text layer — but their name/argument split is a
  /// best-effort guess.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// let tags: Vec<_> = Override::new("\\i1\\zz9").tags().collect();
  /// assert!(tags[0].is_known());
  /// assert!(!tags[1].is_known());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn is_known(&self) -> bool {
    self.known
  }
}

impl fmt::Display for OverrideTag<'_> {
  /// Serializes the tag back to ASS markup, restoring the leading backslash.
  ///
  /// ```rust
  /// use fasrt::ass::text::Override;
  ///
  /// let tag = Override::new("\\pos(10,20)").tags().next().unwrap();
  /// assert_eq!(tag.to_string(), "\\pos(10,20)");
  /// ```
  /// This is the canonical form: any spaces between the backslash and the
  /// name are dropped, so `\ p1` is written back as `\p1`.  Use
  /// [`Override::as_str`] when the block's original bytes are needed.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("\\")?;
    f.write_str(self.name)?;
    f.write_str(self.args)
  }
}

/// A lazy iterator over the override tags of an [`Override`] block.
///
/// Created by [`Override::tags`].
#[derive(Debug, Clone)]
pub struct OverrideTags<'a> {
  rest: &'a str,
}

impl<'a> Iterator for OverrideTags<'a> {
  type Item = OverrideTag<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    // Anything before the first top-level `\` is a comment, per VSFilter.
    let start = find_tag_start(self.rest)?;
    let after = &self.rest[start + 1..];
    // libass skips spaces between the backslash and the tag name.
    let spaces = leading_spaces(after);
    let (name, known) = match_tag_name(&after[spaces..]);

    let args_start = start + 1 + spaces + name.len();
    let args_len = find_tag_start(&self.rest[args_start..]).unwrap_or(self.rest.len() - args_start);
    let args = &self.rest[args_start..args_start + args_len];

    self.rest = &self.rest[args_start + args_len..];
    Some(OverrideTag { name, args, known })
  }
}

/// Returns the number of leading space and tab bytes in `s`.
#[cfg_attr(not(tarpaulin), inline(always))]
fn leading_spaces(s: &str) -> usize {
  s.as_bytes()
    .iter()
    .take_while(|&&b| b == b' ' || b == b'\t')
    .count()
}

/// Returns the byte index of the first `\` that is not inside a parenthesized
/// argument list, or `None` if there is none.
///
/// A parenthesized argument list ends at the **first** `)`, which is what
/// libass does — it does not track nesting.  So in
/// `\t(0,500,\clip(0,0,10,10)\p0` the `\p0` is a following tag, not part of
/// the transform.  Unbalanced parentheses simply mean no further tag start is
/// found, which makes the remainder the current tag's arguments.
///
/// Only ASCII bytes (`(`, `)`, `\`) are compared, so every returned index is a
/// UTF-8 character boundary.
fn find_tag_start(s: &str) -> Option<usize> {
  let mut in_args = false;
  for (i, &b) in s.as_bytes().iter().enumerate() {
    match b {
      b'(' => in_args = true,
      b')' => in_args = false,
      b'\\' if !in_args => return Some(i),
      _ => {}
    }
  }
  None
}

/// Splits a tag name off the front of `after` (the text following a `\` and
/// any spaces).
///
/// Returns the matched name and whether it is a recognized tag.
fn match_tag_name(after: &str) -> (&str, bool) {
  let bytes = after.as_bytes();
  for name in KNOWN_TAGS {
    if bytes.starts_with(name.as_bytes()) {
      return (&after[..name.len()], true);
    }
  }
  let len = bytes.iter().take_while(|b| b.is_ascii_alphabetic()).count();
  (&after[..len], false)
}

// ── Raw logos lexer (private) ───────────────────────────────────────────────

/// Low-level token produced by the logos DFA.
///
/// The grammar is regular. A `{` opens an override block **only** when a `}`
/// follows it; the block then ends at that first `}`, because VSFilter and
/// libass do not nest braces. A `{` with no `}` after it, and a `}` outside a
/// block, are ordinary text. `\{` and `\}` are escapes for a literal brace.
#[derive(Debug, Clone, Logos)]
enum RawTextToken<'a> {
  /// A terminated override block, e.g. `{\i1}`.  Longer than the bare `{`
  /// token whenever a `}` exists, so logos prefers it.
  #[regex(r"\{[^}]*\}")]
  Override(&'a str),

  /// `\N` — a hard line break.
  #[token("\\N")]
  HardBreak,

  /// `\n` — a soft line break.
  #[token("\\n")]
  SoftBreak,

  /// `\h` — a non-breaking space.
  #[token("\\h")]
  HardSpace,

  /// `\{` or `\}` — an escape for a literal brace.
  #[token("\\{")]
  #[token("\\}")]
  EscapedBrace(&'a str),

  /// A `\` that begins no escape: it is literal text, and **only** the
  /// backslash is consumed.
  ///
  /// libass reads an unrecognized escape by emitting the backslash and
  /// advancing one byte, so the character after it is examined afresh.  That
  /// is what makes `\\N` a literal backslash followed by a line break rather
  /// than a literal `\\N`.
  #[token("\\")]
  LoneBackslash,

  /// A `{` that opens no block, because no `}` follows it.
  #[token("{")]
  LiteralBrace(&'a str),

  /// A run of literal text: anything that is neither `{` nor `\`.  A `}` is
  /// ordinary text and is included here.
  #[regex(r"[^{\\]+")]
  Text(&'a str),
}

/// The same grammar as [`RawTextToken`] minus the override-block rule, used
/// once the last `}` of the field is behind us.
///
/// Past that point no `{` can open a block, so the block rule could only ever
/// fail — and failing costs a scan to the end of the field, which would make
/// a field of many unmatched `{` quadratic.  Without the rule every pattern
/// here fails within two bytes, so tokenizing the remainder is linear.  A `{`
/// simply falls into the literal-text rule, since no `}` remains to exclude.
#[derive(Debug, Clone, Logos)]
enum TailToken<'a> {
  /// `\N` — a hard line break.
  #[token("\\N")]
  HardBreak,

  /// `\n` — a soft line break.
  #[token("\\n")]
  SoftBreak,

  /// `\h` — a non-breaking space.
  #[token("\\h")]
  HardSpace,

  /// `\{` or `\}` — an escape for a literal brace.
  #[token("\\{")]
  #[token("\\}")]
  EscapedBrace(&'a str),

  /// A `\` that begins no escape: literal text, consuming only the backslash.
  #[token("\\")]
  LoneBackslash,

  /// A run of literal text: anything that is not `\`.
  #[regex(r"[^\\]+")]
  Text(&'a str),
}

/// What either lexer produced, normalized so that both drive the same token
/// construction.
enum Lexeme<'a> {
  /// Literal text, subject to drawing mode.
  Text(&'a str),
  /// A hard line break, with the slice that produced it.
  HardBreak(&'a str),
  /// A soft line break, with the slice that produced it.
  SoftBreak(&'a str),
  /// A non-breaking space, with the slice that produced it.
  HardSpace(&'a str),
  /// A `\{` or `\}` escape, with the full two-character slice.
  EscapedBrace(&'a str),
  /// An override block, with the full brace-delimited slice.
  Override(&'a str),
}

impl<'a> RawTextToken<'a> {
  /// Normalizes this token, given the slice the lexer matched.
  fn into_lexeme(self, slice: &'a str) -> Lexeme<'a> {
    match self {
      Self::Text(run) | Self::LiteralBrace(run) => Lexeme::Text(run),
      Self::LoneBackslash => Lexeme::Text(slice),
      Self::HardBreak => Lexeme::HardBreak(slice),
      Self::SoftBreak => Lexeme::SoftBreak(slice),
      Self::HardSpace => Lexeme::HardSpace(slice),
      Self::EscapedBrace(escape) => Lexeme::EscapedBrace(escape),
      Self::Override(block) => Lexeme::Override(block),
    }
  }
}

impl<'a> TailToken<'a> {
  /// Normalizes this token, given the slice the lexer matched.
  fn into_lexeme(self, slice: &'a str) -> Lexeme<'a> {
    match self {
      Self::Text(run) => Lexeme::Text(run),
      Self::LoneBackslash => Lexeme::Text(slice),
      Self::HardBreak => Lexeme::HardBreak(slice),
      Self::SoftBreak => Lexeme::SoftBreak(slice),
      Self::HardSpace => Lexeme::HardSpace(slice),
      Self::EscapedBrace(escape) => Lexeme::EscapedBrace(escape),
    }
  }
}

/// Returns the byte index of the last `}` in `input`, if any.
#[cfg_attr(not(tarpaulin), inline(always))]
fn last_close_brace(input: &str) -> Option<usize> {
  let bytes = input.as_bytes();

  #[cfg(all(feature = "memchr", not(miri)))]
  {
    memchr::memrchr(b'}', bytes)
  }
  #[cfg(not(all(feature = "memchr", not(miri))))]
  {
    bytes.iter().rposition(|&b| b == b'}')
  }
}

/// A lazy, zero-copy parser for the `Text` field of an ASS/SSA event, backed
/// by a [`logos`] DFA.
///
/// Yields [`TextToken`]s.  The parser **never** allocates and is available on
/// every feature tier, including `no_std` without `alloc`.  It is usable
/// standalone: an embedded Matroska `S_TEXT/ASS` packet can be tokenized
/// directly, with no surrounding document.
///
/// # Braces
///
/// A `{` opens an override block **only** when a `}` follows it somewhere in
/// the field; the block then ends at that first `}`, since braces do not
/// nest.  An unmatched `{`, and any `}` outside a block, are ordinary text —
/// libass does the same, and the alternative would silently delete text a
/// renderer shows.  `\{` and `\}` are escapes yielding a literal brace.
///
/// # Drawing mode
///
/// The parser tracks vector-drawing mode across tokens.  A `{\p<n>}` block
/// with `n > 0` switches it on and `{\p0}` switches it off; while it is on,
/// literal runs are reported as [`TextToken::Drawing`] instead of
/// [`TextToken::Text`] so callers can skip geometry without interpreting it.
/// Drawing mode does not survive the end of the field.
///
/// # Example
///
/// ```rust
/// use fasrt::ass::text::{TextParser, TextToken};
///
/// let tokens: Vec<_> = TextParser::new("{\\i1}Hello\\NWorld").collect();
/// assert!(matches!(tokens[0], TextToken::Override(_)));
/// assert_eq!(tokens[1], TextToken::Text("Hello"));
/// assert_eq!(tokens[2], TextToken::HardBreak);
/// assert_eq!(tokens[3], TextToken::Text("World"));
/// ```
#[derive(Clone)]
pub struct TextParser<'a> {
  input: &'a str,
  /// Lexes while an override block is still possible.
  head: logos::Lexer<'a, RawTextToken<'a>>,
  /// Lexes the remainder once no `}` is left; see [`TailToken`].
  tail: Option<logos::Lexer<'a, TailToken<'a>>>,
  /// Byte index of the field's last `}`, past which no block can form.
  last_close: Option<usize>,
  /// Byte offset the head lexer has consumed up to.
  head_end: usize,
  drawing: bool,
}

impl<'a> TextParser<'a> {
  /// Create a new parser for the given raw event text.
  ///
  /// ```rust
  /// use fasrt::ass::text::{TextParser, TextToken};
  ///
  /// let mut parser = TextParser::new("plain");
  /// assert_eq!(parser.next(), Some(TextToken::Text("plain")));
  /// assert_eq!(parser.next(), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn new(input: &'a str) -> Self {
    Self {
      input,
      head: RawTextToken::lexer(input),
      tail: None,
      last_close: last_close_brace(input),
      head_end: 0,
      drawing: false,
    }
  }

  /// Whether vector-drawing mode is currently active.
  ///
  /// ```rust
  /// use fasrt::ass::text::TextParser;
  ///
  /// let mut parser = TextParser::new("{\\p1}m 0 0");
  /// assert!(!parser.is_drawing());
  /// let _ = parser.next();
  /// assert!(parser.is_drawing());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn is_drawing(&self) -> bool {
    self.drawing
  }

  /// Classifies a literal run according to the current drawing mode.
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn literal(&self, run: &'a str) -> TextToken<'a> {
    if self.drawing {
      TextToken::Drawing(run)
    } else {
      TextToken::Text(run)
    }
  }

  /// Whether an override block can still be found from the head lexer's
  /// current position.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn blocks_possible(&self) -> bool {
    matches!(self.last_close, Some(last) if self.head_end <= last)
  }

  /// Turns a normalized lexeme into a public token, applying drawing mode.
  fn emit(&mut self, lexeme: Lexeme<'a>) -> TextToken<'a> {
    match lexeme {
      Lexeme::Text(run) => self.literal(run),
      Lexeme::HardBreak(slice) if self.drawing => TextToken::Drawing(slice),
      Lexeme::SoftBreak(slice) if self.drawing => TextToken::Drawing(slice),
      Lexeme::HardSpace(slice) if self.drawing => TextToken::Drawing(slice),
      Lexeme::EscapedBrace(slice) if self.drawing => TextToken::Drawing(slice),
      Lexeme::HardBreak(_) => TextToken::HardBreak,
      Lexeme::SoftBreak(_) => TextToken::SoftBreak,
      Lexeme::HardSpace(_) => TextToken::HardSpace,
      // Strip the backslash; the brace itself is what renders.
      Lexeme::EscapedBrace(slice) => TextToken::EscapedBrace(&slice[1..]),
      Lexeme::Override(slice) => {
        // Strip the braces; the regex guarantees both are present.
        let block = Override::new(&slice[1..slice.len() - 1]);
        if let Some(scale) = block.drawing_scale() {
          self.drawing = scale > 0;
        }
        TextToken::Override(block)
      }
    }
  }
}

impl<'a> Iterator for TextParser<'a> {
  type Item = TextToken<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      // Hand over to the block-free lexer as soon as the field's last `}` is
      // behind us, so the block rule never runs where it could only fail.
      if self.tail.is_none() && !self.blocks_possible() {
        self.tail = Some(TailToken::lexer(&self.input[self.head_end..]));
      }

      let lexeme = match self.tail.as_mut() {
        Some(tail) => {
          let token = tail.next()?;
          let slice = tail.slice();
          match token {
            Ok(token) => token.into_lexeme(slice),
            // Every byte sequence is covered by a rule, so this is
            // unreachable; skipping keeps the iterator total.
            Err(()) => continue,
          }
        }
        None => {
          let token = self.head.next()?;
          let slice = self.head.slice();
          self.head_end = self.head.span().end;
          match token {
            Ok(token) => token.into_lexeme(slice),
            Err(()) => continue,
          }
        }
      };

      return Some(self.emit(lexeme));
    }
  }
}

/// A piece of cleaned text produced by [`PlainText::segments`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
  /// A run of literal text, borrowed from the input.
  Text(&'a str),
  /// A hard line break (`\N`).
  HardBreak,
  /// A soft line break (`\n`).
  SoftBreak,
  /// A non-breaking space (`\h`).
  HardSpace,
}

/// A lazy iterator over the cleaned segments of an event's text.
///
/// Created by [`PlainText::segments`].  Override blocks and drawing payloads
/// are dropped; everything else is borrowed from the input, so this iterator
/// never allocates and is available on every feature tier.
#[derive(Clone)]
pub struct Segments<'a> {
  parser: TextParser<'a>,
}

impl<'a> Iterator for Segments<'a> {
  type Item = Segment<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      return Some(match self.parser.next()? {
        TextToken::Text(run) | TextToken::EscapedBrace(run) => Segment::Text(run),
        TextToken::HardBreak => Segment::HardBreak,
        TextToken::SoftBreak => Segment::SoftBreak,
        TextToken::HardSpace => Segment::HardSpace,
        TextToken::Override(_) | TextToken::Drawing(_) => continue,
      });
    }
  }
}

/// The clean-text layer: an event's `Text` field with rendering markup
/// removed, normalized lazily.
///
/// This is the ASS/SSA counterpart of [`crate::vtt::cue::CueStr`].
///
/// # Zero-copy guarantee
///
/// Construction never allocates.  A field that contains neither `{` nor `\`
/// needs no cleaning at all, and [`normalize`] then returns the original
/// borrowed slice — the common case for dialogue lines.  Otherwise the cleaned
/// text is computed once and cached behind a [`core::cell::OnceCell`]
/// (requires `alloc` or `std`).
///
/// On `no_std` without `alloc`, [`normalize`] returns the raw text; use
/// [`segments`] there, which is allocation-free on every tier.
///
/// [`normalize`]: PlainText::normalize
/// [`segments`]: PlainText::segments
pub struct PlainText<'a> {
  raw: &'a str,
  requires_normalization: bool,
  #[cfg(any(feature = "alloc", feature = "std"))]
  normalized: core::cell::OnceCell<std::string::String>,
}

impl<'a> PlainText<'a> {
  /// Create a `PlainText` for the given raw `Text` field, deciding whether
  /// cleaning is needed.
  ///
  /// ```rust
  /// use fasrt::ass::text::PlainText;
  ///
  /// assert!(!PlainText::new("plain line").requires_normalization());
  /// assert!(PlainText::new("{\\i1}styled").requires_normalization());
  /// assert!(PlainText::new("two\\Nlines").requires_normalization());
  /// ```
  pub fn new(raw: &'a str) -> Self {
    let bytes = raw.as_bytes();

    #[cfg(all(feature = "memchr", not(miri)))]
    let dirty = memchr::memchr2(b'{', b'\\', bytes).is_some();
    #[cfg(not(all(feature = "memchr", not(miri))))]
    let dirty = bytes.iter().any(|&b| b == b'{' || b == b'\\');

    if dirty {
      Self::needs_normalization(raw)
    } else {
      Self::borrowed(raw)
    }
  }

  /// Create a `PlainText` that does **not** need normalization.
  ///
  /// ```rust
  /// use fasrt::ass::text::PlainText;
  ///
  /// let text = PlainText::borrowed("hello");
  /// assert_eq!(text.as_raw(), "hello");
  /// assert!(!text.requires_normalization());
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn borrowed(raw: &'a str) -> Self {
    Self {
      raw,
      requires_normalization: false,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: core::cell::OnceCell::new(),
    }
  }

  /// Create a `PlainText` that **requires** normalization.
  ///
  /// ```rust
  /// use fasrt::ass::text::PlainText;
  ///
  /// let text = PlainText::needs_normalization("{\\i1}hi");
  /// assert!(text.requires_normalization());
  /// assert_eq!(text.as_raw(), "{\\i1}hi");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn needs_normalization(raw: &'a str) -> Self {
    Self {
      raw,
      requires_normalization: true,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: core::cell::OnceCell::new(),
    }
  }

  /// Returns the raw text, exactly as it appeared in the event.
  ///
  /// ```rust
  /// use fasrt::ass::text::PlainText;
  ///
  /// assert_eq!(PlainText::new("{\\i1}hi").as_raw(), "{\\i1}hi");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_raw(&self) -> &'a str {
    self.raw
  }

  /// Whether this text may contain markup that cleaning would remove or
  /// rewrite.
  ///
  /// This is a conservative test on the presence of `{` or `\`: it is never
  /// false for text that does need cleaning, but it can be true for text that
  /// turns out not to — a `{` that opens no block, say — in which case
  /// [`normalize`](Self::normalize) simply reproduces the input.
  ///
  /// ```rust
  /// use fasrt::ass::text::PlainText;
  ///
  /// assert!(!PlainText::new("hello").requires_normalization());
  /// assert!(PlainText::new("hel{\\b1}lo").requires_normalization());
  ///
  /// // Conservative: no block is opened here, but `{` is still flagged.
  /// assert!(PlainText::new("a{b").requires_normalization());
  /// assert_eq!(PlainText::new("a{b").normalize(), "a{b");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn requires_normalization(&self) -> bool {
    self.requires_normalization
  }

  /// Returns a lazy, allocation-free iterator over the cleaned segments.
  ///
  /// Use this instead of [`normalize`](Self::normalize) to apply your own
  /// policy for line breaks and hard spaces, or on `no_std` without `alloc`.
  ///
  /// ```rust
  /// use fasrt::ass::text::{PlainText, Segment};
  ///
  /// let text = PlainText::new("{\\i1}Hi\\Nthere");
  /// let segments: Vec<_> = text.segments().collect();
  /// assert_eq!(
  ///   segments,
  ///   [Segment::Text("Hi"), Segment::HardBreak, Segment::Text("there")],
  /// );
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn segments(&self) -> Segments<'a> {
    Segments {
      parser: TextParser::new(self.raw),
    }
  }

  /// Returns the cleaned text.
  ///
  /// Override blocks and drawing-mode payloads are dropped, `\N` and `\n`
  /// become U+000A, `\h` becomes U+00A0 (a no-break space, which is how
  /// VSFilter/libass render it), and `\{` / `\}` become the brace alone.
  /// Callers wanting a different policy should use
  /// [`segments`](Self::segments).
  ///
  /// A `{` that opens no block is literal text and is kept, so cleaning never
  /// deletes text that a renderer would show.
  ///
  /// When no cleaning is needed the borrowed input is returned as-is and
  /// nothing is allocated; otherwise the cleaned text is computed once and
  /// cached.
  ///
  /// On `no_std` without `alloc` this always returns the raw text.
  ///
  /// ```rust
  /// use fasrt::ass::text::PlainText;
  ///
  /// let plain = PlainText::new("no markup here");
  /// assert_eq!(plain.normalize(), "no markup here");
  ///
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// let styled = PlainText::new("{\\i1}Hello{\\i0}\\NWorld");
  /// assert_eq!(styled.normalize(), "Hello\nWorld");
  ///
  /// // A drawing payload is skipped, never interpreted as geometry.
  /// let drawing = PlainText::new("{\\p1}m 0 0 l 10 0{\\p0}caption");
  /// assert_eq!(drawing.normalize(), "caption");
  /// # }
  /// ```
  pub fn normalize(&self) -> &str {
    if !self.requires_normalization {
      return self.raw;
    }

    #[cfg(any(feature = "alloc", feature = "std"))]
    {
      self.normalized.get_or_init(|| self.clean())
    }
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    {
      self.raw
    }
  }

  /// Builds the cleaned text from the segment stream.
  #[cfg(any(feature = "alloc", feature = "std"))]
  fn clean(&self) -> std::string::String {
    let mut out = std::string::String::with_capacity(self.raw.len());
    for segment in self.segments() {
      match segment {
        Segment::Text(run) => out.push_str(run),
        Segment::HardBreak | Segment::SoftBreak => out.push('\n'),
        Segment::HardSpace => out.push('\u{00A0}'),
      }
    }
    out
  }
}

impl Clone for PlainText<'_> {
  fn clone(&self) -> Self {
    Self {
      raw: self.raw,
      requires_normalization: self.requires_normalization,
      #[cfg(any(feature = "alloc", feature = "std"))]
      normalized: self.normalized.clone(),
    }
  }
}

impl fmt::Debug for PlainText<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("PlainText")
      .field("raw", &self.raw)
      .field("requires_normalization", &self.requires_normalization)
      .finish()
  }
}

impl PartialEq for PlainText<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.raw == other.raw && self.requires_normalization == other.requires_normalization
  }
}

impl Eq for PlainText<'_> {}

impl fmt::Display for PlainText<'_> {
  /// Writes the cleaned text.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::ass::text::PlainText;
  ///
  /// assert_eq!(PlainText::new("{\\b1}bold").to_string(), "bold");
  /// # }
  /// ```
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    #[cfg(any(feature = "alloc", feature = "std"))]
    {
      f.write_str(self.normalize())
    }

    #[cfg(not(any(feature = "alloc", feature = "std")))]
    {
      for segment in self.segments() {
        match segment {
          Segment::Text(run) => f.write_str(run)?,
          Segment::HardBreak | Segment::SoftBreak => f.write_str("\n")?,
          Segment::HardSpace => f.write_str("\u{00A0}")?,
        }
      }
      Ok(())
    }
  }
}
