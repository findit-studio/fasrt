# UNRELEASED

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

# 0.1.2 (January 6th, 2022)

FEATURES


