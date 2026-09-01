# UNRELEASED

# [0.4.0](https://github.com/findit-studio/fasrt/releases/tag/v0.4.0) (September 1st, 2026)

FIXED

- **Cue text class lists now exclude the empty string, as W3C WebVTT §6.4
  requires.** §6.4 attaches a node whose "list of applicable classes" is the
  start tag's classes *"excluding any classes that are the empty string"*, but
  `vtt::cue::TagNode::classes` returned the raw dot-separated slice — so
  `<c.a..b>` gave `"a..b"`, and a consumer splitting it on `.` saw an empty
  class the spec says cannot exist. `classes` now returns that list as a lazy
  `vtt::cue::Classes` iterator; the raw slice keeps a name that says it is raw.
  The crate's own WPT conformance harness was one of the consumers taking the
  trap: it spelled `<c.a..b>` as `class="a  b"` where §6.4's serialization is
  `class="a b"`.

- **Cue text tags now end on all four of §6.4's whitespace characters.** §6.4's
  tokenizer leaves the tag-name and class-list states on TAB, LF, FF or SPACE;
  this parser recognized only TAB and SPACE. A cue payload spans lines, so an
  LF inside a tag is reachable input, and the consequences reached both faces
  above: `<lang` + LF + `en>` was discarded as an unknown tag, taking the whole
  language scope with it, and `<c.a` + LF + `note>` kept `"a\nnote"` as a
  single class. Both are now read as §6.4 reads them, in terminated and
  unterminated tags alike. The annotation is trimmed over ASCII whitespace —
  those four plus CR, which ends no state but is still trimmed — rather than
  over Unicode's whitespace set, so a NO-BREAK SPACE around a `<v>` voice is
  text rather than padding. No cue body in the crate's fixture corpus reaches
  any of these shapes, and the WPT suites are unchanged.

- **Cue text annotations now run §6.4's annotation state in full.** The
  tokenizer trimmed an annotation and stopped there; §6.4 also decodes the
  character references it met while filling the buffer, and then *"replaces any
  sequence of one or more consecutive ASCII whitespace characters ... with a
  single U+0020 SPACE character"*. So `<lang en&#x2D;US>` declared
  `"en&#x2D;US"` rather than the language `"en-US"`, and `<v Roger  Bingham>`
  named a voice no `<v Roger Bingham>` would match. Both are now done, for
  `<v>`'s voice exactly as for `<lang>`'s language, in terminated and
  unterminated tags alike, through the same character-reference decoder the
  cue text data state already used — the two states differ only in what becomes
  of the decoded characters, so they share one decoder and differ in one sink.

  The two steps compose in the order §6.4 writes them, decode before collapse,
  because the buffer §6.4 collapses holds decoded characters: `<v a&#x20; b>`
  is one run of whitespace and gives `"a b"`, and `<v &#x20;a>` is padded
  rather than starting with a space.

  Nothing in the crate's fixture corpus moves — of the 10 annotations in 302
  WebVTT cue bodies, 0 change — and the five WPT cue-parsing suites are green
  before and after with no expectation edited. The crate's own WPT harness was
  reading the raw text into the `title` and `lang` attributes it compares, and
  now reads the normalized annotation a browser would put there.

- **A numeric character reference to a legacy C1 code point now decodes to the
  character the author meant.** WebVTT consumes character references through
  HTML's algorithm, whose numeric character reference end state replaces the
  0x80–0x9F range with the Windows-1252 characters authors write them for, so
  `&#x80;` is a EURO SIGN and not U+0080. The decoder handed the code point
  straight to `char::from_u32` and produced the C1 control. All 27 rows of the
  table are read now, in both radices; the five code points in that range the
  table omits still pass through, and the end state's other substitutions — NUL,
  a surrogate and a value past U+10FFFF, each U+FFFD — are unchanged. This is
  one decoder, so it fixes `vtt::cue::CueStr::normalize` for cue text as much as
  the new annotation state. No fixture in the corpus carries such a reference,
  so the WPT suites are unchanged. (Found by the adversarial review of the
  annotation state, in the decoder it newly exposed.)

