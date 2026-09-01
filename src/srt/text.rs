//! Cue-body text parsing: SubRip's markup vocabulary and clean-text extraction.
//!
//! This is the SubRip counterpart of [`crate::ass::text`], and it has the same
//! two-layer shape:
//!
//! 1. [`TextParser`] — a lazy, zero-allocation [`logos`] DFA token stream over
//!    a cue body.  Usable on every feature tier, including `no_std` without
//!    `alloc`.
//! 2. [`PlainText`] — the clean-text layer.  Markup is dropped, `<br>` becomes
//!    a line break, and normalization is **deferred**: a body that contains no
//!    `<` and no `{` never allocates and [`PlainText::normalize`] returns the
//!    borrowed input unchanged.
//!
//! Both layers take a plain `&str`, so an embedded Matroska `S_TEXT/UTF8`
//! track — which arrives one cue **body** per packet, with no index line and
//! no timing line — can be read without building a whole-file document.  The
//! container's timestamps are authoritative there; nothing in this module
//! looks at timing.
//!
//! # The dialect
//!
//! SubRip has no specification.  Its markup is whatever the players that read
//! it agree to accept, so every rule below is taken from a player rather than
//! from a document.  The three surveyed are FFmpeg's `subrip` decoder
//! (`libavcodec/htmlsubtitles.c`, reached from `srtdec.c`; this is also what
//! mpv and everything else built on libavcodec uses), VLC's subtitle decoder
//! (`modules/codec/subsdec.c`), and Aegisub's SRT reader
//! (`src/subtitle_format_srt.cpp`).
//!
//! | rule | this module | FFmpeg | VLC | Aegisub |
//! |---|---|---|---|---|
//! | `<b>` `<i>` `<u>` `<s>` | markup | markup | markup | markup |
//! | `<font …>` | markup, attributes readable | markup | markup | markup |
//! | `<br>` | line break | line break | line break | *bold* (regex quirk) |
//! | tag names | ASCII case-insensitive | case-insensitive | case-insensitive | case-insensitive |
//! | any other `<` | literal text | mostly literal | mostly literal | literal |
//! | unclosed tag | dropped, text kept | state marker | state marker | state marker |
//! | `&amp;` | literal text | literal text | literal text | literal text |
//! | `{\…}`, `{Y:…}` | dropped | dropped | dropped | kept |
//!
//! The one rule worth stating twice, because it is the one a WebVTT tokenizer
//! gets wrong on SubRip: **a `<` that does not begin a tag is literal text,
//! and so is everything after it.**  `I <3 this` and the Japanese narration
//! convention `<セリフ` are ordinary subtitle lines, and all three players
//! show them.  A `<` only opens markup when what follows is one of the names
//! above; anything else is text.
//!
//! # Not HTML
//!
//! SubRip borrows HTML's *tag* syntax and nothing else.  None of the three
//! players decodes character references, so `&amp;` is the five characters
//! `&amp;` and `&lrm;` is the five characters `&lrm;` — this module keeps
//! them.  That is the one place it disagrees with [`crate::vtt::cue`], whose
//! entity decoding is correct for WebVTT and wrong here.  NULL bytes are left
//! alone for the same reason; WebVTT's U+FFFD substitution is a WebVTT rule.
//!
//! # No voice, no annotation
//!
//! SubRip has no speaker vocabulary: there is no counterpart to WebVTT's
//! `<v Speaker>` or to the ASS/SSA `Name` column, and no player recognizes
//! one.  So this module exposes none, and `<v Roger>` in a body is literal
//! text like any other unrecognized tag.  A speaker in a SubRip file is
//! written in the text itself — `- ` dashes, `[NAME]:` prefixes — and reading
//! that is the caller's business, not the format's.
//!
//! # Depth
//!
//! This module builds no tree.  Both layers are flat iterators over a token
//! stream and use O(1) stack regardless of how the input nests, so the
//! stack-overflow class that bounds [`crate::vtt::cue::CueText`] at
//! [`DEFAULT_MAX_DEPTH`] cannot arise here and there is no depth knob to set.
//! A body of a hundred thousand nested `<i>` is cleaned in one pass.
//!
//! Nesting is still meaningful to a *renderer* — `<font>` is the one SubRip
//! tag whose state is not binary, and FFmpeg keeps a stack of 16 for it,
//! ignoring anything deeper.  A caller that models font state should bound it
//! the same way; the clean-text layer has no state to bound because it drops
//! font styling outright.
//!
//! [`DEFAULT_MAX_DEPTH`]: crate::vtt::cue::DEFAULT_MAX_DEPTH
//!
//! # Non-goals
//!
//! Tag *semantics* are out of scope: this module reports that a `<font>` tag
//! carried `color="#ffff00"` but never turns that into a colour.  Coordinate
//! suffixes (`X1:… X2:… Y1:… Y2:…`) are likewise absent, because they belong
//! to the timing line rather than the body — FFmpeg carries them out of band
//! as packet side data and never lets them reach the text.

