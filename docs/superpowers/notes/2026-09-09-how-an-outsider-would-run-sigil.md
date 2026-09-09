# How someone else would actually run sigil, as things stand today

Written 2026-09-09 for the owner, who asked, verbatim: *"How will sigil run in sonic 1 btw? Do we
make like a sigil.sh/sigil.exe or something and drop it in to the disassembly? Saying we want others
to use it"*

**Read this as a survey of what exists, not a plan. Where something is undecided it says undecided,
because inventing an adoption plan in an answer is how a plan nobody agreed to becomes the record.**

## 1. The invocation today

**One user-facing binary, `sigil`.** The other executables in this repo (`emit_sound_blob`, `repin`,
`refreeze`, the vector generators) are this project's own harness and would not ship to anyone.

For an AS-style tree the whole surface is one line, from `crates/sigil-cli/src/main.rs`:

```
sigil <input.asm> [-o <output.bin>] [--hex]
sigil --version
```

**No manifest, no config file, no source edits.** The Sonic 2 decomposition ran `cd <corpus> &&
sigil s2.asm` with no flags at all. Include paths resolve against the source file's own parent
directory, which is the same rule AS uses, so a tree that assembles under AS is laid out correctly
for us already.

**`-o` writes a flat binary image directly** (`main.rs:177`), so sigil replaces `asl` AND `p2bin` in
one step rather than producing an object file that a second tool converts.

Aeon's build uses a different surface, `sigil build --aeon . --native --lean -o lean.bin`, because
aeon is a `.emp` project rather than an AS tree. **An outsider with a disassembly never touches
that.**

## 2. What an outsider would physically do, and it is not a rename

**Correction to the premise: a modern Sonic 1 disassembly does not call `asw` and `p2bin` from
`build.bat`.** That file is gutted and says so in its own comment; it shells into `build.lua`. The
real work is in `build_tools/lua/common.lua`, which has `find_assembler` locating a tool named `asl`
under `build_tools/<os>-<arch>/`, and `assemble_file(input, output, as_arguments, p2bin_arguments,
...)` running the two-step pair.

So **dropping a binary in and naming it `asl` does not work**, and it is worth being precise about
why rather than saying "incompatible": their helper passes AS's flags (`-xx -n -q -A -L -U -E -i .`)
and then runs a separate `p2bin` step. We accept none of those flags and need no second step. The
swap is therefore **an edit to one Lua function**, replacing the pair with a single call, not a file
drop.

**What it is NOT is source changes.** Nothing about their `.asm` needs editing, and no project file
has to be introduced. That half of his question has a good answer.

**What blocks it today is not integration. Sonic 1 does not assemble yet**: 50 diagnostics in ten
classes, mostly missing directives. A perfect drop-in would fail on the first run.

## 3. Windows

**Nobody has tried, and I will not dress a prediction as a result.** There is no
`.cargo/config.toml`, no Windows target in any script, and no commit in this repository's history
that builds for one.

What is knowable without trying:

- **Nothing in the shipped code is platform-gated.** The only two `cfg(unix)` sites in the workspace
  are TESTS, for symlink containment, in `sigil-frontend-emp/src/eval/sandbox.rs`.
- **The external Rust dependencies are four**: `serde`, `serde_json`, `sha2`, `toml`. All portable.
- **The real consideration is two vendored native crates**, `sigil-clownlzss-sys` (C++) and
  `sigil-clownnemesis-sys` (C), built by `build.rs` and reached by the CLI through the `.emp` front
  end. They are plain compression code with no platform APIs, so a MinGW or MSVC toolchain should
  build them, **and "should" is exactly the word: it has not been done.**

**Honest statement: a Windows build looks like a cross-compile plus a C toolchain rather than a port,
and that stays a prediction until someone runs it.**

## 4. The honest minimum for a first outside user, in order

1. **The two games actually assembling.** Everything else is moot while the answer to "does it build
   my disassembly" is no. Sonic 1 is a short coverage list; Sonic 2 is one large blocker plus an
   unmeasurable tail.
2. **A Windows binary, built and run, not predicted.** The scene is largely on Windows.
3. **The invocation written down somewhere a stranger reads.** Today the entire user-facing surface
   exists as a usage string inside an error path.
4. **The Lua integration**, as either a patch they apply themselves or something offered upstream.

**Undecided, and deliberately left so:** how a binary would be distributed at all (releases,
versioning, where it is downloaded from), and whether we would grow AS-compatible flags so the swap
is a rename, or ask projects to change one build-script function. Both are real options with
different costs and neither has been chosen.
