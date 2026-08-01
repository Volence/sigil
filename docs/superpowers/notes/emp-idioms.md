# `.emp` authoring idioms

A running reference of `.emp` conventions that are coherent-by-design and worth
knowing, but are not (yet) language changes. Ruled during the campaign; each
entry names the demand that surfaced it. This is a NOTES doc, not the spec —
SPEC2 stays v1-frozen; spec consolidation is a separate future pass.

## Data-label references: bare = cross-module, `extern("…")` = same-module

In a DATA initializer (a `data`/`offsets` cell, a struct-literal field, a call
argument in value position), a link-label reference is spelled two ways, and
which one you use depends on WHERE the label is defined:

- **A cross-module label → a BARE identifier.** A name that this module does not
  define resolves as a deferred link symbol: the frontend records the NAME by
  shape and leaves it for the linker. Example — a jump/offset table whose targets
  live in other modules:

  ```
  // engine.player.offsets — targets defined in sibling player modules
  pub offsets Player_States {
      Idle:    Player_Idle,      // bare: cross-module link label
      Walk:    Player_Walk,
      Jump:    Player_Jump,
  }
  ```

  The `offsets` construct emits each row as `dc.w target - Player_States`, exactly
  the `extern("target") - extern("Player_States")` difference form by hand — the
  bare cross-module target is accepted like any local label and folded at link
  (see `2026-08-02-l9-offsets-cross-module.md`).

- **A SAME-module label → `extern("Name")`.** A bare identifier naming a label
  defined in THIS module fails `unknown name` (a data initializer does not
  resolve local labels as bare barewords); spell it `extern("Name")` so it
  resolves as a forward/back link reference:

  ```
  // A page table pointing at blobs defined below it in the same module
  pub data OJZ_Act_Pool_PageTable: [*u8; 3] = [
      extern("OJZ_Act_Pool_Page0"),   // same-module: extern("…")
      extern("OJZ_Act_Pool_Page1"),
      extern("OJZ_Act_Pool_Page2"),
  ]
  ```

**Why the asymmetry is coherent, not a wart.** A bare name is the natural
spelling for "a symbol I don't define — resolve it at link"; `extern("…")` is the
explicit "I mean the label, not a value" marker for a same-module symbol whose
name would otherwise be an unresolved local. The rule is one sentence and reads
the same at every site. (Surfaced by K3 run A — `2026-08-01-k3-run-a.md` — which
hit it on the `OJZ_Act_Pool_PageTable` page pointers and a block-blob dedup alias;
ledgered as L11 and DOCUMENTED-AS-IDIOM by the language round,
`specs/2026-08-02-language-round-agenda.md` Tier 3. Revisit as a grammar change
only if the asymmetry keeps biting.)
