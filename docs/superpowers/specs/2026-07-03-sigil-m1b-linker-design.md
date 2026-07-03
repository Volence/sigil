# Sigil M1.B — The Full Linker (design)

**Status:** design, awaiting review · **Date:** 2026-07-03 · **Track:** Sigil Core backend/linker (not the Spec-2 `.emp` surface language)
**Predecessor:** M1.A (full 68000 ISA encoder, byte-exact vs `asl`, merged @ `04cf9ea`)
**Core spec:** `empyrean/docs/SIGIL_CORE_SPEC.md` §5.4–5.8, §6 (this doc translates §6 into the current IR reality)

---

## 0. Where M1.B sits

M1 decomposes A → B → C → D. **A** built the 68000 encoder (a leaf that takes fully-resolved operands). **B** (this doc) grows `sigil-link` from the M0 Z80-blob linker into the full linker that turns a converged `Module` into a correct single-image 68k ROM: external memory map, single-image output (`p2bin` replacement), 68k fixup resolution, `jmp`/`jsr` operand-width selection, the header checksum (`fixheader` replacement), the `convsym` no-op, and the `s4.lst` symbol listing. **C** is the AS front-end that actually parses Aeon source and *drives* this linker; **D** is the full-ROM `sha256` identity gate. B is testable **without** C by hand-building 68k IR modules and diffing against `asl`-assembled snippets — the same asl-oracle TDD that carried A.

B does **not** assemble real Aeon source (that needs C). B's gate is: the linker produces byte-correct output for hand-built IR corpora, reproduces the reference ROM's checksum, and emits an `s4.lst` the live tools accept.

---

## 1. Goal & what B retires

Retire three external build tools and one deferred-encoder gap:

- **`p2bin`** → single-image layout: walk regions in map order, place each section at its LMA, one contiguous `s4.bin`, gaps filled with the map default (`0x00`). (§6.3)
- **`fixheader`** → the header checksum as the genuinely-last byte-mutating pass. (§6.5)
- **`convsym`** → a verified no-op: append **nothing**. (§6.5.1)
- **M1.A's deferred inbox** → symbolic-branch / PC-relative lowering and bare-symbol `jmp`/`jsr` width selection, which need layout/fixup resolution the encoder (a leaf) can't do.

Non-goal: general relaxation. Aeon pins every branch size explicitly; the **only** length-variable construct in the corpus is bare-symbol `jmp`/`jsr` width. We build exactly the fixpoint that resolves that, and nothing more (this fixpoint doubles as the §5.4 bounded guardrail).

---

## 2. Factual scope (grounded, not guessed)

Verified against the tree during design:

- **ASFLAGS** (`aeon/build.sh:27`): `-cpu 68000 -xx -n -q -c -A -L -U`. `-A` is **live** — its byte effect on width selection is in scope (§5.6).
- **Checksum algorithm — verified by recomputation.** 16-bit **big-endian additive word-sum over `[$200, EOF)`**, stored at `$18E`. Confirmed against the on-disk `aeon/s4.bin`: sum == stored word. ✔
- **Reference ROM is a MOVING target (confirmed in real time).** The handoff quoted length `458737` / checksum `$8553`; the current on-disk `aeon/s4.bin` is a *newer* build: length **458666**, checksum **$5CBE** at `$18E`. The *algorithm* is invariant; the *constants* are properties of a pinned aeon commit. **B pins the aeon reference commit and treats length/checksum as build-derived-and-verified, never hardcoded magic** (§6 of this doc).
- **`asl` is a prebuilt binary in-tree** (`aeon/tools/asl`; no C source present). The `abs.w`/`abs.l` width rule (§5.6, spec R4) is therefore established **empirically through the oracle** — assemble `jmp`/`jsr label` across the width boundary, with/without `-A`, observe the choice, pin golden vectors — optionally cross-checked against upstream AS source fetched separately. Not a guess: the oracle is the spec.
- **`s4.lst` consumers pinned.** `tools/s4budget.py::parse_symbol_table` reads the **"Symbol Table" section** (regex: `(\*?)([\w.]+)\s*:\s*(hex|"str")\s+([C\-])\s*\|`, terminated by `\s+\d+ symbols`). An Oracle parser (`oracle/linux-port/gui/Symbols.cpp`) reads the **per-source-line body listing** header `(D) L/ADDR : … SRCLINE`. The spec cites `Symbols.h::LoadFromAsListing` — **which exact Oracle path loads `s4.lst` is a plan-time verification item**; the M1.d gate (Oracle loads Sigil's `s4.lst` and resolves a known symbol identically to `asl`'s) is the arbiter.

