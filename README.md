# Sigil

A from-scratch, native-Rust assembler and systems language for the Sega Genesis /
Mega Drive (Motorola 68000 + Zilog Z80). Sigil is the assembler engine of
**Crucible** and the toolchain of the [Aeon](https://github.com/Volence/aeon) game
engine, which it builds end to end — no Wine, no external assembler.

Sigil was built under one constraint: **byte-for-byte fidelity to the AS macro
assembler (`asl`)** — the exact bytes AS produced for the Aeon sources, bug-for-bug
where AS has quirks, so the migration off AS was provably a no-op at the ROM level.
Its instruction encoders are still measured that way, against committed **golden
vectors minted by real `asl`**.

## Status

Sigil has two tracks.

### The backend track — the assembler that replaced `asl`

| Milestone | Scope | State |
|---|---|---|
| **M0** | IR + full Z80 backend + AS front-end (Z80 subset) + byte-exact A+B harness | ✅ complete |
| **M0.5** | Procedural-EA spike: `sigil-isa::m68k` MOVE encoder | ✅ complete — all 22 MOVE EA-matrix forms byte-identical to `asl` |
| **M1** | Full 68000 backend (A) + full linker (B) + AS 68k front-end fidelity (C) + full-ROM byte-exactness (D) | ✅ complete — the whole assembled ROM byte-identical to `asl`, in both the **plain** and **`__DEBUG__`** shapes |

The M0.5 spike retired the risk in the 68000's irregular effective-address /
extension-word encoding (the [byte landmines](#the-asl-oracle-discipline)) before M1
committed to the full backend.

> **What those rows mean today.** Each was earned against a live `asl` reference.
> `asl` has since left the pipeline — Sigil *is* the build, since the Spec-5
> Stage-2 flip — and is no longer installed in the repo. So the whole-ROM gates
> that run today compare against a **Sigil-built committed golden**: they prove the
> build still reproduces the frozen bytes, not that those bytes agree with a second
> assembler.
>
> The evidence that is independent of Sigil's own output is the **ISA-level golden
> corpus** (`sigil-isa`'s encoding vectors, frozen from a real pre-flip `asl` run),
> the **Capstone differential** gates (the whole 16-bit opcode space, plus every
> shipped shape's emitted stream), and the `*_port` gates that compare Sigil's two
> front ends against each other.

### The language track — `.emp` (Spec 2)

`.emp` is Sigil's own surface language for the engine: modules, comptime evaluation,
a type layer, and quoted asm templates. It is specified in
`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`, and Aeon is being ported onto it module by
module.

`sigil build --aeon <dir>` is the shipped Aeon build: it lowers every `.emp` module
natively, assembles the game's residual `.asm` root, chain-links, folds the
checksum, emits the Sigil-canonical `.lst`, and appends the `convsym` deb2 symbol
table — the complete shipped file, not just the ROM image.

## Workspace layout

Sigil is a Cargo workspace with a **strictly one-way crate graph** (enforced by the
`crate_graph` test), so the AS front-end is a cleanly deletable unit and the ISA
encoders stay dependency-free and extraction-ready.

| Crate | Responsibility |
|---|---|
| `sigil-span` | Source ids, spans, provenance, string interning |
| `sigil-ir` | `Module`/`Section`/`Fragment`/`Expr`/`Fixup`/`SymbolTable`; the `Backend` + `IrStreamer` traits |
| `sigil-isa` | Instruction encoders/decoders (`z80`, `m68k`). **Zero workspace deps** — extraction-ready for shared use with the emulator core |
| `sigil-backend-z80` | Thin `Backend` adapter binding `sigil-ir` to `sigil-isa::z80` |
| `sigil-backend-m68k` | The same for `sigil-isa::m68k`, including the deferred-target forms (branch / PC-relative / `jmp`+`jsr` fixups) |
| `sigil-link` | VMA≠LMA layout, fixup resolution, image flattening (the linker) |
| `sigil-frontend-as` | The quarantined AS-syntax oracle front-end (lexer/parser/eval/macros/multi-pass). Nothing depends on it except the CLI and harness |
| `sigil-frontend-emp` | The `.emp` front-end: lexer, parser, AST, comptime evaluator, and IR lowering |
| `sigil-s4lz` | Pure-Rust S4LZ v3 encoder, backing the `.emp` comptime `s4lz()` builtin |
| `sigil-salvador-sys` | Safe wrapper around the vendored `salvador` ZX0 compressor |
| `sigil-clownlzss-sys` | Safe wrappers around vendored `clownlzss` (Kosinski, Kosinski+, Saxman, Enigma, Comper, Rocket, …) |
| `sigil-clownnemesis-sys` | Safe wrapper around the vendored `clownnemesis` compressor/decompressor |
| `sigil-cli` | The `sigil` binary (`build` / `emp` / `parse` / `test`) plus `emp_census`, the `.emp` register-contract census tool |
| `sigil-harness` | The whole-ROM native build driver, the sound-blob seams, the generated layout pins, the golden provenance chain, and the byte-exactness gates |

## Build & test

```bash
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Both flags matter, and for different reasons. Clippy's lints are *warnings*, so a bare
`cargo clippy` prints every finding and still exits `0` — `-D warnings` is what gives the
command a verdict. `--all-targets` is what lets it see the test and bench targets, which
is where most of this workspace's code lives. `scripts/landing-run.sh` runs this exact
form as precondition (7), so a landing run cannot report green over a red lint bar; you do
not need to remember it separately.

The full suite runs in release, without fail-fast, against an Aeon reference tree:

```bash
AEON_DIR=/path/to/aeon SIGIL_STRICT_GATE=1 \
    cargo test --release --workspace --no-fail-fast
```

- `AEON_DIR` names the Aeon tree the port gates read
  (`crates/sigil-harness/src/test_support.rs`); it falls back to a sibling `aeon`
  checkout. Gates that read built artifacts need that tree's ROMs built.
- `SIGIL_STRICT_GATE=1` turns a missing-reference *skip* into a failure, so a gate
  cannot pass by measuring nothing. This is the pre-merge bar.
- Without `--release` some gates are impractically slow; without `--workspace
  --no-fail-fast` an early failure hides the rest of the result set.

`sigil-cli` ships two binaries, so `--bin sigil` is required:

```bash
cargo run -p sigil-cli --bin sigil -- --version
cargo run -p sigil-cli --bin sigil -- <input.asm> [-o <out.bin>] [--hex]
cargo run -p sigil-cli --bin sigil -- parse <input.emp>
cargo run -p sigil-cli --bin sigil -- emp   <input.emp> [--root <dir>] [-o <out.bin>] [--hex]
cargo run -p sigil-cli --bin sigil -- test  <input.emp> [--root <dir>]
cargo run -p sigil-cli --bin sigil -- build --aeon <dir> [--game sonic4|demo] [--debug] [-o <out.bin>]
```

A bare `sigil <input.asm>` assembles one AS-syntax source; `emp` does the same for
`.emp`, single-file or `--root`-rooted multi-module. `build` also takes
`--report ram` / `--report contracts`, which print the target's RAM map or contract
closure instead of building. `sigil --version` reports the revision the binary was
built from — the only way to tell a current assembler from a stale one, since byte
identity cannot.

## The asl-oracle discipline

Correctness is proven, not asserted. For each backend, a canonical **corpus** of
instruction snippets was assembled by real `asl` (+ `p2bin`) into committed
golden-vector files; CI encodes each corpus form and asserts the bytes match
byte-for-byte. CI never needs `asl` — the vectors are committed, the **frozen
independent-asl witness**. Regeneration is a manual developer step:

```bash
ASL_BIN=/opt/asl/bin/asl cargo run -p sigil-isa --bin gen-z80-vectors    # Z80 golden vectors
ASL_BIN=/opt/asl/bin/asl cargo run -p sigil-isa --bin gen-m68k-vectors   # 68000 golden vectors
ASL_BIN=/opt/asl/bin/asl cargo run -p sigil-frontend-as --bin gen_snippet_vectors
```

**`asl` is out-of-repo (Stage-3 P4d / OQ-A).** At the flip the `asl`/`p2bin`
binaries were removed from the toolchain (nothing-retained: `sigil build` IS the
build). The committed vectors stay valid forever; EXTENDING the corpus for a new
post-flip instruction shape requires the public [Macro Assembler
AS](http://john.ccac.rwth-aachen.de:8000/as/) installed out-of-tree — point
`ASL_BIN` (and, if not a sibling, `P2BIN_BIN`) at the binaries. The generators fail
loud if `ASL_BIN` is unset — never a silent skip.

The 68000 encoder pairs a declarative fixed-field opcode table with a **procedural
effective-address / extension-word encoder** — the 68000's addressing modes do not
fit a pure table — and pins the real byte landmines with dedicated vectors: the
`MOVE` destination-EA mode/register field swap, the brief extension word,
absolute-width selection, `MOVEM` `-(An)` mask reversal, and 2-wide branches.

## Design docs

- Cross-tool contract & specs live in the sibling `empyrean/docs/` tree:
  `SIGIL_SPEC2_LANGUAGE.md` (the `.emp` language), `SIGIL_CORE_SPEC.md`,
  `SIGIL_M0_DESIGN.md`, `SIGIL_M0_CATALOG.md`, `SIGIL_AEON_COMPAT_NOTES.md`,
  `SIGIL_ORACLE_ISA_SHARING.md`. Read them at a committed revision
  (`git -C ../empyrean show origin/main:docs/<file>`) — that sibling is a live
  working tree.
- `docs/OVERSEER.md` — how a session runs this repo: quality bars, the landing-lane
  division, the worktree quirks, and the queue.
- Per-feature design + implementation plans, packets and notes are under
  `docs/superpowers/`.
