# SIGIL-AS-REPLACEMENT — first pass over the community Sonic 2 disassembly

**Project:** `SIGIL-AS-REPLACEMENT` (`empyrean/contract/projects.json`). Started 2026-09-03
under the owner's overnight fallback, verified at its carrying commit (empyrean `61dfcaa`)
rather than taken from the relay. Evidence base for what to look for: empyrean
`docs/2026-09-02-as-community-feedback.md` at `8a72692`, confirmed reachable from their
`origin/main`.

**Scope of this pass: BOOK WHAT BREAKS.** Read-only against `~/sonic_hacks/s2disasm`
(`e45ebf3`); nothing in aeon touched, no golden, no pin. One fix landed, under the standing
bound allowing a Sigil defect that a single test pins.

## Provenance

| what | value |
|---|---|
| corpus | `~/sonic_hacks/s2disasm` at `e45ebf3`, 332 `.asm` files, 130,025 lines |
| the file assembled | `s2.asm`, 91,276 lines |
| assembler | `.target-land/release/sigil`, built from sigil master this session |
| aeon | not read, not built, not touched |

---

## 1. LANDED — `include` resolved against the caller's cwd, not the source file

Fixed at sigil `4e22e536`; suite GREEN 4233/0/2, reconciling 4232 + 1 new.

`sigil <file.asm>` called `assemble(&str)`, which leaves `Options::include_root` unset, so
every `include` resolved against whatever directory the user was standing in.
`assemble_root(&Path)` exists for exactly this and is documented as *"resolving `include`
paths relative to its parent directory"*. Every in-tree caller that assembles a real
multi-file program already reached it; **only the bare CLI did not, and the bare CLI is the
entry point nobody here uses and every outsider starts with.**

| run | diagnostics |
|---|---|
| before, from s2disasm's own directory | 237 |
| before, from one directory up | **59,122** |
| after, either way | 237, byte-identical output |

The 59,122 opened with five `cannot include` lines naming files that are plainly present,
then misparsed the whole 91k-line source as a cascade from the constants and macros that
never loaded. **Nothing in that output said the paths had been resolved against the wrong
directory**, so it reads as *"this assembler cannot handle my project"* — an adoption verdict
formed in the first thirty seconds, from a one-line defect.

## 2. THE HEADLINE — not one diagnostic carries a file or a line number

**Zero of 237, against a 91,276-line source.** Measured:
`grep -cE "\.asm|:[0-9]+|line [0-9]+"` over the diagnostic output returns `0`.

The cause is not missing information. `sigil_span::Diagnostic` carries `primary: Span`, and
`SourceMap::location(span) -> (line, col)` already exists. The CLI prints
`eprintln!("error: {}", d.message)` and discards the span. **But the fix is not one line:**
`sigil-frontend-as` builds no `SourceMap` at all (`grep -rn SourceMap crates/sigil-frontend-as/src/`
returns nothing) and stamps every span `SourceId(0)`, so with `include` splicing many files
there is currently nothing to resolve an offset against. Threading a source map through the AS
front end is a real parcel; it is **not** an overnight fix and is deliberately not attempted here.

**Why this ranks above every convenience item in the feedback doc.** The complaint ordering
that doc sets is trust first, build time second, conveniences third. A diagnostic with no
location is a trust failure of the first order, and it is one where **Sigil is currently behind
the thing it proposes to replace**: the doc's own screenshot transcript shows AS reporting
`smps-bug.asm(9): error: StartOffset is located after EndOffset somehow!`. AS says where. Sigil
does not.

## 3. SETTLED BY EVIDENCE — nameless temporary labels (`+` / `-`)

`AS-COMPAT-NAMELESS-LABELS-DISPOSITION` sat open for weeks with the reason *"nothing in the
engine uses them, so no test can settle it; it is a language-surface decision."* The corpus
settles it:

- `s2.asm`'s own header instructs the reader to go and learn nameless temporary symbols
  *"before diving too far into this disassembly"*;
- **1,875 definitions** and **1,764 branch references** in `s2.asm` (control: the same
  instrument finds 2,040 `rts` tree-wide, so it is not returning empty for a broken-command
  reason);
- both directions fail today — forward `bra.s +` gives `bad operand expression`, backward
  `bra.s -` gives `expected mnemonic, directive, or label`.

