# aeon parallax round 2 (c06c786b): what it asks of sigil at the next pin advance

Received from the aeon lane 2026-09-28, verified: `c06c786b` is an ancestor of aeon
`origin/master` (merge of `dbcd1d92`, branch `perf/parallax-perband-2`).

Nothing to do while the reference pin stays at `ec640bcf`: the port tests and strict
gates read that revision, not aeon tip. When the pin advances past `c06c786b`:

- `engine/level/parallax.emp` and `engine/ram.emp` change; `Parallax_State` grows by
  24 B at its tail in sonic4 (828 to 852), demo unchanged.
- New cross-module names the `*_port` dep lists may need (`crates/sigil-cli/tests/parallax_port.rs`,
  `bg_port.rs`, `buffers_port.rs` already reference `Parallax_*`): `Parallax_Curve_Walk`,
  `Parallax_Band_Sel`, `Parallax_Band_Sel_Valid`, `pub const BAND_SEL_N`.
- aeon states the `Decode_Factor_A/B` call sites our baseline pins are untouched. Their
  claim, not re-measured here.
- aeon's canonical figures at that merge: s4 `f52861a3/830971`, s4.debug `24b22fa8/858024`
  (their measurement, built on sigil `1173bb31`).

Falsifier at advance time: `git -C <aeon> merge-base --is-ancestor c06c786b <new pin>`; if
true, run the three port tests above strict against the new pin before anything else.
