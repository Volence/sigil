# Assembler-side diagnostic instrumentation — idea capture 2026-07-20

Status: **idea capture**, not committed design. These are things the assembler can do for
debugging that a drop-in library (vladikcomper's Error Handler + `convsym`) structurally
cannot, because they require the *assembler* to emit or know something at build time.

## Framing (shared across three repos)

We use vladikcomper's Error Handler/Debugger (+ `convsym`, which we already run in the
build) as our one significant not-from-scratch tool. As a *drop-in library* it can only
assume a symbol blob: it maps a crashing PC to the nearest label, nothing more, because a
generic user's assembler gives it nothing else. We control sigil end to end, so we can emit
richer debug metadata and build-time-injected instrumentation that no drop-in can.

On-target pieces live in `aeon/docs/DEFERRED_WORK.md`; emulator/bus-level pieces in
`oracle-next/docs/2026-07-20-diagnostic-tooling-ideas.md`.

## Ideas

- **Source line tables (DWARF-lite): PC → `file:line`, not just PC → symbol.** vladikcomper's
  handler gives "somewhere after `Obj_Update`" because a symbol table is all it has. sigil
  can emit a real line table so a crash resolves to the exact source line, and Oracle can
  render source context around any PC. This is the single most useful piece of debug
  metadata and it's purely a sigil output-format addition (consumed by Oracle via MCP).

- **Contract-enforcement instrumentation (sigil half; aeon holds the trap handler).** The
  aeon codebase is retrofitting honest In:/Out: register contracts (its `contract-grammar`
  work). sigil knows each routine's contract, so a DEBUG build can auto-inject shadow-state
  capture on entry and a verify on return — trapping the exact instant a routine clobbers a
  register it promised to preserve or returns garbage in a promised `Out:`. This is a class
  of bug currently invisible until it corrupts something several calls later. No drop-in can
  do it: it needs the assembler and the trap handler to share a contract vocabulary — which
  we're building from scratch anyway, so this just cashes in the prerequisite.

- **Call-site tagging for exact backtraces.** Emit metadata marking real call sites so
  Oracle can reconstruct an exact 68K call stack instead of the heuristic
  return-address-shaped-word scan the drop-in handler is forced into (68K has no
  frame-pointer convention). Pairs with the Oracle-side call-stack reconstruction idea.

- **Build-time invariant checks / instrumentation injection.** Some runtime `assert`s can be
  proven false (or true) at assembly time and rejected outright; a DEBUG build can also
  auto-insert profiling markers and the contract checks above without touching source, so
  release builds carry zero overhead.

## Note on repo boundaries

sigil's language/contract *specs* live in `empyrean/docs/SIGIL_*`; this file is idea capture
kept with the sigil repo per "jot them in their respective repos." Promote to a real spec in
empyrean when any of these graduates from idea to committed design.