use derive_more::{Display, IsVariant};
use logos::Logos;

use core::fmt;

/// A markup tag SubRip's readers recognize.
///
/// Names are matched **ASCII case-insensitively**, so `<I>` and `<i>` are the
/// same tag: FFmpeg lowercases with `av_tolower`, VLC compares with
/// `strcasecmp`, and Aegisub matches with a case-insensitive regex.
///
/// `<br>` is deliberately absent.  It carries no state a caller could open or
/// close, so it is reported as [`TextToken::LineBreak`] rather than as a tag.
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
  /// `<s>` — strikeout.
  #[display("s")]
  Strikeout,
  /// `<font>` — font styling, carrying attributes.
  #[display("font")]
  Font,
}

/// A start tag, e.g. `<i>` or `<font color="#ffff00" size=14>`.
///
/// The tag name is normalized to a [`Tag`]; the attributes are kept as the
/// verbatim slice between the name and the `>`, and are never interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StartTag<'a> {
  tag: Tag,
  attributes: &'a str,
}

impl<'a> StartTag<'a> {
  /// Create a start tag from its name and its verbatim attribute text.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// let tag = StartTag::new(Tag::Font, "color=\"#ffff00\"");
  /// assert_eq!(tag.tag(), Tag::Font);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(tag: Tag, attributes: &'a str) -> Self {
    Self { tag, attributes }
  }

  /// Returns the tag name.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// assert_eq!(StartTag::new(Tag::Italic, "").tag(), Tag::Italic);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn tag(&self) -> Tag {
    self.tag
  }

  /// Returns everything between the tag name and the `>`, verbatim, with
  /// leading whitespace trimmed.
  ///
  /// Empty for a tag that carried none.
  ///
  /// ```rust
  /// use fasrt::srt::text::{TextParser, TextToken};
  ///
  /// let tokens: Vec<_> = TextParser::new("<font color=\"#ffff00\" size=14>hi").collect();
  /// let TextToken::StartTag(tag) = &tokens[0] else { panic!() };
  /// assert_eq!(tag.attributes(), "color=\"#ffff00\" size=14");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn attributes(&self) -> &'a str {
    self.attributes
  }

  /// Returns a lazy iterator over the individual attributes.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// let tag = StartTag::new(Tag::Font, "color=\"#ffff00\" size=14");
  /// let names: Vec<_> = tag.attrs().map(|a| a.name()).collect();
  /// assert_eq!(names, ["color", "size"]);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn attrs(&self) -> Attributes<'a> {
    Attributes {
      rest: self.attributes,
    }
  }
}

impl fmt::Display for StartTag<'_> {
  /// Serializes the tag back to SubRip markup.
  ///
  /// This is the canonical form: the name is written in lower case and a
  /// single space separates it from the attributes, so `<FONT   SIZE=1>` is
  /// written back as `<font SIZE=1>`.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// assert_eq!(StartTag::new(Tag::Italic, "").to_string(), "<i>");
  /// assert_eq!(StartTag::new(Tag::Font, "size=14").to_string(), "<font size=14>");
  /// ```
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "<{}", self.tag)?;
    if !self.attributes.is_empty() {
      write!(f, " {}", self.attributes)?;
    }
    f.write_str(">")
  }
}

/// A single attribute of a [`StartTag`], e.g. `color="#ffff00"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Attribute<'a> {
  name: &'a str,
  value: Option<&'a str>,
}

