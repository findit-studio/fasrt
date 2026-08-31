# ASS/SSA fixtures

These scripts are **authored for this test suite**, not copied from a released
subtitle track: the crate ships no third-party subtitle content. Each one
reproduces the structure, field order and markup conventions that real files
use, so the parser is exercised against the shapes it will actually meet.

| Fixture | Shape it reproduces |
| --- | --- |
| `aegisub_dialogue.ass` | Canonical Aegisub output: `[Script Info]` with `; ` comments, `[V4+ Styles]`, `[Events]` in the ASS v4+ field order. LF endings, canonical field spellings — used for the byte-exact round-trip test. |
| `typesetting.ass` | Fansub typesetting: `\pos`, `\an`, `\move`, `\t`, `\fad`, `\clip`, nested `\t(…\frz…)`, `\p1` vector drawings, `\N` breaks, commas inside `Text`. |
| `karaoke.ass` | Karaoke timing: long `\k`/`\kf`/`\ko` runs, per-syllable override blocks, `\K`, an `Effect` column in use. |
| `speaker_names.ass` | The `Name` column populated on every row, including names with spaces and non-ASCII names — the person-name observation material an indexer wants. |
| `ssa_v4.ssa` | Legacy SSA v4: `[V4 Styles]`, a `Marked=0` first field instead of `Layer`, `ScriptType: v4.00`. |
| `embedded_fonts.ass` | `[Fonts]` and `[Graphics]` sections: `fontname:`/`filename:` headers followed by encoded payload lines, including payload lines that contain `:` and `;`, which must not be read as properties. |
| `crlf_bom.ass` | CRLF line endings and a leading UTF-8 BOM, as produced by Windows tooling. |
| `malformed.ass` | Deliberately broken input for the lossy-mode tests: short rows, an unclosed section header, a bad timestamp, a padded `0000` margin, an event before any `Format:` line, an unmatched `{`. |

`malformed.ass` is the only fixture that is not valid ASS; every other file is
expected to parse cleanly in strict mode.
