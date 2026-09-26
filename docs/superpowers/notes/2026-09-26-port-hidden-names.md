# PORT-TESTS-HIDDEN-NAME-FAILURES: which gates can hide a name behind a byte failure

Branch `parcel/port-hidden-names`. Pin: aeon `ec640bcf` (`.aeon-sigil-ref`, the provenance
`aeon_rev`). Tip: aeon `9caa1368` (aeon `origin/master` at dispatch, held fixed), provisioned
for this parcel at `.aeon-hidden-tip`. Every run passed `AEON_DIR` and `SIGIL_STRICT_GATE=1`;
`CARGO_TARGET_DIR` under `.scratch/port-hidden-names/`, where every log named here lives.

## Verdict

The masking shape `ojz_run_b_port` had (compare one part, then lower the next) existed in
nine more tests across seven files. All nine now lower, link or build every part first and
compare after. At aeon `9caa1368` NONE of them was hiding a name: after the restructure every
one reports the same first failure it reported before, or stays green. The value of the
parcel is the structural fix and the census below, not a found name. Red-first mutations show
the old structure hides an injected name in all nine and the new one reports it.

The tip does carry NEW visible name failures (`Music_Service`, `Music_Want`, 12 tests), from
aeon commits after the previous parcel's tip `8a6f92c4`. They are not hidden, so they are
outside this parcel; they are listed under Open.

## The instrument

`2026-09-26-port-hidden-names/mask_scan.py` (a copy of the scratch tool, runnable from the
repo root). For every `#[test]` fn in the 474 Rust sources under `crates/*/tests/` (vectors
excluded; 4511 tests) it flattens the test into an ordered event stream, inlining the file's
own helpers and its `mod x;` siblings, unrolling loop and closure bodies twice:

- `P`: a call that lowers, links, assembles or builds (`lower*`, `link*`, `resolve_layout`,
  `place_sections`, `assemble*`, `compile*`, `build_*`, `emit_*`, `native_section*`,
  `resolve_canonical_sections`, `resolve_frozen_layout`) that is not a local fn;
