# Does engine code nothing references ship in the cartridge?

The owner's trailing question, verified verbatim at empyrean `origin/main` `20854d7`,
`docs/OVERSEER.md:177`, 2026-09-12T22:16:45Z: *"We discussed this where we can have code that
doesn't reach anything like this but it doesn't get bundled in at compile right?"* The hub put it to
aeon (engine structure) and to this lane (assembler and linker), with the instruction that it **must
not be answered by assumption**.

**This note answers the ASSEMBLER AND LINKER half only.** Whether a given mechanism can be
structured to sit where the answer below makes it free is aeon's half, and nothing here rules on it.

## The answer, in one line

**Broadly yes, and the granularity is the MODULE.** A module that nothing in the game's profile
`use`s is never resolved, never lowered, and emits nothing: it costs zero bytes. **But there is no
finer elimination than that.** Once a module IS in the closure, everything it emits ships, whether
or not anything calls it.

So the owner's instinct is right, and it comes with one condition he has not been told: **the
mechanism has to live in a module the game does not `use`.** Leaving it in a module the game already
imports costs its full size in every ROM.

## What decides inclusion, from source at master `547c13ec`

**The profile's `use` closure**, not directory membership and not section placement.

`crates/sigil-frontend-emp/src/resolve/imports.rs:525` states it while explaining an unrelated lint:
the `use` edge alone *"pulls a zero-emitting guard module into the profile's use closure, which is
what makes its module-level `ensure`s elaborate"*. A module reached by no `use` edge is not in the
closure and is not built.

**The corroborating design fact is `--extra-entry`, and it is stronger than the comment.**
`crates/sigil-cli/src/main.rs:2470`: it names *"a module the build must evaluate although nothing
`use`s it, so its module-level `ensure`s run inside the real profile"*, and **"a NAMED module that
would emit is refused by name"**.

That flag only needs to exist because unimported modules are otherwise absent from the build, and
its refusal only needs to exist because reaching one must stay byte-neutral. **A capability built to
reach unbuilt modules without emitting is evidence that unbuilt is the default**, and it is better
evidence than any comment, because a comment can be stale and a flag has to work.

## Where it stops

**No per-item elimination exists.** `sigil-link`'s entry point is
`link(sections: &[Section], stubs: &SymbolTable)` (`crates/sigil-link/src/lib.rs:67`): it places the
sections it is handed. Searched `crates/sigil-link/src` and `crates/sigil-frontend-emp/src/lower`
for any skip-because-uncalled and found none, against controls confirming both paths are greppable
(591 files match `fn ` under `crates`, 8 under `lower/`).

The one place `unused` appears is `crates/sigil-link/src/listing.rs:4` and `:20`, and it is a
**reporting** feature, not an elimination one: the asl-compatible listing prints
`Symbol Table (* = unused):`. **A symbol can be marked unused in the listing and still be in the
ROM**, which is exactly the shape that would mislead someone checking this question by reading a
listing.

## What this means for his ruling, and why it needs no new capability

He has ruled that two test effects leave the playable map but stay in the engine, injectable
anywhere. **If each mechanism lives in its own module that no game module `use`s, that ruling is
already free**, and the cost of "inject it later" is one `use` line at the moment he wants it.

**So the follow-on the hub offered to have priced does not arise.** No language capability, linker
feature, build-time gate or source-level conditional is needed for the case he asked about. The
capability exists and it is spelled `use`.

**The residual worth stating, because it is the half that can surprise him:** this is an
all-or-nothing edge. There is no way to import part of a module, so a mechanism sharing a module
with something the game does use is not separable by any flag. If the two effects currently sit
inside a module the game imports, making them free is an aeon-side **file split**, not an assembler
change. That is the aeon half of the question and this lane does not rule on it.

## Method note, kept because it nearly went wrong

The first search for elimination machinery used the pathspec `crates/*/src` and returned a clean
zero. **That pathspec matches NOTHING in this repo** (`git grep -l 'fn ' -- 'crates/*/src'` is 0,
against 591 for `crates/`), so the zero was vacuous and would have been reported as "no liveness
machinery exists" on the strength of a search that could not have found any. It was caught by a
control run in the same breath, not by suspicion.

**Two earlier claims rested on that same pathspec and are corrected in place:** see the scope
paragraphs in `crates/sigil-harness/golden/ab/AB_PROTOCOL.md` and in the
`AB-PROTOCOL-CART-UNVERIFIED` ledger row. Their CONCLUSION survives re-measurement with a working
pathspec (zero bus-client users outside `golden/ab/`), but the evidence as published was partly
vacuous, and **a right conclusion does not launder the evidence offered for it** — this document's
own banked rule, arriving on the seat that wrote it.