### 2.1 B's inbox from M1.A (the explicitly-deferred items)
1. Bare-symbol `jmp`/`jsr` `abs.w`/`abs.l` width selection (§5.6 + `-A`).
2. `Pcd16`/`Pcd8Xn`/branch-`Disp` **target→disp** resolution — the encoder takes an already-resolved displacement measured from the extension word; B computes it from the target label at layout.
3. Symbolic-branch PC-relative lowering (`bra`/`bsr`/`Bcc` to a label → PC-relative fixups).

---

## 3. Architecture & decisions

Discretionary calls made for this design are tagged **[D]** so they can be vetoed at review. The one call already made by the user is tagged **[user]**.

### 3.0 The pipeline

```
Module (may contain size-variable jmp/jsr fragments + symbolic fixups)
   │
   ├─ resolve_layout(module, map)         ← NEW: bounded fixpoint; picks jmp/jsr widths,
   │                                          lowers size-variable frags to concrete DataFragments
   │      → ResolvedModule (Data/Fill/Reserve only — link()'s invariant restored)
   │
   ├─ link(resolved, stubs)               ← EXISTING, extended: resolves 68k fixups
   │      → LinkedImage (per-section bytes @ LMA)
   │
   ├─ emit_rom(image, map)                ← NEW: map-ordered single image + gap fill,
   │                                          convsym no-op (append nothing),
   │                                          HeaderChecksum as the final pass
   │      → Vec<u8>  (s4.bin)
   │
   └─ emit_listing(module, layout)        ← NEW: s4.lst for Oracle + s4budget
          → String (s4.lst)
```

### 3.1 External memory map — `MemoryMap` **[D]**

A first-class linker input, mirroring how `SymbolTable` is a first-class input today.

- **Type in `sigil-ir`** (pure, no I/O), consumed by `sigil-link`. **Thin TOML loader** (`serde`+`toml`) in `sigil-link` (or `sigil-cli`); the pure type never depends on the parser.
- Declares **regions** `{ name, lma_base, size, kind: Rom|M68kRam|Z80Bank, vma_base }`, the **ROM output ordering**, and the **default gap-fill byte (`0x00`** — matches `p2bin` invoked without `-p`).
- **Jobs in B:** (a) the single-image walk order + default fill; (b) the ROM's total extent/terminus; (c) validate no section escapes its declared region (a mis-placed LMA otherwise silently shifts every later byte — padding is globally OFF in Aeon, so there is no slack). VMA≠LMA phased relationships are already carried per-`Section` (`vma_base`); the map records them for validation and for C to populate later.
- In B, the **harness/tests construct the map** (or load a test `sigil.map.toml`); the real `sigil.map.toml` beside `aeon/build.sh` is wired by C. B ships a `sigil.map.toml` matching the current Aeon layout as the canonical example.

### 3.2 Layout + the bounded fixpoint — `resolve_layout` **[user: layout-time, fixpoint-capable]**

The **only** length-variable fragment is bare-symbol `jmp`/`jsr`. Model it as a minimal size-variable fragment carrying `{ is_jsr, target: Expr }`. `resolve_layout` runs the spec's §5.4 loop:

