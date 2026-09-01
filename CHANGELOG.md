# UNRELEASED

FIXED

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