**Ruled (hub, under the owner's project declaration; recorded as `d-22`): the `@as_compat`
surface must accept them.** The corpus is unassemblable without them and assembling it is what
the project is for. **Whether they enter the `.emp` language proper is NOT ruled** — that stays
the owner's taste call under `d-6`, and the split is what keeps the ruling inside its warrant.

## 4. RULED OUT BY MEASUREMENT — five things that are NOT broken

Recorded because a first pass that only lists breakage sends the next person re-treading
ground, and **every one of these was a hypothesis this pass formed and then refuted with a
minimal case.** Each was run standalone through the CLI:

| construct | example | verdict |
|---|---|---|
| `#` as the modulo operator | `dc.w ((B-A)#$10)/2` | **OK** — does not collide with immediate syntax |
| `endm` closing a `rept` | `rept 2` / `nop` / `endm` | **OK** (so is `endr`) |
| `set` / `equ` / `:=` on plain names | `Foo set 4`, `Foo := 4` | **OK**, including redefinition |
| dotted local names as assembly-time variables | `Routine:` / `.a set 4` / `dc.w .a` | **OK** |
| `rept` from a constant, symbol, `equ` pair, or backward label arithmetic | `rept (RamEnd-RamStart)/4`, `rept (*-A)/2` | **OK** |

**The dotted-name row is the one worth reading twice.** It first measured as BROKEN and the
finding was withdrawn: the minimal case had no enclosing global label, which is not valid AS in
the first place. Re-run in its real shape — under a global label, as `s2.asm` writes it — it
passes. *A minimal case can be minimal enough to stop being the construct.*

## 5. STILL BROKEN, with minimal reproductions

| construct | diagnostic | s2 site |
|---|---|---|
| nameless labels, both directions | `bad operand expression` / `expected mnemonic, directive, or label` | 1,875 |
| `rept` whose count names a label defined ON the rept's own line | `unresolved rept count` | — |
| `rept` whose count names a FORWARD label | `assembly did not converge within 16 passes (symbol values still changing)` | — |
| backward `org` (declare a RAM map high, then return) | `org target precedes the current phase base` | RAM maps |
| a label beginning with a digit | (bucket 3 below) | `s2.sounddriver.asm:159`, `1upPlaying:` |
| a macro invoked with an unquoted argument containing spaces | (bucket 3 below) | `s2.asm:3935`, `palette Special Stage 1 2p.bin` |

**The forward-label `rept` is worth flagging beyond its own row.** vladikcomper's complaint
about AS is *"instability... most of it comes from multi-pass architecture and how the next
pass forward-references everything from the previous pass"*. Sigil hits a **16-pass
convergence limit** on the same shape. Whether that case is even well-defined is a separate
question — the rept's size depends on the label and the label's position depends on the
rept's size — but *"did not converge within 16 passes"* is the same class of answer the
community is trying to get away from, and it deserves a decision rather than a limit constant.

## 6. HONESTLY UNRESOLVED — the 237 do not decompose yet

The buckets, from the after-fix run:

| count | diagnostic |
|---|---|
| 121 | `trailing tokens in expression` |
| 53 | `unresolved rept count` |
| 30 | `malformed number (hex needs a trailing h)` |
| 22 | `expected mnemonic or directive after label` |
| 9 | `unknown directive or mnemonic` (`purecode`, `endm`, `STRUCT`, `ENDSTRUCT`, `objoff_30`, `status_secondary.*`) |
| 1 | `The RAM variable declarations are too large by $N bytes.` |
| 1 | `phase needs a constant expression` |

**The malformed-number bucket is measured and NOT explained, and this is its second wrong
mechanism.** The lexer rule is: a digit-leading run must be all digits or `h`-suffixed
(`lexer.rs:194-201`). A first scan found 23 distinct offending tokens / 73 occurrences and was
**wrong** — it matched inside quoted `BINCLUDE` filenames (`16x16`, `128x128`, `01_1`). Re-run
with string literals stripped, across all 332 files: **2 distinct tokens, 4 occurrences.**
Against 30 diagnostics. **The residual is unaccounted for.** The untested hypothesis is macro
expansion — AS macros substitute textually, so an offending token need not exist in any source
file — and it is recorded as a hypothesis rather than a finding because nothing here has tested it.

**Why the buckets cannot be decomposed further right now: finding 2.** With no file or line on
any diagnostic, there is no way to walk from a count to the construct that produced it. Every
attribution in section 5 was obtained by forming a hypothesis and building a minimal case, which
is why two of them were wrong before they were right. **Finding 2 is therefore not only the
headline for adoption — it is the blocker on this project's own next pass**, and it should be
sequenced first for that reason as well as for the user-facing one.

## What this pass did not do

No build-time measurement (the community's second-ranked complaint, ASM68K's 0.1 s as the bar);
the corpus does not assemble yet, so a wall-clock figure would be measuring a failure. No
`ref()`/`IFUSED`, `RS`, or `FSIZE` work. No Windows build. All of these stay booked in the
project entry.