```
sizes := all size-variable frags guessed at their minimum (abs.w)     // grow-only lattice
loop (hard cap 64):
    layout: assign every fragment a provisional VMA/LMA using current sizes
    for each size-variable frag:
        w := asl_width_rule(resolved target VMA, -A)     // .w (4 bytes) | .l (6 bytes)
        sizes[frag] := max(sizes[frag], w)               // monotone: never shrink → terminates
    if no size grew: break
if still growing after cap: hard diagnostic naming the frag/symbol, exit non-zero   // A5
```

On convergence, **lower each size-variable fragment to a concrete `DataFragment`** (chosen opcode word `4EF8/4EF9` or `4EB8/4EB9` + an `Abs16Be`/`Abs32Be` operand fixup), yielding a `ResolvedModule` whose fragments are only `Data`/`Fill`/`Reserve`. **This keeps the existing `link()` and all `Section`/`Fragment` invariants (`image_len`, `image_bytes`, …) untouched** — the size-variable type exists only *between* the front-end and this stage and is lowered away here.

- **Branches are NOT in this loop** — Aeon pins `.s`/`.w` explicitly, so branch fragments are fixed-length `Data` with a PC-relative fixup (§3.3). The loop is a genuine no-op for everything except `jmp`/`jsr`, exactly as the spec promises.
- **Monotone growth guarantees termination, not asl-equivalence** (§5.4 honesty note). Equivalence is guaranteed instead by matching asl's *per-site width rule* (§3.4) — proven by golden vectors, not trusted from the fixpoint.

### 3.3 68k fixup resolution — extend `apply_fixup`, mirror the Z80 pattern **[D]**

The Z80 backend already uses the deferred-resolution idiom: emit placeholder bytes + a `Fixup` the linker patches after layout (`Z80JrRel8`, `BankPtr16Le`). The 68k backend mirrors it with **inherent `lower_*` methods** on `M68kBackend` (A left `M68kBackend` handling only fully-resolved forms). New/implemented `FixupKind`s (each with a `byte_width`):