- **Fixture corpora are pinned against end-of-line conversion.** A new
  `.gitattributes` marks `fixtures/**` (and the build script's entity table)
  `-text`, so Git checks them out byte for byte on every platform. Without it a
  Windows checkout under `core.autocrlf=true` — the default on the GitHub
  Actions Windows runners — rewrote all 245 LF-stored fixtures to CRLF, and
  `tests/ass.rs`'s byte-for-byte writer battery compared an LF write against a
  CRLF file and failed. Not `eol=lf`: 131 SubRip fixtures, one ASS fixture and
  a WebVTT fixture that mixes both terminators carry their line endings on
  purpose, and normalization would rewrite them.

- **Cue text `<ruby>` trees now follow W3C WebVTT §6.4.** Two branches of the
  cue text parsing rules were read more loosely than the spec writes them, and
  both are on ruby paths:

  - `<rt>` attached whenever a `<ruby>` was open anywhere above it. §6.4 says
    "If *current* is a WebVTT Ruby Object, then attach a WebVTT Ruby Text
    Object" — the test is on the current node. `<ruby><b><rt>x</rt></b></ruby>`
    used to build `ruby > b > rt > "x"`; it now builds `ruby > b > "x"`, with
    the `<rt>` ignored and its text left in the `<b>`.
  - Every end tag first closed any open `<rt>`. §6.4 closes an open `<rt>`
    from `</ruby>` alone, and that clause closes the `<ruby>` too, in one
    step; every other unmatched end tag is ignored.
    `<ruby><rt>x</b><i>y` used to build `ruby > [rt > "x", i > "y"]`; it now
    builds `ruby > rt > ["x", i > "y"]`, because `</b>` names nothing that is
    open.

  A token the spec ignores also costs no nesting depth, so a cue whose §6.4
  tree fits `vtt::cue::Options::max_depth` is no longer refused for a level it
  does not have. Cue text without `<ruby>`/`<rt>` is unaffected: enumerating
  all 5 380 840 payloads of up to seven tokens over
  `<ruby> </ruby> <rt> </rt> <b> </b> <i> </i> x` finds 354 654 that changed,
  every one of them in one of the two branches above, and the WPT cue-text
  suites are unchanged — every ruby case they carry puts `<rt>` directly
  inside `<ruby>`, where both readings agree.

- **Deeply nested cue text no longer aborts the process.**
  `vtt::cue::CueText::parse` now bounds the tree it builds at
  `vtt::cue::DEFAULT_MAX_DEPTH` (16). WebVTT places no limit on cue payload
  nesting and the depth is chosen by the file, so a corrupt or hostile cue —
  a few thousand nested `<i>` in an embedded `S_TEXT/WEBVTT` packet, say —
  used to build a tree that the recursive walks over it could not survive:
  the compiler's drop glue, `Display`, `Debug`, `Clone` and `PartialEq`. A
  stack overflow is an `abort`, not a catchable panic, so it took down the
  whole process. Measured against 0.3.0 on a 2 MiB stack (debug,
  `aarch64-apple-darwin`), `Display` aborted from ~470 levels, `Clone` and
  `PartialEq` from ~1 600, `Debug` from ~1 800 and drop from ~7 700; tree
  *construction* was already iterative and survived 20 000.

  The default is chosen from both ends: it is five times the deepest cue in
  the crate's whole fixture corpus (107 264 cue bodies nest three deep) and
  twice WebVTT's entire eight-tag vocabulary, while keeping the most expensive
  walk — `Display`, at roughly 3.7 KiB of stack per level unoptimized — inside
  a 128 KiB thread, a sixteenth of the stack a Rust thread is given by
  default. Cue text within the limit parses exactly as it did before.

- **The test suite now compiles and runs on every feature tier.**
  `tests/ass.rs` imported the `std`-gated `ass::Writer` unconditionally, so
  `cargo hack test --feature-powerset` failed to build at
  `--no-default-features` and at `--features alloc`; beneath that failure both
  ASS test files asserted cleaned text that `PlainText::normalize` can only
  produce with `alloc`. The `std`- and `alloc`-only cases are gated to the
  tiers where they mean something, so the no-alloc tier the crate advertises
  is now exercised rather than merely built.

FEATURES

- Add `vtt::cue::Options`, carrying the cue text `max_depth` bound, and
  `vtt::cue::DEFAULT_MAX_DEPTH`.
- Add `vtt::cue::CueText::parse_with`, which parses under given `Options`.
  Markup nested past the limit is dropped exactly as an unrecognized tag
  already is: the text is kept and the matching end tag is still consumed, so
  the structure following an over-deep run is the structure an unbounded parse
  would have produced.
- Add `vtt::cue::CueText::try_parse` and `try_parse_with`, which refuse
  over-deep input with the new `error::MaxDepthExceededError` instead of
  flattening it.
- Add `vtt::cue::Classes`, the lazy iterator over §6.4's list of applicable
  classes. `Classes::new` reads it straight from a `vtt::cue::CueToken::StartTag`'s
  raw class list, so a token-level consumer needs no tree.
- Add `vtt::cue::TagNode::classes_raw`, the raw dot-separated class list
  exactly as it appeared — the source text `classes` used to return.
- Add `vtt::cue::TagNode::declared_language`, the language a node makes
  applicable to its own subtree: §6.4's language-stack push, which only
  `<lang>` performs. An annotation-less `<lang>` pushes the empty string, so it
  clears an enclosing language rather than inheriting it.
- Add `vtt::cue::CueText::nodes_with_language` and `vtt::cue::NodesWithLanguage`,
  a document-order walk pairing every node with the language §6.4 makes
  applicable to it. §6.4's language stack is exactly the chain of enclosing
  `<lang>` nodes, so the language is derived from the tree rather than stored on
  each node, where an edit through `children_mut` could leave it stale. The walk
  keeps its ancestors on the heap and so costs no stack in the depth of the tree.

  The language is an `Option<&str>`, and the two empty answers are different.
  `None` is an empty language stack: nothing in the cue speaks to that node's
  language, so a fallback from outside the cue — a track's language, say —
  applies to it. `Some("")` is an annotation-less `<lang>`, which pushed the
  empty string and thereby said the subtree is in no known language, clearing
  that fallback rather than deferring to it.

  Both the scope and the value are §6.4's. The value is the annotation its
  annotation state produced, so `<lang en&#x2D;US>` declares `"en-US"`; read
  `TagNode::annotation` for the text the cue spelled. One limit remains, and it
  is documented and pinned by fixtures: the walk answers for the tree it is
  given, so a `<lang>` that `Options::max_depth` dropped is not in the tree and
  its scope is not either, exactly as its classes and markup are not.
  `try_parse` refuses input that deep rather than returning a tree that dropped
  part of it.

- Add `vtt::cue::Annotation`, the carrier a start tag's annotation now travels
  in. `Annotation::normalize` is §6.4's annotation — character references
  decoded, NULs replaced, ASCII whitespace trimmed and its runs collapsed —
  and `Annotation::as_raw` the text the cue spelled, which is what a node is
  serialized from and must be: a normalized annotation may hold a U+003E that a
  `&gt;` stood for, and writing that back would end the start tag early. For
  that reason the type deliberately implements no `Display`.

  It borrows wherever §6.4 allows: an annotation already in normal form is its
  own normalized value and is handed back pointing into the cue, so the parser
  still never allocates, and `Annotation::requires_normalization` says which
  case a caller is in. Without `alloc` there is nowhere to put a decoded string,
  so `normalize` returns the stored text — the same degradation
  `vtt::cue::CueStr::normalize` already makes for cue text, and the reason the
  annotation carries a flag rather than an owned string: the shape of the answer
  is the same on every feature tier.

- Add `srt::text`, a clean-text layer for SubRip cue bodies. WebVTT and
  ASS/SSA each had one; SubRip did not, so an embedded `S_TEXT/UTF8` packet —
  a cue *body*, with no index line and no timing line — had nothing in this
  crate to point at. `srt::text::TextParser` is a zero-allocation `logos` DFA
  over a body, available on every feature tier, and `srt::text::PlainText`
  mirrors `ass::text::PlainText`: deferred normalization, an `OnceCell` cache,
  and an allocation-free `segments()` iterator. A body containing neither `<`
  nor `{` is returned borrowed and never allocates.

  SubRip has no specification, so the dialect is the common ground between
  FFmpeg's `subrip` decoder, VLC's subtitle decoder and Aegisub's SRT reader,
  and every rule is documented against them. `<b>`, `<i>`, `<u>`, `<s>` and
  `<font>` are markup, matched case-insensitively, with `<font>`'s attributes
  readable through `StartTag::attrs`; `<br>` in every form is a line break;
  SSA and MicroDVD inline codes left behind by a converter are dropped. Two
  rules matter most, and both are where routing a SubRip body through the
  WebVTT cue-text layer gives a wrong answer rather than a lucky one: a `<`
  that begins none of those tags is literal text, and so is the rest of the
  line — `I <3 this` survives, as do the 198 lines of the crate's own
  fixture corpus that open a Japanese narration bracket and never close it —
  and character references are not decoded, because no SubRip reader
  decodes them.

  The module builds no tree, so no nesting depth can overflow the stack and
  there is no depth bound to configure.

CHANGED

- **`vtt::cue::TagNode::classes` returns `vtt::cue::Classes` rather than
  `&str`.** This is the class-list fix above, and it is a breaking change to a
  published signature: it was taken deliberately, because the trap was the name
  — `classes()` reads as "the classes" and returned something that is not the
  list. Every call site fails to compile rather than changing meaning silently.
  To migrate: `node.classes()` → `node.classes_raw()` for the old return value,
  or iterate it for §6.4's list. `with_classes` and `set_classes` are unchanged
  and still take the raw dot-separated form.

- **A cue text annotation is a `vtt::cue::Annotation`, not a `&str`.** §6.4's
  annotation state cannot be run over a slice borrowed from the cue — decoding
  and collapsing both produce text the cue does not contain — so the annotation
  moved into a carrier that can hold both answers, across
  `vtt::cue::CueToken::StartTag`'s field, `vtt::cue::TagNode::annotation`,
  `with_annotation` and `set_annotation`. Like the class-list change this breaks
  loudly: the type changed, so every call site fails to compile.
  To migrate: `node.annotation()` → `node.annotation().map(Annotation::as_raw)`
  for the old return value, or `.map(Annotation::normalize)` for §6.4's;
  `with_annotation(Some("en"))` → `with_annotation(Some(Annotation::new("en")))`.

- **`vtt::cue::TagNode::declared_language` returns §6.4's annotation and is no
  longer `const`.** What §6.4 pushes onto its language stack is the value the
  annotation state produced, so the accessor normalizes, which no `const fn`
  can do. `vtt::cue::CueText::nodes_with_language` reports the same value, and
  its language borrows from the tree rather than from the cue.

# [0.3.0](https://github.com/findit-studio/fasrt/releases/tag/v0.3.0) (August 31st, 2026)

FEATURES

- Add ASS/SSA support in a new `ass` module, on the same two-layer shape as
  `vtt`, so an indexer can extract content without a renderer.
  - `ass::Parser` — a lazy, zero-copy document parser over `[Script Info]`,
    `[V4 Styles]`, `[V4+ Styles]`, `[Events]` and unknown sections, yielding
    one `ass::Block` per meaningful line. It needs neither `std` nor `alloc`.
  - `ass::Event` — an event row, with the `Format:`-declared field order
    resolved through `ass::EventFormat`. `EventFormat::ass()`,
    `EventFormat::ssa()` and `EventFormat::matroska()` cover the three orders
    that occur in practice; `Event::parse_fields` reads a bare field list, so
    an embedded Matroska `S_TEXT/ASS` packet parses standalone, one event per
    packet, with the container's timing authoritative.
  - The `Name` (a.k.a. `Actor`) speaker column is surfaced as first-class
    data.
  - `ass::text::TextParser` — a logos DFA token stream over an event's `Text`
    field: override blocks, `\N`/`\n`/`\h` escapes, and `\p<n>` drawing
    payloads that are tokenized and skipped, never interpreted as geometry.
    Tag boundaries follow libass: names are longest-match so `\fscx` is not
    `\fs`, spaces after the backslash are skipped, and an argument list ends
    at the first `)`, so `\t(0,500,\frz360)` stays a single tag.
    Brace handling follows libass: a `{` opens a block only when a `}`
    follows, and `\{`/`\}` are literal-brace escapes, so an unmatched brace
    never swallows the text after it.
  - Columns an `[Events]` `Format:` line declares under a name this crate does
    not recognize keep their values through a write, readable by position via
    `ass::Event::field`.
  - `ass::text::PlainText` — clean-text extraction with deferred
    normalization: a field with no `{` and no `\` never allocates and
    returns the borrowed input unchanged. `PlainText::segments()` is
    allocation-free on every feature tier.
  - `[Fonts]` and `[Graphics]` payload lines are kept verbatim as
    `ass::Block::Data`, since the encoding alphabet includes `:` and a payload
    line must never be read as a property.
  - `ass::Options` — strict and lossy presets, matching `srt::Options`.
  - `ass::Writer` — a `std` writer with round-trip fidelity; event rows are
    emitted in the declared field order.
- Add `types::Centisecond`, the sub-second unit of ASS/SSA timestamps, and its
  `error::ParseCentisecondError`.

CHANGED

- **MSRV raised `1.85` → `1.95`.** Internal control flow in `ass` and `vtt`
  now uses stable let-chains and `Vec::pop_if`.
- **`phf` and `phf_codegen` floors raised `0.13` → `0.14`.** Both are
  internal: `phf` backs the crate-private HTML5 entity lookup table and
  `phf_codegen` only regenerates it at build time, so the public API is
  unchanged.

INTERNAL

- Stable-clippy `-D warnings` lint fixes across `vtt`/`ass` control flow and
  the benches; no public API or behavior change.

# 0.1.2 (January 6th, 2022)

FEATURES


