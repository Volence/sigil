# .emp VS Code Syntax Highlighter — Design

**Date:** 2026-07-09
**Status:** Approved (Volence, 2026-07-09)
**Scope:** Local (unpublished) VS Code extension for reading-comfort syntax
highlighting of `.emp` files. Marketplace packaging, tree-sitter, and LSP
semantic tokens are explicitly out of scope; the grammar file carries over if
any of those happen later.

## Problem

`.emp` files render as plain text in VS Code, which makes them markedly harder
to read than `.asm` files (highlighted by the "Motorola 68000 Assembly"
extension, `clc_xce.motorola-68k-assembly`). `.emp` is a two-layer language —
a declaration layer (`module`/`proc`/`data`/`offsets`/…) wrapping embedded
68k (and eventually Z80) instruction lines — and both layers need color.

## Decision

A single hand-written TextMate grammar in a minimal local extension.

Rejected alternatives:

- **Injection of the M68k extension's grammar** into proc bodies — depends on
  that extension's internal scope names, doesn't know `.emp`-isms
  (`timer(a0)` typed field access, `jbra`/`jbsr`, prelude helpers like
  `anim`/`routine`/`spawn`), and fights the emp layer at boundaries.
- **LSP semantic tokens from the sigil parser** — right long-term answer,
  wildly out of scope for reading comfort today.

## Layout

```
sigil/editors/vscode/
├── package.json                  # language + grammar contribution; no deps, no build
├── language-configuration.json   # comments, brackets, auto-close, folding
├── syntaxes/emp.tmLanguage.json  # the grammar
└── README.md                     # symlink install one-liner
```

Install: symlink the directory into `~/.vscode/extensions/` (documented in the
README). No `vsce`, no npm, no build step.

## Grammar

Standard TextMate scope names throughout, so the user's existing theme colors
`.emp` the same way it colors the 68k extension's output.

Ground truth for the token surface (verified 2026-07-09):

- **Keywords** — the 58-keyword set extracted from
  `crates/sigil-frontend-emp/src/parser.rs`: `align asm bind bitfield block
  clobbers comptime const data default dispatch else encoding ensure
  ensure_fatal enum equ expect_error export falls_into fixed fn for if in
  label let long_ptrs loop match max_size mnemonic module name newtype
  offsetof offsets patch proc pub register rescale return script section
  shows size sizeof struct test todo unreachable use var vars wait_frames
  where while word_offsets yield`
- **Lexical forms** — from `crates/sigil-frontend-emp/src/lexer.rs`:
  `//` line comments, `///` doc comments (but `////` is a plain comment),
  `/* */` block comments; integer literals as decimal, `$`-hex, `0b`-binary,
  and `%`-binary (only when `%` is immediately followed by a binary digit —
  otherwise `%` is the modulo operator); char literals `'a'`.

Scope mapping:

| Token class | Members (representative) | Scope family |
|---|---|---|
| Doc comment | `/// …` | `comment.block.documentation` |
| Comment | `// …`, `/* … */` | `comment.line` / `comment.block` |
| Declaration keywords | `module proc script const data vars offsets section struct enum newtype bitfield fn let use pub dispatch patch test comptime bind equ label block asm var export fixed register mnemonic name` | `keyword.declaration` / `storage.type` |
| Control keywords | `yield wait_frames return if else match loop while for expect_error` | `keyword.control` |
| Attribute/modifier words | `in clobbers shows falls_into encoding max_size align where size long_ptrs word_offsets rescale` | `storage.modifier` |
| Builtins | `ensure ensure_fatal here sizeof offsetof default unreachable todo` | `support.function` |
| Primitive types | `u8 u16 u32 i8 i16 i32`, `*Name` pointers | `entity.name.type` / `storage.type` |
| PascalCase identifiers | `ObjDef`, `Ani.Idle`, `PitcherPlantV` | `entity.name.type` |
| Numbers | `$60`, `0b1010`, `%1010`, `64` | `constant.numeric` |
| Char/string literals | `'a'`, `"…"` (with `{…}` interpolation in ensure messages left plain) | `string` |
| 68k mnemonics | `move.w subq.b cmpi.b tst.b bne bhi jbra jbsr dbf …` (full mnemonic table + optional `.b/.w/.l/.s` suffix, line-leading position inside bodies) | `keyword.mnemonic` / `support.function.mnemonic` |
| Z80 mnemonics | `ld jp jr djnz call ret …` | same as 68k mnemonics |
| Registers | `d0–d7 a0–a7 sp usp sr ccr pc`; Z80: `a b c d e h l af bc de hl ix iy sp i r` + shadow `af'` | `variable.language.register` |
| Immediates | `#CONST`, `#$28` | `keyword.operator` + nested numeric/ident |
| Local labels | `.draw:` definitions, `.draw` references | `entity.name.label` |

Field/helper calls (`timer(a0)`, `anim`, `routine`, `spawn(…)`) fall out as
generic function-call scope + register scope; no special-casing.

Embedded asm needs no stateful region tracking: an instruction line is
recognized lexically (known mnemonic, optionally size-suffixed, at
line-leading position). This keeps the grammar flat and robust.

`language-configuration.json`: `//` line comment, `/* */` block comment,
bracket pairs `{} [] ()`, auto-closing pairs (including quotes), indentation
brackets for folding.

## Verification

Open the three richest exhibits in VS Code and eyeball against the 68k
extension's rendering of `.asm`:

- `examples/game/badniks/pitcher_plant.emp` (procs, offsets, data, vars, asm)
- `examples/game/badniks/pitcher_plant_script.emp` (script/yield/wait_frames)
- `examples/guards.emp` (ensure/ensure_fatal, sections, max_size)

No automated grammar snapshot tests (npm infra not worth it for a local
extension; revisit if this ever heads to the marketplace).