impl<'a> Attribute<'a> {
  /// Returns the attribute name, verbatim — it is **not** case-folded.
  ///
  /// Use [`is_known`](Self::is_known) to test it against the recognized set
  /// without minding case.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// let attr = StartTag::new(Tag::Font, "Color=red").attrs().next().unwrap();
  /// assert_eq!(attr.name(), "Color");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn name(&self) -> &'a str {
    self.name
  }

  /// Returns the attribute value with surrounding quotes stripped, or `None`
  /// when the attribute carried no `=` at all.
  ///
  /// `None` and `Some("")` are different: `<font color>` has no value, while
  /// `<font color="">` has an empty one.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// let attrs: Vec<_> = StartTag::new(Tag::Font, "color=\"red\" size=14 face").attrs().collect();
  /// assert_eq!(attrs[0].value(), Some("red"));
  /// assert_eq!(attrs[1].value(), Some("14"));
  /// assert_eq!(attrs[2].value(), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn value(&self) -> Option<&'a str> {
    self.value
  }

  /// Whether the name is one of the three `<font>` attributes every surveyed
  /// player reads: `color`, `size` and `face`.
  ///
  /// The comparison is ASCII case-insensitive.  Other attributes are still
  /// reported — VLC reads six more, including `back-color` and `alpha` — but
  /// only these three are common ground, and none of them survives into the
  /// clean text, which drops styling entirely.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// let attrs: Vec<_> = StartTag::new(Tag::Font, "SIZE=1 back-color=red").attrs().collect();
  /// assert!(attrs[0].is_known());
  /// assert!(!attrs[1].is_known());
  /// ```
  pub fn is_known(&self) -> bool {
    ["color", "size", "face"]
      .iter()
      .any(|known| self.name.eq_ignore_ascii_case(known))
  }
}

impl fmt::Display for Attribute<'_> {
  /// Serializes the attribute, quoting the value only when it must be.
  ///
  /// A value containing whitespace is quoted, with a single quote chosen when
  /// the value itself contains a double one — the parser ends a quoted value
  /// at its own quote, so writing the other delimiter is what keeps the pair
  /// a round trip.
  ///
  /// ```rust
  /// use fasrt::srt::text::{StartTag, Tag};
  ///
  /// let attrs: Vec<_> = StartTag::new(Tag::Font, "size=14 face=\"Comic Sans\"").attrs().collect();
  /// assert_eq!(attrs[0].to_string(), "size=14");
  /// assert_eq!(attrs[1].to_string(), "face=\"Comic Sans\"");
  ///
  /// let quoted: Vec<_> = StartTag::new(Tag::Font, "face='a \"b\"'").attrs().collect();
  /// assert_eq!(quoted[0].value(), Some("a \"b\""));
  /// assert_eq!(quoted[0].to_string(), "face='a \"b\"'");
  /// ```
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.name)?;
    match self.value {
      None => Ok(()),
      Some(value) if value.as_bytes().iter().any(u8::is_ascii_whitespace) => {
        let quote = if value.contains('"') { '\'' } else { '"' };
        write!(f, "={quote}{value}{quote}")
      }
      Some(value) => write!(f, "={value}"),
    }
  }
}

/// A lazy iterator over the attributes of a [`StartTag`].
///
/// Created by [`StartTag::attrs`].
#[derive(Debug, Clone)]
pub struct Attributes<'a> {
  rest: &'a str,
}

impl<'a> Iterator for Attributes<'a> {
  type Item = Attribute<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    let rest = self.rest.trim_start_matches([' ', '\t']);
    if rest.is_empty() {
      self.rest = rest;
      return None;
    }

    // A name runs until `=`, whitespace, or the end.  Hyphens are part of it:
    // VLC's `outline-color` and `back-color` are real attribute names.
    let name_len = rest
      .as_bytes()
      .iter()
      .take_while(|&&b| !matches!(b, b'=' | b' ' | b'\t'))
      .count();
    let name = &rest[..name_len];
    let after = rest[name_len..].trim_start_matches([' ', '\t']);

    let Some(after) = after.strip_prefix('=') else {
      // No `=`: a bare name, and the next attribute starts after it.
      self.rest = after;
      return Some(Attribute { name, value: None });
    };

