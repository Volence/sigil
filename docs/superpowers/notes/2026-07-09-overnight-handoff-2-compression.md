# Overnight handoff 2 of 2 — Plan-7 #10: compression builtins

Written 2026-07-09 (Fable, post-T3-checkpoint) for the OVERNIGHT session — run AFTER
handoff 1 (`notes/2026-07-09-overnight-handoff-1-seam-reeval.md`; its ground-rules section
applies verbatim: Fable decides/verifies, sonnet/haiku implement, worktree branch, NO
functional merge without Volence's morning checkpoint, docs commits to master OK).

This note bakes in a full research pass (local survey + web, 2026-07-09) so the session can
go straight to design-freeze → plan → implement. Facts below are verified; trust them over
folklore.

## The verified fact base

**What aeon actually uses — exactly two formats, neither classic Sega:**
- **ZX0** — already a builtin (`zx0()`, byte-exact vs vendored salvador,
  `crates/sigil-salvador-sys` + `eval/sandbox.rs`). Done.
- **S4LZ** — aeon's own word-aligned LZ: encoder `aeon/tools/s4lz.py` (v3 format,
  `[u16 BE size][u8 flags: bit0=tile_delta][u8 version]` header, dictionary-seeded
  variant), decoder `aeon/engine/compression/s4lz_decompress.asm`. Today `.emp` embeds its
  output as opaque blobs; the encoder is a live build-time Python generator.
- Authoritative (empyrean `SIGIL_CORE_SPEC.md:48`): "Nemesis / Kosinski / Enigma / UFTC do
  not exist in Aeon." Aeon's only Kosinski code is a build-time Python DEcompressor
  (`tools/ojz_common.py`, ports legacy OJZ assets out of sonic_hack).

**What classic-Sonic ports demand:** s2disasm = 205 `.nem` + 70 `.kos` + 24 `.eni`
BINCLUDEs, ALL committed pre-compressed — its only build-time COMPRESSOR is Saxman (SMPS
songs; `build.lua` has an "saxman-bugged" mode reproducing the original tool's
stray-trailing-byte bug). skdisasm adds KosinskiM (95 files) + Nemesis (107) + Enigma —
all pre-compressed. **A compressor is only REQUIRED where source is authored; everything
else can stay `embed()`.** The Plan-7 research doc says this itself: "none of these are
required as builtins to hit byte-exactness — the pre-compressed blob can always be
`embed()`ed" (counts there: Nemesis 274, Kosinski 133, Enigma 30, Saxman 10, Comper 1).