| FixupKind | Bytes | Value written | Reference point |
|---|---|---|---|
| `Abs16Be` *(exists; implement)* | 2 | target VMA as BE `u16`, sign-checked to fit `abs.w` semantics | — |
| `Abs32Be` *(exists; implement)* | 4 | target VMA as BE `u32` | — |
| `PcRel8` *(new)* | 1 | `target − (site_vma + 1)` as `i8`; reject `0` (that's the `.w` form → asl error) | branch: in-opcode disp byte |
| `PcRelDisp16` *(new)* | 2 | `target − site_vma` as BE `i16` | the extension word's **own** VMA |
| `PcRelDisp8` *(new, if corpus needs it)* | 1 | `target − site_vma` as `i8` | brief-ext word's own VMA |

**The 68000 PC-relative reference rule (load-bearing, documented like `Z80JrRel8`):** the effective PC value is the **address of the extension word containing the displacement**. Hence for all extension-word displacements (`bra.w`/`bsr.w`/`Bcc.w`, symbolic `(d16,PC)`, `(d8,PC,Xn)`), `disp = target − site_vma` where `site_vma` is the VMA of the displacement bytes themselves. The one exception is the 8-bit branch whose disp lives in the *opcode* word's low byte (`bra.s`): PC ref is `op+2` but the byte sits at `op+1`, so `disp = target − (site_vma + 1)`. Both forms are unit-tested against `asl` output.

`PcRelDisp16`/`PcRelDisp8` also cover the M1.A **target→disp** inbox item: the encoder emits placeholder disp bytes for a symbolic `(d16,PC)`/branch; B patches the resolved displacement post-layout. Out-of-range or unresolved ⇒ hard diagnostic with span + provenance (existing `apply_fixup` diagnostic path).

### 3.4 `jmp`/`jsr` width selection rule (§5.6, `-A`) — oracle-derived **[D]**

`asl_width_rule(target_vma, dash_A)` returns `Abs16` or `Abs32`. Because `asl` source is not in-tree, the rule is **derived empirically and pinned as golden vectors**: assemble `jmp label` / `jsr label` with `label` placed across the signed-16-bit boundary and beyond `$8000`/`$FFFF`, **with and without `-A`**, under the exact ASFLAGS; record which width `asl` chose; implement to match. The near-miss classifier (§8.3) surfaces any mismatch to the byte. This is a *named deliverable*, gated by vectors — never an assumed default.

### 3.5 Single-image ROM output + `convsym` no-op — `emit_rom` **[D]**

Extend today's `flatten`/`flatten_checked` into a map-aware `emit_rom(image, map) -> Vec<u8>`:
- Walk regions in map order, place each `LinkedSection`'s bytes at its LMA, fill internal gaps + head with the map default (`0x00`). Reuse `flatten_checked`'s overlap guard.
- **`convsym` = append nothing.** No in-ROM symbol table is ever emitted (emitting one breaks A1). A test asserts the image ends at the source terminus with **no trailing bytes**.
- No `.p`/`.h` intermediates; no power-of-two padding; the ROM ends where the source terminus places it.

### 3.6 Header checksum — final pass (`fixheader` replacement) **[D]**

Modeled conceptually as `FixupKind::HeaderChecksum` but implemented as a **post-image pass** in `emit_rom`, run **after every other byte is placed (including any future appended bytes)** — currently `convsym` appends nothing, so it covers the pure image. Algorithm (verified §2): BE 16-bit additive word-sum over `[$200, EOF)`, written BE at `$18E`.
- **Constants are build-pinned, not hardcoded.** The harness asserts `emit_rom`'s checksum equals the stored `$18E` word of the *pinned* `ref_s4.bin` — whatever that build's length/checksum are.

### 3.7 `s4.lst` emitter — `emit_listing` **[D]**

Emit an AS-`-L`-compatible listing whose **symbol-table section** satisfies `s4budget.py` and whose format satisfies the **actual Oracle load path** (verify which: `LoadFromAsListing` vs `Symbols.cpp::ParseLineHeader` — plan-time item). Scope is **only** the columns those parsers consume — symbol name (`[A-Za-z0-9_.]`), 24-bit hex address/value, the `C`(code)/`-`(equate) type marker, the `|` entry separator, the `Symbol Table (* = unused):` header, and the `N symbols` terminator. Cosmetic listing fidelity (pass markers, per-file line counters, macro-depth, `=>FALSE` markers) is **out of scope** (§7.3). The M1.d gate proves both tools accept it and resolve a known symbol identically to `asl`'s `s4.lst`. **Not** `convsym`-compatible (different parser).

---

## 4. IR changes (minimal, enumerated)

- `sigil-ir::FixupKind`: add `PcRel8`, `PcRelDisp16`, `PcRelDisp8`, `HeaderChecksum`; implement `byte_width` for each. Implement the currently-stubbed `Abs16Be`/`Abs32Be` in `sigil-link::apply_fixup`.
- `sigil-ir`: a minimal size-variable fragment representation for bare-symbol `jmp`/`jsr` (`{ is_jsr, target }`), used only pre-`resolve_layout` and lowered away there — so `Fragment`'s existing helpers stay total over `Data`/`Fill`/`Reserve`. (Exact type plumbing — new `Fragment` variant vs a small pre-link wrapper — decided in the plan; the design constraint is that `link()` never sees it.)
- `sigil-ir::MemoryMap` (§3.1).
- `sigil-backend-m68k`: inherent `lower_*` methods producing the size-variable `jmp`/`jsr` fragment and the symbolic-branch / PC-relative-EA fixups (mirrors `Z80Backend::lower_rel`/`lower_abs16`). The M1.A `lower` (fully-resolved forms) is untouched.
- `sigil-link`: `resolve_layout` (fixpoint), 68k `apply_fixup` arms, `emit_rom` (map + convsym-noop + checksum), `emit_listing`.

The crate-graph one-way rule (§9.1) is preserved: `sigil-isa` stays zero-workspace-dep; `sigil-link` depends on `sigil-ir` (+ backends via traits) and now `serde`/`toml`; nothing new depends on `sigil-frontend-as`.

---

## 5. Test strategy (asl-oracle TDD — the A/M0 pattern)

The `asl` oracle is the spec; reuse the `gen_*_vectors` generator pattern.

- **Fixup corpus (golden):** hand-built 68k IR modules exercising `Abs16Be`/`Abs32Be`, `PcRel8`/`PcRelDisp16`/`PcRelDisp8`, and target→disp — each diffed against the bytes `asl` produces for the equivalent snippet. Includes range-edge and out-of-range (must diagnose, not wrap) cases.
- **Width-selection corpus (golden):** `jmp`/`jsr label` across the `abs.w`/`abs.l` boundary, `-A` on/off, byte-matched to `asl`.
- **Multi-section layout:** cross-section fixups resolve to correct phased VMAs (extends the existing `link()` tests); overlap/region-escape diagnosed.
- **Checksum:** `emit_rom` reproduces the pinned `ref_s4.bin`'s `$18E` word over its real `[$200,EOF)`.
- **`s4.lst` (M1.d):** Oracle loads Sigil's `s4.lst` and resolves a known symbol to the same address as `asl`'s `s4.lst`; `s4budget.py --summary` runs clean. Harness asserts the reference `.lst` and `.bin` come from the **same** `asl` invocation (stale-listing hazard).
- **Fixpoint guardrail:** a synthetic oscillating input hits the pass cap and produces the non-convergence diagnostic (exit non-zero), demonstrating A5.

---

## 6. Reference pinning & hygiene

- **Pin the aeon reference commit** used for `ref_s4.bin` + `ref s4.lst`; record it in `PROVENANCE.md`. Length/checksum flow from that pin (currently 458666 / `$5CBE`), asserted by the harness — never hardcoded. Re-baseline deliberately.
- **[D] Add CI** early: a GitHub Actions job running `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and the crate-graph assertion. Commit the `Cargo.lock` entry for any new dep (`serde`/`toml`) — M1.A missed a lockfile entry; don't repeat it.

---

## 7. Out of scope (deferred to C / D)

- **All AS front-end fidelity** (`.ATTRIBUTE`/`pbyte`, `struct`, `function`, `strstr`, float folding, operator quirks) — **C**. B takes hand-built resolved IR.
- **Full-ROM `sha256` identity + stub-table deletion** — **D**.
- **General relaxation / a full `InstCandidate`/`ChosenSizes` lattice** — YAGNI; Aeon pins everything but `jmp`/`jsr` width, which the minimal fixpoint covers.
- **Human-readable `.lst` fidelity** beyond the symbol columns the two tools consume; **`s4.h`/`-shareout`** as a user artifact; **in-ROM `convsym` table** (D7).
- **Decode/disassembly.**

---

## 8. Acceptance (M1.B gate)

1. `cargo test --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` clean.
2. Multi-section layout with cross-fixups byte-correct; region-escape/overlap diagnosed.
3. `Abs16Be`/`Abs32Be`/`PcRel8`/`PcRelDisp16`/`PcRelDisp8` + target→disp byte-match `asl` across the fixup corpus, incl. out-of-range diagnostics.
4. `jmp`/`jsr` width selection byte-matches `asl` (with/without `-A`) across the width corpus.
5. `emit_rom` reproduces the pinned `ref_s4.bin` checksum at `$18E`; appends nothing (no trailing table); no power-of-two padding.
6. **M1.d:** Sigil's `s4.lst` loads in Oracle (known symbol resolves identically to `asl`'s) and `s4budget.py --summary` runs clean.
7. Fixpoint cap + non-convergence diagnostic demonstrated on a synthetic oscillating input.
8. Crate-graph rule intact (`sigil-isa` zero-workspace-dep; nothing depends on `sigil-frontend-as`); CI green; `Cargo.lock` updated.