    let after = after.trim_start_matches([' ', '\t']);
    let (value, rest) = match after.as_bytes().first() {
      // A quoted value ends at the matching quote, or at the end of the
      // attribute text when the quote was never closed.
      Some(&quote @ (b'"' | b'\'')) => match after[1..].find(quote as char) {
        Some(end) => (&after[1..1 + end], &after[end + 2..]),
        None => (&after[1..], ""),
      },
      // A bare value ends at the next space, as it does in every surveyed
      // player.
      _ => {
        let len = after
          .as_bytes()
          .iter()
          .take_while(|&&b| !matches!(b, b' ' | b'\t'))
          .count();
        (&after[..len], &after[len..])
      }
    };

    self.rest = rest;
    Some(Attribute {
      name,
      value: Some(value),
    })
  }
}

/// An inline style code borrowed from SSA (`{\…}`) or MicroDVD (`{Y:…}`),
/// e.g. `{\an8}` or `{Y:i}`.
///
/// The stored slice is the code's *content*, with the surrounding braces
/// stripped.  These are not SubRip markup — they are what a converter left
/// behind — but FFmpeg and VLC both strip them from the text they show, so
/// this module reports them separately from the text rather than as part of
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InlineCode<'a> {
  raw: &'a str,
}

impl<'a> InlineCode<'a> {
  /// Create an inline code from its content, without the braces.
  ///
  /// ```rust
  /// use fasrt::srt::text::InlineCode;
  ///
  /// assert_eq!(InlineCode::new("\\an8").as_str(), "\\an8");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(raw: &'a str) -> Self {
    Self { raw }
  }

  /// Returns the code's content, without the surrounding braces.
  ///
  /// ```rust
  /// use fasrt::srt::text::{TextParser, TextToken};
  ///
  /// let tokens: Vec<_> = TextParser::new("{\\an8}top").collect();
  /// let TextToken::InlineCode(code) = &tokens[0] else { panic!() };
  /// assert_eq!(code.as_str(), "\\an8");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'a str {
    self.raw
  }
}

impl fmt::Display for InlineCode<'_> {
  /// Serializes the code back to markup, restoring the braces.
  ///
  /// ```rust
  /// use fasrt::srt::text::InlineCode;
  ///
  /// assert_eq!(InlineCode::new("Y:i").to_string(), "{Y:i}");
  /// ```
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{{{}}}", self.raw)
  }
}

