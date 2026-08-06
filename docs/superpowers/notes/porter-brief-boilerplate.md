# Porter brief boilerplate (prepend to every lane brief)

## HARD RULE, FIRST AND NON-NEGOTIABLE: FOREGROUND SHELLS THROUGH FINAL REPORT

Every shell command — builds, the strict suite, capture_goldens.sh, everything —
runs in the FOREGROUND of your session, to completion, with output captured. No
`&`, `nohup`, `setsid`, detached/background shells, or fire-and-forget runs for
ANY command, especially the final strict run. If a command may exceed your shell
timeout, raise the timeout (up to 600000 ms) or split into foreground chunks
(e.g. per-crate test invocations) — never detach. Your turn is not done until
you have READ the final output of your final gate run and written your report.
(Last two arcs: detached shells stranded 4 porters; the rule cut it to 1.)

## Repos, worktrees, hygiene

- sigil = /home/volence/sonic_hacks/sigil · aeon = /home/volence/sonic_hacks/aeon.
- Work ONLY in your pre-created `<repo>/.worktrees/<lane>` worktrees. NEVER edit,
  build into, stash, or checkout in the MAIN checkouts. Never `git add -u`;
  explicit paths; check your branch before every commit.
- Fresh aeon worktrees need TWO gitignored seeds: `games/sonic4/data/editor/` and
  `engine/debug/generated/` (enumerate with `git status --ignored --short |
  grep '^!!'` — the list has grown before). Prove the seed: build and compare
  byte-identical BEFORE any edit. Never reuse a warm `target/` across a worktree
  move — force a clean rebuild.
- `AEON_DIR` for corpus-walking tests = YOUR aeon worktree at the RIGHT commit.
  Masters move (other sessions + other lanes' merges); re-check before gates.
- capture_goldens.sh needs `SIGIL_EMIT` and `SIGIL_BUILD` set (a bare invocation
  dies silently). Byte-bar order = the script's order (config_a writes
  s4.debug.bin; config_b and lean both write s4.bin — out of order they clobber
  the canonical reference; the script restores canonical at the end).
- Never pipe cargo through tail/head (truncates + wrong exit code). Full capture
  to a file, failures-first, explicit pass/fail/ignore counts.

## Bars (all own-run, none waived)

1. Byte bar: seven targets against the current chain goldens — derive the target
   list from `crates/sigil-harness/golden/`, never assume a count. On a
   byte-NEUTRAL parcel, any byte moving = STOP and report. On a byte-CHANGING
   parcel, every delta must be named and explained, and the refreeze/A-B-ref
   discipline applies.
2. Full strict: `cargo test --workspace --release` (foreground!). Closing
   arithmetic: passed + ignored == your branch's own `#[test]` total
   (`git grep -c '^\s*#\[test\]' -- 'crates/**/*.rs'`); chase every delta to the
   named function.
3. `cargo run -q --release -p sigil-harness --bin refreeze -- --check` OK;
   repin unchanged unless bytes moved (then the FULL 5-site ripple:
   pins.rs + engine.inc + mixed_dac_rom.rs + repin_pins.rs, repin.toml if a
   region was added).
4. Warn tiers: firing lint-id SET identical ×7 unless the spec names a
   deliberate delta (baseline updated same parcel, delta named in the packet).
5. Negative probes both polarities for every new check; non-vacuity guards on
   any assertion that could pass by measuring nothing; revert probes where the
   spec asks.

## Process

- Verify each spec claim against the CURRENT tree before building it (7+ stale
  plan items caught to date); a wrong claim = STOP and report.
- Comments: present-tense contract facts only — no change-history narration, no
  parcel tags. Brace-indent house style. Ledger rows for honest gaps, with
  measurements, same commit as the code where possible.
- Behaviour changes isolated in their own commits with their own probes;
  mechanical refactors separate.
- DO NOT MERGE, ever. Gate-green → packet (per-pass step-3 vs step-5 findings +
  neither-bucket headlines; NO merge-state claims) → end your turn with the
  report. A read-only lens panel reviews before merge; expect a fixup round;
  the overseer owns the merge queue.
- If resumed after an interruption: re-verify tree state (status/branch/HEAD)
  before continuing.