- `C`: an `assert!`/`assert_eq!`/`assert_ne!`/`panic!` whose condition or message is not about
  diagnostics (panics inside `unwrap_or_else`/`expect` count as the producer's own report).

A test is FLAGGED when a `C` lies between two `P`s: a comparison can stop it after one unit
was produced and before another was. It is tagged AEON when its calls reach the aeon tree.

**Positive control.** `mask_scan.py --rev 78f5c7ca^ crates/sigil-cli/tests/ojz_run_b_port.rs`
flags both `ojz_run_b` tests (first P `lower_module`, compare "emitted NO BYTES", later P
`lower_module`); at HEAD the same file reports `flagged=0`.

**Limits, stated so they are not read as coverage.** It is lexical: calls into other crates
are atoms, so a harness helper that loops lower-then-compare internally is invisible from its
caller (a root scan of `native.rs`, `seam1.rs`, `seam2.rs` flags product build internals, the
first-error-class behaviour the previous note describes, not test gates). A comparison spelled
as a message-less `if` escapes `C`. The census is the candidate list, and every AEON flag was
then read by hand.

## Census and triage (66 AEON flags before, 60 after)

Workspace totals: 575 flagged before, 569 after; 509 of them inline-source unit tests, which
no aeon revision can change, so no aeon drift can surface a name there.

The population, read by hand, is a flag where a comparison that can fail on AEON DRIFT (the
reference ROM, a golden, `pins.rs`, a literal about aeon source) precedes the production of a
DIFFERENT unit (another module, the other shape, or the link). Nine tests in seven files:

| gate | masked part | status |
|---|---|---|
| `ojz_run_a_port` (both) | `ojz_act_pool` lowered after `entity_data` compared | fixed `a6691120` |
| `seam2_seq_colink::..._both_shapes` | debug co-link after plain golden compare | fixed `a6691120` |
| `seam2_sfx_head_colink::..._both_shapes` | debug co-link after plain golden compare | fixed `a6691120` |
| `soundbankhead_port::soundbankhead_pin_is_the_lma_not_the_vma` | debug program after plain pin compare | fixed `a6691120` |
| `game_debug_port::two_module_flip_resolves_debug_music_toggle` | the link after a placement compare | fixed `a6691120` |
| `seam2_phased_head` (both) | debug lowering after plain `$8000`/64-byte literals | fixed `79c1cd94` |
| `section_row_fixture::both_spellings_...` | debug full-file builds after the plain provenance CRC | fixed `79c1cd94` |

The last two were missed by the first hand triage and found when the tip run showed
`both_spellings` stopping on the plain CRC before any debug build; the rest of the 66 were
re-read for the same mistake after that.

The other 57, by class (each still flagged, none a member):

- **One aeon unit plus inline scaffolding**, the `C` a doctor or seam precondition:
  `boot_port`, `buffers_port`, `children_port`, `dma_queue_port`, `load_art_port` (region
  tests, 10), `test_p1_player_port` (3), `objdef_port` (2).
- **Both modules produced before the only comparisons**, the `C` a harness-table conflict
  check: the `two_module_*_flip` tests of `entity_window_port`, `plane_buffer_port`,
  `tile_cache_port` (6).
- **The same sources produced again** (control then doctored, ROM then full file, a map or
  size-table doctoring, a mirrored tree), so no new name set can appear: the
  `wrong_base_map` probes (`core`, `dplc`, `hblank`, `tranche2` x2; their `C` echoes the map
  base the test itself chose), `mt`/`sfx` `wrong_bank` probes (a `C` on pin arithmetic),
  `native_full_rom` (2), `native_offcanonical_full` (5), `native_offcanonical_placement`,
  `measure_at_packed_base`, `section_alignment_declared`, `listing_defines`,
  `error_handler_island_membership`, `tranche4`/`tranche5` doctored probes, `extra_entry`,
  `section_row_fixture`'s two NON-VACUITY probes, `colink_is_deterministic`,
  `hole_interior_reserved`.
- **Comparisons no repin could absorb** (self-consistency or tree-derived invariants), or no
  aeon unit at all: `anchor_overlay`, `m68k_capstone_stream`, `m68k_roundtrip_stream`,
  `sound_bank_id_check`, `emit_seq_artifacts`/`emit_sfx_artifacts` (file equals in-memory),
  `offcanon_assembled_bar` (its `P` reads a frozen table), `emp_cpu_attr_spelling` (lowers
  fixtures), `scene_registry_port` (2; a one-section precondition before the link, and at the
  tip that link is reached).

Two members stay flagged after the fix, by premise checks inside production, not comparisons:
`ojz_run_a` (the ObjDef seam's "names no ObjDef_" check inside `compile_section`) and
`both_spellings` (the fixture's nonempty-section and one-spelling checks on the live build).

## The tip, gate by gate (first failure; before `7bd90670`, after `79c1cd94`)

| gate | before @tip | after @tip | hidden name |
|---|---|---|---|
| `ojz_run_a` plain, debug | bytes: `entity_data` must emit exactly 0x188 | same | none |
| `seq_colink` both shapes | green | green | none |
| `sfx_head_colink` both shapes | length: sfx_bank body is 2284 bytes | same | none |
| `soundbankhead` pin test | green | green | none |
| `game_debug` flip | NAME, visible: unresolved branch target `Music_Service` at resolve | same | link stage NOT MEASURABLE (resolve stops first) |
| `phased_head` (both) | green | green | none |
| `section_row` both spellings | CRC: 836641be/822160 vs 91c46c94/820209 | same | none |

"None" is measured, not assumed: after the restructure every part of the gate was produced
before its first comparison ran, and the reported failure did not change. For `game_debug` the
link stage cannot be reached at this tip until `Music_Service` is supplied, so whether a link
name hides behind it is unknown, and it is not counted as clean.

## Red-first (all at the pin, `.aeon-sigil-ref`)

Mutations (scratch scripts `mutate_ojz_run_a.py`, `mutate_shapes.py`, `mutate_more.py`)
applied identically to the pre-fix and fixed files, quoted back from disk before each run:

- M1, the EARLIER part wrong: `entity_data`'s ObjDef seam addresses +0x10; the plain slice
  offset by 2 (`seq`, `sfx`); the plain `SOUNDBANKHEAD` pin +2; `BASE + 2` (`game_debug`);
  plain `$8002` (`phased_head`); the plain provenance CRC xor 1 (`section_row`).
- M2, the LATER part carries a real name: `pub data HIDDEN_NAME_PROBE: [u8;
  NO_SUCH_NAME_PROBE] = embed(...)` appended to `ojz_act_pool.emp`, to the debug shape's
  `seq_opcode_tab.emp`/`sfx_bank.emp`/`boot.emp` (a `shadow_aeon_tree`), and
  `bsr.w NO_SUCH_LINK_PROBE` in `game_debug.emp`.

Old structure (`git checkout 7bd90670 --` / `a6691120 --`, staged, `git diff --cached --stat`
shown; logs `redfirst-old.log`, `redfirst2-old.log`): all nine red on M1 (entity_data first
diff at 0x5, `SeqOpcodeTable differs from plain`, `SFX head differs from plain`, `pins::SOUNDBANKHEAD
base (debug=false)`, `first proc (at BASE)`, `window VMA ($8000)` twice, `91c46c95/820209`); the
string `NO_SUCH` appears in no diagnostic. Fixed structure (`redfirst-new.log`,
`redfirst2-new.log`): all nine report the injected name (`unknown name NO_SUCH_NAME_PROBE`,
`unresolved symbol NO_SUCH_LINK_PROBE for fixup in section game_debug`). Each run restored with
`git checkout HEAD --` from the committed fix; `git status --porcelain` empty after.

Two first attempts did not count and were redone: an unused `pub const` naming an unknown name
is never evaluated (the `ojz_run_a` M2 did not fire; a typed data length is evaluated), and a
doubled `{{src}}` wrote a literal `{src}` into the shadow file (a parse error, not a name).
The `soundbankhead` M1 anchor first matched twice and did not apply; that test stayed green
unmutated, as it must, and was rerun with a unique anchor.

## Full suites (`cargo test --release --workspace --no-fail-fast`)

Launched = `Running` + `Doc-tests` headers; reported = `test result:` lines.

| run | sigil | AEON_DIR | launched | reported | passed | failed | ignored |
|---|---|---|---|---|---|---|---|
| before @pin | `7bd90670` | pin `ec640bcf` | 504 | 504 | 5732 | 0 | 2 |
| before @tip | `7bd90670` | tip `9caa1368` | 504 | 504 | 5556 | 176 | 2 |
| after @pin | `a6691120` | pin `ec640bcf` | 504 | 504 | 5732 | 0 | 2 |

Name-shaped lines at the tip before (`grep -cE 'unknown name|unresolved symbol|not defined in
this link|embed.not-found|unresolved branch'`): 12, all `Music_Service` or `Music_Want`, all
visible, in 12 tests (Open, item 1).

A run at `a6691120` against the tip is VOID and not cited: during it this parcel committed
`79c1cd94` and rebuilt two test binaries into the same target, which relinked the `sigil` it
tests (`a6691120-dirty`), and three `version_provenance` tests failed on exactly that.

`repin --check` at the pin: `pins.rs unchanged` (`repin-check-pin.log`).

## The tip tree

`scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-hidden-tip 9caa1368`, tool
built from this tree (`sigil 0.1.0 (7bd90670)`, closure `da5f6ae0`), `PROVISION-END rc=0`.
No `repin --check` control exists at a non-pinned revision; what shows the tree is right is
that the provisioner built both s4 shapes from it and emitted both listings, and that the
tree's HEAD is `9caa1368`. zlib CRC-32 + size, recomputed independently:

| file | CRC-32 / size | origin |
|---|---|---|
| s4.bin | `836641be/822160` | BUILT at `9caa1368` |
| s4.debug.bin | `2e0a2207/848817` | BUILT at `9caa1368` |
| demo.bin | `1c7a34d3/96863` | the pin's golden, copied (not rebuilt) |
| demo.debug.bin | `72e405a5/103185` | the pin's golden, copied (not rebuilt) |

The demo pair is not a tip measurement: the provisioner builds demo shapes only under
`REF_BUILD_DEMO=1`.

## Open

1. Visible name failures at aeon `9caa1368`, not hidden and so outside this parcel:
   `Music_Service` (unresolved branch target in `game_loop`: `game_loop_port` x4,
   `game_debug_port` flip, `tranche5` `drain_define_is_load_bearing` and
   `oversize_combo_...`) and `Music_Want` (not defined in this link: `parallax_port` x2,
   `sound_api_port` x2, `tranche5` `typed_extern_has_no_mirror...`). They need the same
   derive-from-the-tree repair the previous parcel did, before the pin advances past them.
2. `game_debug`'s link stage at the tip is unmeasured until item 1 is repaired.
3. `ojz_run_b_port`'s stale declared shape delta (previous note, open item 1) is untouched.