/// A run of literal text, a tag, a line break, or an inline style code.
///
/// Produced by [`TextParser`].  Every payload borrows directly from the
/// input, so the token stream never allocates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextToken<'a> {
  /// A run of literal text.
  ///
  /// A `<` that opens no tag, and a `{` that opens no inline code, are
  /// literal text and are reported here **on their own**: the character after
  /// them is then examined afresh, which is what every surveyed player does
  /// and what keeps `I <3 this` intact.  So a run of literal text may arrive
  /// as several consecutive `Text` tokens.
  Text(&'a str),
  /// A start tag, e.g. `<i>` or `<font size=14>`.
  StartTag(StartTag<'a>),
  /// An end tag, e.g. `</i>`.
  ///
  /// End tags are reported wherever they appear.  SubRip's readers treat
  /// these tags as binary state markers rather than as a tree — FFmpeg says
  /// so outright, so that `<b> foo <i> bar </b> bla </i>` does not break —
  /// which means an end tag with no start tag, and a start tag with no end
  /// tag, are both ordinary and neither loses text.
  EndTag(Tag),
  /// A `<br>`, `<br/>` or `</br>` line break.
  LineBreak,
  /// An SSA or MicroDVD inline style code, e.g. `{\an8}` or `{Y:i}`.
  InlineCode(InlineCode<'a>),
}

// ── Raw logos lexer (private) ───────────────────────────────────────────────

/// Low-level token produced by the logos DFA.
///
/// The grammar is regular, and every rule that scans forward is bounded by a
/// character class that excludes its own opener: a tag body cannot contain
/// `<` or `>`, and an inline code cannot contain `{` or `}`.  So a rule that
/// fails gives up at the next opener rather than at the end of the input, and
/// tokenizing a body of many unterminated `<` or `{` stays linear.
///
/// Tag names are spelled out in both cases rather than folded, because the
/// DFA matches bytes.
#[derive(Debug, Clone, Logos)]
enum RawTextToken<'a> {
  // ── start tags ────────────────────────────────────────────────────────
  #[regex(r"<[bB]>|<[bB][ \t][^<>]*>")]
  StartBold(&'a str),
  #[regex(r"<[iI]>|<[iI][ \t][^<>]*>")]
  StartItalic(&'a str),
  #[regex(r"<[uU]>|<[uU][ \t][^<>]*>")]
  StartUnderline(&'a str),
  #[regex(r"<[sS]>|<[sS][ \t][^<>]*>")]
  StartStrikeout(&'a str),
  #[regex(r"<[fF][oO][nN][tT]>|<[fF][oO][nN][tT][ \t][^<>]*>")]
  StartFont(&'a str),

  // ── end tags ──────────────────────────────────────────────────────────
  #[regex(r"</[bB]>|</[bB][ \t][^<>]*>")]
  EndBold,
  #[regex(r"</[iI]>|</[iI][ \t][^<>]*>")]
  EndItalic,
  #[regex(r"</[uU]>|</[uU][ \t][^<>]*>")]
  EndUnderline,
  #[regex(r"</[sS]>|</[sS][ \t][^<>]*>")]
  EndStrikeout,
  #[regex(r"</[fF][oO][nN][tT]>|</[fF][oO][nN][tT][ \t][^<>]*>")]
  EndFont,

  /// `<br>`, `<br/>` or `</br>`.  FFmpeg reaches its `br` branch whether or
  /// not the tag was a closing one, so all three forms are one break.
  #[regex(r"</?[bB][rR]/?>|</?[bB][rR][ \t][^<>]*>")]
  LineBreak,

  // ── inline style codes left behind by a converter ─────────────────────
  /// An SSA override block, `{\…}`.
  #[regex(r"\{\\[^{}]*\}")]
  SsaCode(&'a str),
  /// A MicroDVD style, `{Y:…}`.  The letter set is FFmpeg's.
  #[regex(r"\{[CcFfoPSsYy]:[^{}]*\}")]
  MicroDvdCode(&'a str),

  // ── literal text ──────────────────────────────────────────────────────
  /// A run of text containing neither `<` nor `{`.
  #[regex(r"[^<{]+")]
  Text(&'a str),
  /// A `<` that opens no tag: literal text, consuming only the `<`.
  #[token("<")]
  LiteralLt(&'a str),
  /// A `{` that opens no inline code: literal text, consuming only the `{`.
  #[token("{")]
  LiteralBrace(&'a str),
}

/// Splits a start tag's slice into its [`Tag`] and its attribute text.
///
/// `slice` is the whole `<name…>` match and `name_len` the byte length of the
/// name, so the attributes are what lies between the name and the `>`.
#[cfg_attr(not(tarpaulin), inline(always))]
fn start_tag<'a>(tag: Tag, slice: &'a str, name_len: usize) -> StartTag<'a> {
  let attributes = slice[1 + name_len..slice.len() - 1].trim_start_matches([' ', '\t']);
  StartTag { tag, attributes }
}

/// A lazy, zero-copy parser for a SubRip cue body, backed by a [`logos`] DFA.
///
/// Yields [`TextToken`]s.  The parser **never** allocates and is available on
/// every feature tier, including `no_std` without `alloc`.  It is usable
/// standalone: an embedded Matroska `S_TEXT/UTF8` packet can be tokenized
/// directly, with no surrounding document.
///
/// # Example
///
/// ```rust
/// use fasrt::srt::text::{Tag, TextParser, TextToken};
///
/// let tokens: Vec<_> = TextParser::new("<i>Hello</i> world").collect();
/// assert!(matches!(tokens[0], TextToken::StartTag(t) if t.tag() == Tag::Italic));
/// assert_eq!(tokens[1], TextToken::Text("Hello"));
/// assert_eq!(tokens[2], TextToken::EndTag(Tag::Italic));
/// assert_eq!(tokens[3], TextToken::Text(" world"));
/// ```
///
/// A `<` that begins no tag is text, and so is the rest of the line:
///
/// ```rust
/// use fasrt::srt::text::{TextParser, TextToken};
///
/// let tokens: Vec<_> = TextParser::new("I <3 this").collect();
/// assert_eq!(tokens, [
///   TextToken::Text("I "),
///   TextToken::Text("<"),
///   TextToken::Text("3 this"),
/// ]);
/// ```
#[derive(Clone)]
pub struct TextParser<'a> {
  lexer: logos::Lexer<'a, RawTextToken<'a>>,
}

impl<'a> TextParser<'a> {
  /// Create a new parser for the given raw cue body.
  ///
  /// ```rust
  /// use fasrt::srt::text::{TextParser, TextToken};
  ///
  /// let mut parser = TextParser::new("plain");
  /// assert_eq!(parser.next(), Some(TextToken::Text("plain")));
  /// assert_eq!(parser.next(), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn new(input: &'a str) -> Self {
    Self {
      lexer: RawTextToken::lexer(input),
    }
  }
}

impl<'a> Iterator for TextParser<'a> {
  type Item = TextToken<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      return Some(match self.lexer.next()? {
        Ok(RawTextToken::StartBold(s)) => TextToken::StartTag(start_tag(Tag::Bold, s, 1)),
        Ok(RawTextToken::StartItalic(s)) => TextToken::StartTag(start_tag(Tag::Italic, s, 1)),
        Ok(RawTextToken::StartUnderline(s)) => TextToken::StartTag(start_tag(Tag::Underline, s, 1)),
        Ok(RawTextToken::StartStrikeout(s)) => TextToken::StartTag(start_tag(Tag::Strikeout, s, 1)),
        Ok(RawTextToken::StartFont(s)) => TextToken::StartTag(start_tag(Tag::Font, s, 4)),

        Ok(RawTextToken::EndBold) => TextToken::EndTag(Tag::Bold),
        Ok(RawTextToken::EndItalic) => TextToken::EndTag(Tag::Italic),
        Ok(RawTextToken::EndUnderline) => TextToken::EndTag(Tag::Underline),
        Ok(RawTextToken::EndStrikeout) => TextToken::EndTag(Tag::Strikeout),
        Ok(RawTextToken::EndFont) => TextToken::EndTag(Tag::Font),

        Ok(RawTextToken::LineBreak) => TextToken::LineBreak,

        // Strip the braces; the regexes guarantee both are present.
        Ok(RawTextToken::SsaCode(s) | RawTextToken::MicroDvdCode(s)) => {
          TextToken::InlineCode(InlineCode::new(&s[1..s.len() - 1]))
        }

        Ok(RawTextToken::Text(s) | RawTextToken::LiteralLt(s) | RawTextToken::LiteralBrace(s)) => {
          TextToken::Text(s)
        }

        // Every byte sequence is covered by a rule, so this is unreachable;
        // skipping keeps the iterator total.
        Err(()) => continue,
      });
    }
  }
}

/// A piece of cleaned text produced by [`PlainText::segments`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
  /// A run of literal text, borrowed from the input.
  Text(&'a str),
  /// A `<br>` line break.
  ///
  /// The newlines a multi-line cue body already contains arrive as part of a
  /// [`Text`](Self::Text) segment; only an explicit `<br>` is reported here.
  LineBreak,
}

/// A lazy iterator over the cleaned segments of a cue body.
///
/// Created by [`PlainText::segments`].  Tags and inline style codes are
/// dropped; everything else is borrowed from the input, so this iterator never
/// allocates and is available on every feature tier.
#[derive(Clone)]
pub struct Segments<'a> {
  parser: TextParser<'a>,
}

impl<'a> Iterator for Segments<'a> {
  type Item = Segment<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      return Some(match self.parser.next()? {
        TextToken::Text(run) => Segment::Text(run),
        TextToken::LineBreak => Segment::LineBreak,
        TextToken::StartTag(_) | TextToken::EndTag(_) | TextToken::InlineCode(_) => continue,
      });
    }
  }
}

/// The clean-text layer: a cue body with its markup removed, normalized
/// lazily.
///
/// This is the SubRip counterpart of [`crate::ass::text::PlainText`] and of
/// [`crate::vtt::cue::CueStr`].
///
/// # Zero-copy guarantee
///
/// Construction never allocates.  A body that contains neither `<` nor `{`
/// needs no cleaning at all, and [`normalize`] then returns the original
/// borrowed slice — the common case for dialogue lines.  Otherwise the
/// cleaned text is computed once and cached behind a [`core::cell::OnceCell`]
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
  /// Create a `PlainText` for the given raw cue body, deciding whether
  /// cleaning is needed.
  ///
  /// ```rust
  /// use fasrt::srt::text::PlainText;
  ///
  /// assert!(!PlainText::new("plain line").requires_normalization());
  /// assert!(PlainText::new("<i>styled</i>").requires_normalization());
  /// ```
  pub fn new(raw: &'a str) -> Self {
    let bytes = raw.as_bytes();

    #[cfg(all(feature = "memchr", not(miri)))]
    let dirty = memchr::memchr2(b'<', b'{', bytes).is_some();
    #[cfg(not(all(feature = "memchr", not(miri))))]
    let dirty = bytes.iter().any(|&b| b == b'<' || b == b'{');

    if dirty {
      Self::needs_normalization(raw)
    } else {
      Self::borrowed(raw)
    }
  }

  /// Create a `PlainText` that does **not** need normalization.
  ///
  /// ```rust
  /// use fasrt::srt::text::PlainText;
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
  /// use fasrt::srt::text::PlainText;
  ///
  /// let text = PlainText::needs_normalization("<i>hi</i>");
  /// assert!(text.requires_normalization());
  /// assert_eq!(text.as_raw(), "<i>hi</i>");
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

  /// Returns the raw text, exactly as it appeared in the cue body.
  ///
  /// ```rust
  /// use fasrt::srt::text::PlainText;
  ///
  /// assert_eq!(PlainText::new("<i>hi</i>").as_raw(), "<i>hi</i>");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_raw(&self) -> &'a str {
    self.raw
  }

  /// Whether this body may contain markup that cleaning would remove.
  ///
  /// This is a conservative test on the presence of `<` or `{`: it is never
  /// false for text that does need cleaning, but it can be true for text that
  /// turns out not to — a bare `<` in `I <3 this`, say — in which case
  /// [`normalize`](Self::normalize) simply reproduces the input.
  ///
  /// ```rust
  /// use fasrt::srt::text::PlainText;
  ///
  /// assert!(!PlainText::new("hello").requires_normalization());
  /// assert!(PlainText::new("<b>hello</b>").requires_normalization());
  ///
  /// // Conservative: no tag is opened here, but `<` is still flagged.
  /// assert!(PlainText::new("I <3 this").requires_normalization());
  /// assert_eq!(PlainText::new("I <3 this").normalize(), "I <3 this");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn requires_normalization(&self) -> bool {
    self.requires_normalization
  }

  /// Returns a lazy, allocation-free iterator over the cleaned segments.
  ///
  /// Use this instead of [`normalize`](Self::normalize) to apply your own
  /// policy for line breaks, or on `no_std` without `alloc`.
  ///
  /// ```rust
  /// use fasrt::srt::text::{PlainText, Segment};
  ///
  /// let text = PlainText::new("<i>Hi</i><br>there");
  /// let segments: Vec<_> = text.segments().collect();
  /// assert_eq!(
  ///   segments,
  ///   [Segment::Text("Hi"), Segment::LineBreak, Segment::Text("there")],
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
  /// Tags and inline style codes are dropped and `<br>` becomes U+000A.
  /// Everything else is kept verbatim: character references are **not**
  /// decoded, because no SubRip reader decodes them, and a `<` that opens no
  /// tag is text.  Callers wanting a different policy should use
  /// [`segments`](Self::segments).
  ///
  /// When no cleaning is needed the borrowed input is returned as-is and
  /// nothing is allocated; otherwise the cleaned text is computed once and
  /// cached.
  ///
  /// On `no_std` without `alloc` this always returns the raw text.
  ///
  /// ```rust
  /// use fasrt::srt::text::PlainText;
  ///
  /// let plain = PlainText::new("no markup here");
  /// assert_eq!(plain.normalize(), "no markup here");
  ///
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// let styled = PlainText::new("<font color=\"#ffff00\"><i>Hello</i></font>");
  /// assert_eq!(styled.normalize(), "Hello");
  ///
  /// // A bare `<` is text, and so is the rest of the line.
  /// assert_eq!(PlainText::new("I <3 this").normalize(), "I <3 this");
  ///
  /// // `&` is not an entity opener: SubRip is not HTML.
  /// assert_eq!(PlainText::new("Tom &amp; Jerry").normalize(), "Tom &amp; Jerry");
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
        Segment::LineBreak => out.push('\n'),
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
  /// use fasrt::srt::text::PlainText;
  ///
  /// assert_eq!(PlainText::new("<b>bold</b>").to_string(), "bold");
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
          Segment::LineBreak => f.write_str("\n")?,
        }
      }
      Ok(())
    }
  }
}