**The two incompatible compressor contracts (the research's key finding):**
- *Optimal/minimum-size*: flamewing/mdcomp (aka FW-KENSC, the community incumbent — but
  **LGPL-3.0 + hard Boost dep + C++14/CMake**, and deliberately optimal-parse: it does NOT
  reproduce Sega's output) and **Clownacy/clownlzss** (**0BSD**, mixed C/header-only-C++,
  graph-based optimal: Kosinski, Kosinski+, moduled variants, Saxman, Comper, Rocket,
  Enigma — Nemesis lives in the separate **Clownacy/clownnemesis** repo, also
  accurate-capable).
- *Byte-identical-to-Sega*: requires replicating Sega's encoder BUGS —
  Clownacy/accurate-kosinski, clownnemesis's Shannon-Fano mode ("perfectly recompress all
  Nemesis data in the first two Sonics"); **no complete accurate Enigma compressor exists
  anywhere**. Per-game caveats apply (the MD BIOS used a different Kosinski tool; Kosinski
  is literally Bellard's 1990 LZEXE).
- **No Rust prior art for any of these** (crates.io + GitHub verified) — from-scratch Rust
  or vendored-C only.

**The proven integration pattern to mirror (zx0):** `-sys` crate with verbatim vendored
sources + `build.rs` (cc) + one safe `compress(&[u8]) -> Vec<u8>` + a byte-exact test vs a
committed reference vector; frontend split `eval_*` (arity/type diags) / `*_from_data`
(testable core) with the diagnostic taxonomy (`[zx0.byte-order]`, `[zx0.symbolic]`,
`[zx0.byte-range]`, `[zx0.too-large]`); pipeline golden vectors in tests/vectors/. NOTE:
zx0's 4-byte `[size][0][2]` wrapper is AEON's convention, not ZX0's — a real per-builtin
design point below.

## Fable's recommended rulings (going-in position; overnight Fable confirms/adjusts and freezes)

- **CR1 — contract: builtins are OPTIMAL compressors, not Sega-accurate.** Byte-exact
  stock-ROM rebuilds keep using `embed()` of the original blobs (the research shows that's
  how the disassemblies themselves work). Accurate-to-Sega compressors: explicitly OUT of
  scope, recorded as deferred (revisit only if a real regenerate-a-stock-asset need
  appears). This kills the mdcomp/LGPL question entirely.
- **CR2 — vendor source: clownlzss (+ clownnemesis for Nemesis), 0BSD** — license-clean
  (same ethos as salvador), C-with-header-only-C++ (cc-buildable), covers Kosinski/
  KosPlus/KosM/Saxman/Enigma(+Comper/Rocket if free). One `sigil-clownlzss-sys` crate
  (+ `sigil-clownnemesis-sys` or one combined crate — implementer's call). NOTE: s2disasm
  has an OLDER zlib-licensed clownlzss snapshot vendored locally — vendor UPSTREAM current
  (0BSD), not that copy.
- **CR3 — scope tiers (commit-per-format, each independently gated):**
  - **Tier 1 (the aeon payoff, do FIRST): `s4lz()`** — pure-Rust port of
    `aeon/tools/s4lz.py` (it's a small LZ encoder; no vendoring needed). Output = EXACTLY
    what s4lz.py emits INCLUDING the v3 header (aeon's decompressor expects it).
    Byte-exact gate: run the real s4lz.py over aeon's actual compressed-asset inputs and
    commit input/output vector pairs (find the real call sites in aeon's build first —
    survey which assets + flags/dictionary modes are live, and gate THOSE shapes). This
    finishes absorbing a real aeon build generator (the D2 vision) and is the only piece
    tomorrow's engine port could actually want.
  - **Tier 2: the classic family** — `kosinski()`, `kosinski_m()`, `kosplus()`,
    `saxman()`, `enigma()`, `nemesis()` via CR2 vendoring. Golden gates per format:
    (a) COMPRESSION vector = committed output of the vendored code itself (capture via a
    small driver at vendor time — the salvador CLI-capture pattern), documented as
    "byte-exact vs vendored clownlzss@<rev>", NOT vs Sega; (b) DECOMPRESSION round-trip
    sanity vs the workspace's real committed `.nem`/`.kos`/`.eni` blobs (decompress the
    original with the vendored decompressor, recompress, decompress again → equal plain
    bytes) — the s2disasm/skdisasm trees are a free vector farm.
  - **Tier 3 (design-note only tonight, no code): decompression builtins**
    (`kosinski_dec()` etc.) — aeon's ojz_common.py proves a real build-time DEcompression
    use case (legacy asset migration); note the shape, defer.
- **CR4 — wrapper ruling: classic-format builtins emit the RAW format stream** (no aeon
  4-byte wrapper — S2/S3K blobs are raw; the wrapper was zx0()-specific). `s4lz()` emits
  the s4lz v3 header because that IS its format. Any future aeon-wrapper need composes in
  `.emp` (`bytes(...) ++`), not in the builtin.
- **CR5 — diagnostics:** mirror the zx0 taxonomy per builtin (`[kosinski.symbolic]` etc.);
  size limits per format's real header/window constraints (e.g. Saxman's u16 size), all
  loud, never panic.

## Process

Design-check (confirm CR1-CR5 against the vendored code's real API — freeze deviations in
the plan doc) → plan with tasks per tier → sonnet implementers, TDD w/ recorded RED,
two-stage reviews on the `-sys` vendoring task and the s4lz port (Fable spot-checks the
byte-exact gates personally — that's the trust surface) → workspace + strict-gate nets
(aeon reference gates must stay untouched — these builtins add NO aeon consumers tonight;
`.emp` adoption of `s4lz()` in aeon's build is a FOLLOW-UP with its own byte-gate, note it,
don't do it) → completion note + memory + checkpoint packet, branch UNMERGED.

If the night runs short: Tier 1 + kosinski/kosinski_m are the must-haves; each further
format is an independent commit that can stop cleanly.

## After this arc (end-of-night deliverable)

Write the **68k engine-port campaign kickoff handoff**: spec-FREEZE checklist (Plan-7 is
then complete: #1-#10 all landed), the S2-D14/seam outcomes folded in, proposed first
migration targets (survey aeon's engine/ for the smallest self-contained 68k module with an
existing byte-gate story), and the morning checkpoint list for Volence (merge seam-reeval
branch, merge compression branch, ratify the kickoff). Update [[spec2-progress]].
