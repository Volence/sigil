//! The `repin` staleness guard (D-T10.5) + the acceptance baseline (D-T10.8).
//!
//! `pins_rs_is_current` regenerates the pin table IN-MEMORY from the live
//! listings and compares against the committed `src/pins.rs` — a stale
//! pins.rs can no longer hide. REFERENCE-DEPENDENT: needs the sibling `aeon`
//! tree's `s4.lst`/`s4.debug.lst` (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, it SKIPS green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! The acceptance tests are HERMETIC: they pin the committed `pins::*`
//! values against the hand-typed literals the 16 test files carried at the
//! tool's first green (the tranche-10 design note's survey table). If the
//! generator ever mis-derives a value, the mismatch surfaces HERE first,
//! named — not as a byte-diff panic three suites later.

use std::path::PathBuf;

use sigil_harness::native;
use sigil_harness::pins;
use sigil_harness::repin::{
    diff_pins, load_manifest, render, resolve, strip_provenance, Listing, Provenance,
};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The committed pins.rs must equal an in-memory regeneration from SIGIL'S OWN
/// resolved layout, modulo the `[provenance]` stamp lines (a rebuild that moves no
/// pin is not drift).
///
/// RE-POINTED at the sigil-native source (Stage-3 P4c, kill-list row 34): the asl
/// `.lst` parse is gone; the addresses now come from `native::sigil_native_symbol_
/// listing` (the fully-resolved symbol table — labels + folded equates incl. MDDBG,
/// `.emp` locals demangled, section-END markers synthesized). Currency is checkable
/// again — against sigil's own layout, no asl. REFERENCE-DEPENDENT: needs the sibling
/// aeon tree (`AEON_DIR`) + `SIGIL_EMIT` (the resolve builds the sound-on shape).
#[test]
fn pins_rs_is_current() {
    let aeon = aeon_dir();
    if !aeon.join("s4.bin").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", aeon.display());
        }
        eprintln!("skip: aeon tree not present (set AEON_DIR)");
        return;
    }
    let (pm, pe) = native::sigil_native_symbol_listing(&aeon, false)
        .unwrap_or_else(|e| panic!("plain resolve: {e}"));
    let (dm, de) = native::sigil_native_symbol_listing(&aeon, true)
        .unwrap_or_else(|e| panic!("debug resolve: {e}"));
    // Phase-bank label LMAs (T4): a `phase_bank` region pins its base to the LMA.
    let pl = native::phase_bank_lmas(&aeon, false).unwrap_or_else(|e| panic!("plain phase lma: {e}"));
    let dl = native::phase_bank_lmas(&aeon, true).unwrap_or_else(|e| panic!("debug phase lma: {e}"));
    let plain = Listing::from_symbols(pm, pe, "plain".into()).with_phase_lma(pl);
    let debug = Listing::from_symbols(dm, de, "debug".into()).with_phase_lma(dl);
    let manifest = load_manifest(include_str!("../repin.toml")).expect("repin.toml must load");
    let resolved = resolve(&manifest, &plain, &debug).unwrap_or_else(|e| panic!("resolve: {e}"));
    let prov = Provenance {
        plain_path: "sigil-native canonical resolve".into(),
        debug_path: "sigil-native canonical resolve".into(),
        plain_stamp: plain.stamp.clone(),
        debug_stamp: debug.stamp.clone(),
    };
    let generated = render(&resolved, &prov);
    let committed = include_str!("../src/pins.rs");

    if strip_provenance(committed) != strip_provenance(&generated) {
        let changes = diff_pins(committed, &generated);
        let detail: Vec<String> = changes
            .iter()
            .map(|c| {
                format!(
                    "  {}: {} → {}",
                    c.name,
                    c.old.as_deref().unwrap_or("(new)"),
                    c.new.as_deref().unwrap_or("(removed)")
                )
            })
            .collect();
        panic!(
            "src/pins.rs is STALE against the live listings ({} changed pin(s)):\n{}\n\
             run: cargo run -p sigil-harness --bin repin",
            changes.len(),
            detail.join("\n")
        );
    }
}

/// D-T10.8 acceptance: the generated values byte-match the CURRENT
/// hand-typed pins for a representative spread of every pin class —
/// per-shape bases, shape-INVARIANT lens (animate), shape-DEPENDENT lens
/// (rings/core), literal-len regions (sound_api implicitly via SOUND_API in
/// the migration), symbol Pins, dotted-local offsets, and the ROM end pins.
#[test]
fn generated_pins_match_the_hand_typed_baseline() {
    // input-6button wave (2026-08-02, incl. the LOW-FIRST cadence fix +8): the full 6-button burst rewrite grows the
    // controllers region +0xB0 (both shapes), sliding EVERY engine base after it
    // +0xB0; the 8 new controller-block RAM cells slide every RAM symbol after
    // 0xFFFF8035 by +0x8. GAME_LOOP plain LEN 0x12->0x14 (the shift landed
    // S4LZ_DecompressDict misaligned in plain -> a real 2-byte align pad inside
    // the region). All other LENs unchanged. ASSEMBLED_LEN +0xB0-absorbed by
    // `org $10000` (engine block only).
    // wave-2 bugfix batch (fix/sprites-pb1-pb2): every base below slid +0xC —
    // B1 VSync_Wait SR-mask grew vblank +8 (shifts hblank onward), C1 controllers
    // 2nd TH-settle nop grew controllers +4 (shifts controllers onward). Lens
    // below are unchanged (those regions' content did not change this batch;
    // controllers' own len grew +4 but no assertion here pins it).
    // animate_port.rs: PLAIN/DEBUG Shape { base, len } — len shape-invariant.
    // Bases slid −4 (t10 core), −8 (t11 sprites), +8 (t11 A1 camera-bias),
    // −2 plain/−4 debug (C-A1 core shrink), +0x22 both (object-pool occupancy
    // grew the core region) — net.
    // BASES: shifted by the byte-changing wave (items 5/10/11) — dma_queue
    // item-11 carry-return grew the engine block +0xC upstream of everything,
    // and dplc item-11 grew +0xC more. animate's OWN plain LEN shrank −8 (item 5:
    // drop both Sound_PlaySFX saves), so its debug LEN is 0x2A8 (was 0x2B0).
    // silent-drop parcel (2026-07-17): the FIRST .asm growth UPSTREAM of ALL
    // engine regions in campaign history — queueStaticDMA's drop-carry contract
    // grew buffers' 7 expansions, and load_art's out-of-line drop handler grew
    // load_art, both ahead of hblank..section in the pre-$10000 bank. Every base
    // below slides +0x62 BOTH shapes (SOUND_API is +0x6C plain / +0xB6 debug — see
    // its note; load_art's DEBUG RaiseError vs release drain-retry makes that one
    // region's shift shape-different). NO region content changed: all lens are
    // unchanged, and ASSEMBLED_LEN/DEBUG_ASSEMBLED_LEN are UNCHANGED — the engine
    // growth is absorbed by `org $10000`, __END__ does not move; only the convsym
    // symbol-table appendix (not pinned) grew.
    // phase2.5 c3 (vdp_init M1 early-exit): Flush_VDP_Shadow grew +2 (btst/dbf →
    // lsr/tst/dbeq) — the FIRST gated engine region — so every base below takes a
    // further +2 BOTH shapes (the "+0x62" tags below are +0x64 cumulative);
    // ASSEMBLED_LEN still unchanged (absorbed by `org $10000`).
    // Then +0x36 BOTH shapes (t18 HBlank RAM-jmp trampoline, 2026-07-23): the
    // hblank region grows 0x12→0x48 (HBlank_Dispatch/HBlank_Null ROM dispatch →
    // HBlank_Install/HBlank_Uninstall + the RAM-slot patch/shadow writes); hblank
    // is upstream of every gated engine region, so every base below slides +0x36
    // both shapes (LENs unchanged). Boot stays byte-neutral (8-byte move.l slot
    // init) so vdp_init and above do not move; the slot lives at the RAM TAIL, so
    // PLAYER_1/DYNAMIC_SLOTS below are unchanged (zero existing-RAM churn).
    // Then −0xA BOTH shapes (t21 buffers step-2 branch modernization, 2026-07-24):
    // buffers.emp goes bare-Bcc/jbra-jbsr and five of the six `bsr.w .build_entry`
    // calls relax to `.s` (the first stays .w — 132-byte reach); the twin shrunk in
    // lockstep; `jsr Parallax_Active_Config` → bsr.w is length-neutral. buffers
    // shrinks $262→$258 both shapes; every base downstream of buffers slides −0xA
    // (LENs unchanged). ASSEMBLED_LEN unchanged (absorbed by `org $10000`).
    // Then −2 BOTH shapes (t23 boot step-5 wave, 2026-07-24): boot.emp's
    // `move.w #0, Frame_Accumulator` → `clr.w` (twin lockstep) — boot is the
    // FIRST region, so EVERY engine base below slides −2 both shapes (boot's
    // own LEN shrinks $1AA→$1A8 / $1AE→$1AC; all other LENs unchanged).
    // ASSEMBLED_LEN unchanged (absorbed by `org $10000`).
    // Then −4 PLAIN ONLY (t22 s4lz step-2 branch modernization, 2026-07-24):
    // s4lz.emp goes bare-Bcc/jbra-jbsr and TWO branches relax to `.s`
    // in the plain shape only (`beq .lit_extended` + `jbra .no_literals` — both
    // spans cross the debug dict-hit assert blob, so debug keeps `.w`; the twin
    // rides ifdef widths, the t19 bg_anim precedent). s4lz shrinks $FC→$F8 plain
    // ($200 debug unchanged); every plain base downstream slides −4, debug bases
    // unchanged. ASSEMBLED_LEN unchanged (absorbed by `org $10000`).
    // boot_port.rs (tranche 23): the FIRST region in the engine chain —
    // [EntryPoint $200, BootData). Debug len +4 = the `__DEBUG__`
    // bsr.w CompressionSelfTest. BOOT_DATA is the data-tail head (the .emp's
    // forward `lea BootData(pc)` target), NOT inside the region.
    // Then −0x10 BOTH shapes (PAL NTSC-only, ruling B, 2026-08-02): boot.emp's
    // region-detection block deletes the two `move.w #TIMING_STEP,(Timing_Step).w`
    // stores (6 B each, one per NTSC/PAL branch) and the `clr.w Frame_Accumulator`
    // (4 B) = −16 B code; boot is the FIRST engine region, so its own LEN shrinks
    // −0x10 and EVERY engine base below slides −0x10 both shapes (LENs unchanged).
    // The matched RAM deletion (Timing_Step/Frame_Accumulator u16 pair) shifts every
    // RAM symbol after 0xFFFF8028 by −0x4 (see the symbol-Pin asserts below).
    // ASSEMBLED_LEN unchanged (the engine-block shrink is absorbed by `org $10000`).
    // Then the I2 Logic_Tick parcel (input/replay plan, 2026-08-02): GameLoop gains
    // `addq.l #1, Logic_Tick` (first instruction after VSync_Wait) — GAME_LOOP is the
    // top region of the pre-$10000 engine bank, so every engine base BELOW it slides
    // (bg_anim's Frame_Counter→Logic_Tick+2 operand is byte-neutral). The shift is
    // SHAPE-DIVERGENT via an alignment ripple: PLAIN +0x10 (the addq landed
    // S4LZ_Decompress misaligned → GAME_LOOP plain LEN 0x12→0x22, a legit align pad),
    // DEBUG +0x4 (the raw addq; GAME_LOOP debug LEN 0x12→0x16, no realign). LENs of
    // every region below GAME_LOOP are unchanged (content untouched); a downstream
    // per-shape reabsorption returns the far-tail debug bases to origin (not asserted
    // here). The Logic_Tick u32 (inserted after Frame_Counter, 0xFFFF8004) slides
    // every RAM symbol after it +0x4 both shapes (see the symbol-Pin asserts). BOOT /
    // BOOT_DATA are UPSTREAM of GAME_LOOP → unchanged; ASSEMBLED_LEN /
    // DEBUG_ASSEMBLED_LEN unchanged (the +2 engine growth absorbed by `org $10000`);
    // CC_DELETE_OFF is an intra-animate offset → unchanged.
    assert_eq!(pins::BOOT.plain_base, 0x200);
    assert_eq!(pins::BOOT.debug_base, 0x200);
    assert_eq!(pins::BOOT.plain_len, 0x1A0);  // +0x8 item27: EntryPoint's `lea (SYSTEM_STACK).w, sp` (4 B) + a 4 B align pad the new plain length needs
    assert_eq!(pins::BOOT.debug_len, 0x1B0);  // +0x4 item27: the same lea; debug was already 4-aligned so it takes no pad — the two shapes converge  // +0x10 art-streaming-p2-task3: DEBUG-only branch-reach ripple — the downstream page_in section + RAM growth pushes a boot-debug jbsr target (Sound_Init / CompressionSelfTest) far enough to flip its reach form (+0x10 code, debug shape only; plain len unchanged)
    assert_eq!(pins::BOOT_DATA, pins::Pin { plain: 0x3A0, debug: 0x3B0 });  // +0x8/+0x4 item27: rides BOOT's growth; both shapes now start boot_data at the same address  // debug +0x10 art-streaming-p2-task3: rides BOOT.debug_len's boot-debug reach ripple

    // Then the I3 replay parcel (input/replay plan, 2026-08-02): engine.replay
    // (Input_Tick + Replay_Hash) inserts between game_loop and s4lz, so every
    // engine-bank region BELOW it slides +0xF0 plain / +0x1A8 debug (the replay
    // region's per-shape span; the debug shape carries the record/checkpoint path).
    // LENs unchanged (content untouched); ASSEMBLED_LEN below is re-absorbed by the
    // pre-$10000 padding, so it stays put. The replay live cells (+0xA after the
    // controller block) slide every RAM symbol after them; the DEBUG @shape_divergent
    // record ring (+0x2800) slides the game-RAM tail in the debug shape only.
    // Then the bug005-sprites-player parcel (2026-08-03) — a MULTI-REGION content
    // wave, not a single upstream slide. Per-region OWN deltas in the engine bank:
    // core +2 code both shapes (DeleteObject's explicit frame_off tail-word clear —
    // sizeof(Sst) $52 is not long-divisible; plain pin +4 = +2 code + a 2-byte align
    // pad to sprites' even base, debug pin +2); sprites −0x18 plain (H2 emit-loop
    // stream-order restructure + size/link word merge) but +0xBA debug (H1 staleness
    // net + the BUG-005 chain-walk, both `if DEBUG == 1 {}` assert blocks — sprites'
    // LEN goes shape-DEPENDENT for the first time); animate +0xA plain / +0x10 debug
    // (AF_SET_FIELD rail fence + refresh-idiom follow-ups); camera +0x48 (H2
    // single-pass + Camera_Init seed clamp, clamp macro split); children +0x10;
    // load_object +6 (frame_off seeding at spawn); sensors −8 (H3 AtLedgeEdge
    // direct probe). Bases below compose those deltas in ladder order — hence
    // plain slides differ from debug slides region by region. The SST $50→$52
    // growth (+2/slot × 66 slots) shifts every RAM symbol after Object_RAM by
    // +0x84 both shapes (SPRITE_*/CAMERA_*/CACHE_* pins et al., not asserted
    // here); the game-side PlayerV jump_headroom+pad + Player_Bound_Right/Bottom
    // cells shift the player RAM tail (+0x100 plain ring realign).
    //
    // Then the replay-hash-addrfree parcel (2026-08-03, aeon master 3191140) —
    // a SINGLE-REGION upstream slide, the simplest shape yet. Only three files
    // changed: engine/system/replay.emp (Replay_Hash rewritten ADDRESS-FREE —
    // the one `dc.l Player_1, PLAYER_HASH_LONGS` hash-table row became five
    // narrower long rows + three word folds that skip the SST's pointer fields,
    // plus six excluded-offset pins / base-evenness / per-span long-divisibility
    // / total-byte accounting `ensure`s), games/sonic4/data/replays/
    // ojz_fixture.bin (re-recorded 208 → 320 B), and docs. So:
    //   • the REPLAY region GREW +0x40 plain / +0x3E debug (its BASE holds —
    //     game_loop is upstream and untouched);
    //   • every engine-bank region BELOW replay slides by that span. In DEBUG the
    //     +0x3E is ≡2 (mod 4), so the first downstream section boundary that was
    //     already aligned re-pads: core's tail picks up a 2-byte `00 00` filler
    //     after ObjectMoveY (proven in the bytes — both shapes now read
    //     `… 4e 75 00 00 41 f8` into InitSpriteSystem, where the chain-31 debug
    //     build needed no pad). Hence CORE.debug_len +2 and a uniform +0x40 for
    //     everything from SPRITES down, in BOTH shapes. core.emp is untouched by
    //     this parcel — the +2 is placement, not content;
    //   • the slide is fully REABSORBED by the pre-error-handler padding: BusError
    //     is anchored and does not move, so no gameplay/object-bank address shifts
    //     (Ground_Move_Cap et al. hold at their bug005 values);
    //   • the fixture sits at the CHAIN TAIL past that anchor, so its BASE holds
    //     too and only its LEN grows +0x70 (0xD0 → 0x140) — which is the ONLY
    //     contribution to the +0x70 on both ASSEMBLED_LEN pins.
    // WAVE-4 Z80 SOUND RECLAIM (aeon parcel/wave4-z80-sound-reclaim). The resident
    // Z80 sound blob shrank 231 B in both shapes (6172 -> 5941 plain, 6298 -> 6067
    // debug) and then takes a 1-byte evenness pad, so BOOT_HEAD's LEN falls
    // 0x1852 -> 0x176C plain and 0x18D0 -> 0x17EC debug. Everything below the boot
    // head slides up by that much, re-quantised by section alignment: a uniform
    // -0xE0 plain / -0xF0 debug on the four engine-bank bases below (the raw span
    // is -0xE6/-0xE4; the remainder is absorbed as inter-section padding).
    //   • SOUND_API is the one that does NOT simply slide: -0x40 plain but +0x70
    //     DEBUG. It sits past the pre-error-handler anchor described further down,
    //     where the reabsorbed slack redistributes rather than translating, so its
    //     debug base rises even though everything upstream fell. sound_api.emp is
    //     untouched by the parcel — this is placement, not content.
    //   • No LEN moves here: the parcel changes Z80 code only (plus a 68k-side
    //     `align 2` in boot_data that is already counted in BOOT_HEAD's LEN).
    // The blob's evenness is now enforced by an `ensure` in aeon boot_data.emp —
    // an odd blob address-errors the 68k at boot, which is how this parcel found
    // out (see the A/B note referenced by chain entry `wave4-z80-sound-reclaim`).
    // DEFECT-BATCH-8 (aeon parcel/defect-batch-8, chain entry 45). Eight defects:
    // the 2026-08-05 reconciliation's five NEW findings + children C1b/C1c/C1d.
    // Pin-relevant shifts, all CODE (no data or RAM-layout change; the one new RAM
    // cell, Sprite_Emit_Active, consumed an existing pad byte):
    //   • vblank +8 (NEW-1: VInt_Lag's `move.w #$8F02, VDP_CTRL` drain-head
    //     re-assert) and buffers +8 (NEW-3: the Sprite_Emit_Active gate) sit
    //     upstream of dplc/core → DPLC/CORE bases slide +0x10 plain / +0x20 debug.
    //   • CORE LEN +0x18 plain / +0x20 debug: the C1b cascade — DeleteObject's
    //     parent_ptr/sibling_ptr front-guards, the a0 entry-park (its pop replaces
    //     the old lea walk-back), and the d2-parked DeleteChildren call.
    //   • Everything below core inherits base + len: ANIMATE/RINGS +0x30 plain,
    //     +0x4C debug (core's growth lands ≡2 mod 16 in debug, so the slide
    //     re-quantises differently per shape — same mechanism as the bug005 row
    //     above).
    //   • SOUND_API plain +0x30 (the plain slide); debug −0x20 — it sits past the
    //     pre-error-handler anchor where slack REDISTRIBUTES rather than
    //     translating (the wave-4 row below describes the same non-sliding zone),
    //     and C1d's debug shrink (assert + message blob deleted, −0x44 raw in
    //     children) hands the zone more slack than the growth takes.
    // SST-FOLD (aeon parcel/sst-fold, chain entry 47, owner-directed). frame_off
    // moved from its H1 bolt-on position at $50 into the engine block at $2E;
    // the custom window is the record tail ($30-$4F) and the record is $50
    // again. Pin-relevant shifts:
    //   • replay.emp's hash walk gained the $4C custom-tail word fold (+~0x10
    //     with alignment), sliding DPLC/CORE/ANIMATE/RINGS +0x10 both shapes
    //     (replay sits upstream of the object modules).
    //   • RAM: the record shrink (66 × 2 = 132 B) pulls Object_RAM_End and
    //     every upper-RAM cell below it down — those pins are all regenerated,
    //     not asserted here.
    //   • ROM lengths are otherwise flat: core's clear lost its tail-word
    //     special case (−4) inside existing pad; DPLC.debug_len +4 is the
    //     region's own align pad re-quantising against the moved successor.
    //   • The replay FIXTURE was re-stamped (33 checkpoint hashes — the
    //     rotate-and-add fold is walk-sensitive), same raw stream and ticks;
    //     determinism proof green on the final ROM.
    // ART-STREAMING-P2-TASK2 (aeon parcel/art-streaming-p2, chain entry 56). The new
    // resumable ZX0 decoder section (ZX0R_Decompress, zx0_resume.emp) inserts between
    // zx0 and math in the engine block, so every engine region from MATH downward slides
    // +0x80 BOTH shapes — DPLC/CORE/ANIMATE/RINGS bases below + DELETE_OBJECT (an
    // intra-core Pin). LENs are unchanged (no ported region's content changed). The
    // +0x80 engine growth is absorbed by `org $10000`, so ASSEMBLED_LEN /
    // DEBUG_ASSEMBLED_LEN hold; the debug symbol appendix grew with the new labels but is
    // past EndOfRom (unpinned). The three DEBUG PageIn counters (+0x6 RAM, debug only)
    // slide RAM pins not asserted in this function.
    // ART-STREAMING-P2-TASK3 (aeon parcel/art-streaming-p2, chain entry 57). Two engine-block
    // growths + one new section. (1) VBLANK grows +0x30 BOTH shapes — the VBlank bookmark hook
    // (tst InFlight / irq_frame.pc range-check / redirect) + VSync_Wait's PageIn_Process slice +
    // its moveq re-zero. (2) BOOT grows +0x10 DEBUG only — a boot-debug jbsr reach flip from the
    // downstream growth (see BOOT.debug_len). So every engine region downstream of VBLANK slides
    // +0x30 PLAIN / +0x40 DEBUG (= vblank +0x30 in both, plus the boot +0x10 in debug): DPLC/CORE/
    // ANIMATE/RINGS bases + DELETE_OBJECT. (3) The new page_in.emp section (PageIn_Process, between
    // load_art and bg) adds +0x5A plain / +0x166 debug, so regions BELOW it (SOUND_API) slide the
    // fuller +0x80 plain / +0x1A0 debug. LENs of ported regions unchanged; the engine growth is
    // absorbed by `org $10000`, so ASSEMBLED_LEN / DEBUG_ASSEMBLED_LEN hold. RAM: the 36-byte
    // release bookmark record sits at the RAM tail (game-RAM-only ripple, not asserted here); the
    // 2 DEBUG scaffold bytes sit in the @shape_divergent block, sliding Object_RAM +0x2 DEBUG
    // (PLAYER_1 / DYNAMIC_SLOTS below). NOTE: the SOUND_API / MDDBG / PLAYER_1 / DYNAMIC_SLOTS hand
    // literals below were also STALE from prior rounds (the repin_pins baseline missed them); this
    // update brings them fully current, folding the prior drift into the task-3 tags.
    // ART-STREAMING-P2-TASK4 (aeon parcel/art-streaming-p2, chain entry 58). The page-in request
    // FIFO + landing handshake + cancel/flush, and level init rerouted through the streaming path.
    // Byte effects: (1) VBLANK grows — VInt_Level's Important-drain Staging_Busy release —
    // plain_len 0x1B0->0x1D0 (+0x20) / debug_len 0x1C0->0x1D0 (+0x10), the two shapes converging;
    // every engine region downstream of VBLANK inherits +0x20 PLAIN / +0x10 DEBUG (DPLC/CORE/
    // ANIMATE/RINGS bases + DELETE_OBJECT). (2) LOAD_ART loses its DEBUG-only raise_error drop
    // handler and gains the enqueue/drain loop: len converges to 0x70 both shapes (plain +0xC /
    // debug -0x40). (3) PAGE_IN replaces the DEBUG-only self-test scaffold with the release FIFO/
    // landing/flush/enqueue procs: len plain 0x5A->0x196 / debug 0x166->0x1A6, so SOUND_API (below
    // page_in) slides the fuller +0x170 PLAIN / +0x10 DEBUG. LENs of ported regions unchanged;
    // engine growth absorbed by `org $10000`, so ASSEMBLED_LEN / DEBUG_ASSEMBLED_LEN hold. RAM:
    // the release FIFO/landing state rides the bookmark record at the RAM tail (game-RAM-only
    // ripple, not asserted here); removing the 2 DEBUG scaffold bytes (Dbg_PageIn_Test_Cycles/
    // Done) from the @shape_divergent block pulls Object_RAM and the tail bookmark record -0x2
    // DEBUG (PLAYER_1/DYNAMIC_SLOTS below; plain unchanged).
    // ART-STREAMING-P2C-T8-T9 (aeon parcels/art-streaming-p2, chain entry 75; the combined
    // Vectorman dual DMA cap + B&R per-act art budget + phase fix + fixture-floor bump,
    // chains 75/76/77/78). VInt_Level grows +0x20 BOTH shapes (the DMA_Enq_Bytes_Frame cap
    // reset + the Act_Art_Budget->Art_Budget_Remaining reload it briefly carried, now moved
    // to the PageIn tick — net: the enqueue-cap clr stays here) and the shared QueueDMA
    // .transfer core grows +0x1A (the byte-cap charge/compare/reject), plus PageIn's tick
    // reload + EnqueueLanding gate. DEBUG additionally emits two guarded counter bumps
    // (Dbg_DMA_Enq_Capped / Dbg_PageIn_Deferred), the +0x10 debug-only delta. So every
    // object-code region downstream slides +0x20 PLAIN / +0x30 DEBUG (DPLC/CORE/ANIMATE/
    // RINGS bases + DELETE_OBJECT). LENs of these regions unchanged; engine growth absorbed
    // by `org $10000`, so ASSEMBLED_LEN / DEBUG_ASSEMBLED_LEN hold. The Act struct's +2-byte
    // act_art_budget + the new RAM words slide only unasserted data/RAM pins.
    assert_eq!(pins::ANIMATE.plain_base, 0x3160);  // +0x10 item27: the boot-growth slide (aligned), which every region downstream of boot inherits  // +0x30 defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x30 art-streaming-p2-task3  // +0x20 art-streaming-p2-task4  // +0x20 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::ANIMATE.debug_base, 0x38de);  // +0x10 item27: same boot-growth slide  // +0x4c defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x40 art-streaming-p2-task3  // +0x10 art-streaming-p2-task4  // +0x30 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::ANIMATE.plain_len, 0x194);  // +0xA bug005: AF_SET_FIELD rail + refresh idiom
    assert_eq!(pins::ANIMATE.debug_len, 0x2B8);  // +0x10 bug005: same + the debug-fenced rail

    // rings_port.rs: the campaign's first shape-dependent LENGTH. RINGS LEN
    // shrank −6 (item 10: DrawRings camera-bias fold nets −6 B). Bases shifted by
    // the upstream wave.
    assert_eq!(pins::RINGS.plain_base, 0x34f4);  // +0x10 item27: boot-growth slide  // +0x30 defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x30 art-streaming-p2-task3  // +0x20 art-streaming-p2-task4  // +0x20 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::RINGS.debug_base, 0x3d9e);  // +0x10 item27: boot-growth slide  // +0x4c defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x40 art-streaming-p2-task3  // +0x10 art-streaming-p2-task4  // +0x30 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::RINGS.plain_len, 0x1B8);   // −6: item 10 DrawRings fold
    assert_eq!(pins::RINGS.debug_len, 0x214);

    // core LEN shrank −0xA in c4 (Spawn_Count: InitObjectRAM store −4 + RunObjects
    // moveq+store −6). Bases −0xA in c5 (the boot.asm CROSS_RESET store removal is
    // upstream of dplc/core, so core's base slides with everything downstream of boot).
    assert_eq!(pins::CORE.plain_base, 0x2a40);  // +0x10 item27: boot-growth slide  // +0x10 defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x30 art-streaming-p2-task3  // +0x20 art-streaming-p2-task4  // +0x20 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::CORE.plain_len, 0x300);    // addrfree-invariant: plain's +0x40 span is ≡0 (mod 4), tail pad unchanged  // +0x18 defect-batch-8
    assert_eq!(pins::CORE.debug_base, 0x2ca0);  // +0x10 item27: boot-growth slide  // +0x20 defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x40 art-streaming-p2-task3  // +0x10 art-streaming-p2-task4  // +0x30 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::CORE.debug_len, 0x750);    // +2 addrfree: the tail align pad that absorbs debug's ≡2 (mod 4) span — placement, core.emp untouched  // +0x20 defect-batch-8
    assert_eq!(pins::DPLC.plain_base, 0x2998);  // +0x10 item27: boot-growth slide  // +0x10 defect-batch-8  // +0x10 sst-fold  // +0x80 art-streaming-p2-task2  // +0x30 art-streaming-p2-task3  // +0x20 art-streaming-p2-task4  // +0x20 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::DPLC.debug_base, 0x2bf8);  // +0x10 item27: boot-growth slide  // +0x20 defect-batch-8  // +0xc sst-fold  // +0x80 art-streaming-p2-task2  // +0x40 art-streaming-p2-task3  // +0x10 art-streaming-p2-task4  // +0x30 art-streaming-p2c-t8-t9  // -0x50 wave-f3-f1-f6
    assert_eq!(pins::DPLC.plain_len, 0xA8);     // +0xC: item-11 bcs + post-loop commit (both procs)
    assert_eq!(pins::DPLC.debug_len, 0xa8);   // item 6 REMOVED (soak disproved single-entry) — debug == plain  // +0x4 sst-fold

    // animate_port.rs: the DeleteObject inbound label. bug005-invariant: the +2
    // tail clear lands INSIDE DeleteObject after its label, so the label holds.
    assert_eq!(pins::DELETE_OBJECT, pins::Pin { plain: 0x2b10, debug: 0x2d70 });  // +0x10 item27: slides with core on the boot growth  // defect-batch-8: plain +0x10, debug +0x20  // sst-fold  // +0x80 art-streaming-p2-task2  // art-streaming-p2-task3: plain +0x30, debug +0x40  // art-streaming-p2-task4: plain +0x20, debug +0x10  // art-streaming-p2c-t8-t9: plain +0x20, debug +0x30  // wave-f3-f1-f6: plain -0x50, debug -0x50

    // m1d_rom.rs / m1d_debug_rom.rs / mixed_dac_rom.rs: the END-line pins.
    // +0xCC both shapes from the churn-first ObjectTest scene (test_churn.asm +
    // object_test_state growth), then +0xC debug only from the OJZ scene-pin
    // hook's two `ifdef __DEBUG__` guards (Debug_Scene_Freeze).
    // UNCHANGED by `mddbg-symbols`: that parcel is a pure PERMUTATION of the tail
    // (Replay_OJZ_Fixture moved ahead of the fault-handler island so the vendored
    // MDDBG blob's baked `lea` locator — which points at blob end — lands on the
    // deb2 appendix instead of the fixture's `ARP0` magic). Both totals held; only
    // the tail regions' bases moved, +0x140 for the island.
    //
    // +0x1082 `crash-report`: the owner ruling that a shipped crash must be
    // REPORTABLE. RELEASE now carries the MD Debugger island (+0x10B0) instead of
    // ReleaseFault (−0x2E), plus the deb2 symbol appendix past EndOfRom. This
    // EXACTLY reverses item29p4's plain-shape delta — that parcel converted
    // unconditional behaviour into a flag, and this is choosing the other setting.
    // ReleaseFault survives as the lean shape's arm (CRASH_REPORT=0), which is
    // off-canonical and therefore has no pin at all. The DEBUG total does not move:
    // debug always carried the island.
    // `cheat-flag`: debug-fly moves behind a runtime `Cheat_Flags` bit (owner-ruled a
    // CHEAT, not equipment — the payload ships, gated at runtime, awaiting a cheat
    // code). Release is UNCHANGED at 0x5DC30: the two gate sites live in the fixed
    // 0x10000-byte object bank, whose growth is absorbed by fill, and release writes
    // the flag nowhere (boot already zeroes Work RAM, so default-off costs zero
    // bytes). DEBUG grows +6 for the one `move.b #CHEAT_DEBUG_FLY, Cheat_Flags` that
    // arms the bit, so the debug shape behaves exactly as it did before the parcel.
    // `b-jumps` (2026-08-05): B joins the jump mask whenever CHEAT_DEBUG_FLY is
    // clear (classic three-button jump), and is stripped back out while the cheat
    // owns B. +0xE at each of the two raw-button readers (the press latch in
    // player_common, the release-cap held check in player_air). BOTH totals stay
    // put — the object bank absorbs it, same as the cheat-flag parcel.
    // `objtest-gate` (2026-08-05): the object-test scene + eight scene-only
    // objects + particle_anims left the PLAIN shape (registry if-debug; owner
    // ruling — a harness you drive is equipment, and equipment does not ship).
    // The object bank absorbs its own removals as fill, but the post-sound-bank
    // tail loses object_test_state's span: ASSEMBLED_LEN −0x2C8 (re-quantised).
    // DEBUG grows +0x34 net from the ownership moves (TestArt -> ojz_scroll_test,
    // the enemy/parent objdefs -> object_test_state).
    // `slide-fixture` (2026-08-07): a second recorded input fixture (240 B,
    // +0xF0 after alignment) embedded beside the standing one, covering the
    // entity-window slide path the shipped stream never reaches, in all four
    // crossing directions. It is placed after ALL gameplay content and before
    // the fault-handler island, so zero gameplay addresses move — BOTH shapes
    // grow by exactly 0xF0 and every fault-vector pin shifts by the same amount.
    // `patchrun-batch` (2026-08-09, aeon 01d9d05): PageCache_PatchWord/Ref/Unref
    // deleted, PatchRun_Seq/_Col emitted (net −230 B real code) — the object
    // bank absorbs the shrink as fill, BOTH totals stay put; every pin in the
    // affected modules shifts (the −0x30 family in the repin diff).
    // `replay-rerecord` (2026-08-09, P-2 closure): both input fixtures
    // re-recorded against post-P2 master (standing 314→272 B, slide 240→288 B,
    // net +6 → +8 after alignment). Placed after all gameplay content, so zero
    // gameplay addresses move; plain absorbs it in quantization, debug grows +8.
    assert_eq!(pins::ASSEMBLED_LEN, 0x5DA58); // +0xF0 slide-fixture; patchrun/rerecord absorbed
    assert_eq!(pins::DEBUG_ASSEMBLED_LEN, 0x5F850); // +6 cheat-flag arm write; +0x34 objtest-gate moves; +0xF0 slide-fixture; +8 replay-rerecord

    // animate_port.rs: `AnimateSprite.cc_delete` − `AnimateSprite`. Shape-
    // DEPENDENT (item 4). Offset stable within animate (.cc_delete precedes the
    // item-5 .evt_sound edit), so plain 0x104 / debug 0x15E hold.
    assert_eq!(pins::CC_DELETE_OFF, pins::ShapeOffset { plain: 0x104, debug: 0x15E });
}

/// The remaining pin classes the migration will lean on: per-shape offsets
/// (rings), literal-len regions (sound_api), debug-only symbols (MDDBG),
/// and a RAM-cell Pin — all against the hand-typed sources.
#[test]
#[ignore = "RETIRED by Wave-B B-0 (packed placement): this test asserts literal pin VALUES,
which now legitimately move on every layout-shifting parcel — the hand-typed baseline is the
pin-tax class the packing walk exists to kill. Repin correctness stays covered by
generated_pins_match_the_hand_typed_baseline (generator-vs-file) and pins_rs_is_current
(file-vs-resolve). The pin-history narration in this body is preserved for archaeology."]
fn secondary_pin_classes_match_the_hand_typed_baseline() {
    // rings_port.rs: ringcol_off, the one per-shape offset. −6 (item 10:
    // DrawRings shrinks ahead of RingCollision within the region).
    assert_eq!(pins::RINGCOL_OFF, pins::ShapeOffset { plain: 0x116, debug: 0x172 });

    // sound_api_port.rs: base + literal len (no end symbol in the listing).
    // Bases slid −4 (t10), −8 (t11), +8 (A1), +4/+2 (C-A1/Bug-1), +0xA (ring-art
    // DrawRings), +0x22 (object-pool occupancy core growth), then −0x1C plain /
    // −0xC debug (tranche-12 entity_window step-2 branch shrink), then the whole
    // retro-fix-audit-1 batch. Item 11's dma_queue +0xC shifts BOTH shapes;
    // items 5 (−8) / 10 (−6) net into the plain base too. Plain 0x5D46 / debug
    // 0x770E. Then −0x6 both shapes (tranche-13 load_object: step-2 `bne.w
    // .alloc_fail` → bne.s −2, step-5 Load_ObjectList redundant a0 save/restore
    // removed −4; both upstream of sound_api). Then −0x16 both shapes (t13
    // step-5 second look: Load_Object burst copy movem-pairs → 6× move.l −0x10,
    // d4 push/pop eliminated −0x4, Load_ObjectList `bsr.w Load_Object` → bsr.s
    // −0x2 as the −0x14 shrink pulled the backward target into .s range). Then
    // −0xE both shapes (tranche-15 section.emp step-2: the modernization to bare
    // Bcc / jbra / jbsr relaxed 7-8 of section.asm's conservatively-.w branches
    // to .s at asl's fixpoint, shrinking the section region 0x3EA→0x3DC; section
    // is upstream of sound_api in the pre-$10000 engine bank). Then −0xE plain /
    // −0x6 debug (tranche-16 tile_cache.emp step-2: the same bare-Bcc/jbra/jbsr
    // modernization relaxed 7 plain / 3 debug of tile_cache.asm's conservative-.w
    // branches to .s — 4 of them shape-divergent (ifdef __DEBUG__, the assert
    // block blocks .s in debug), shrinking the tile_cache region 0x924→0x916 /
    // 0x9DC→0x9D6; tile_cache is upstream of sound_api in the engine bank). Then +0xA both (t16 Wave 2 (i): the crossing-decompress prefetch SCAN replaced the one-block prefetch, growing tile_cache +0xA). Then +0x76 both (t16 Wave 2 (ii): TileCache_WarmupBelowRow cold-start pre-stage proc + the Init bsr.w, growing tile_cache 0x920→0x996 plain / 0x9E0→0xA56 debug).
    // Then +0xA both (t16 Wave 2 (i) prefetch scan) and +0x76 both (t16 Wave 2
    // (ii) WarmupBelowRow) landed the 0x996/0xA56 above; then +0x10 both
    // (unified-prefetch H5: BlockStage_PtrTable 12->16 slots grew tile_cache
    // 0x996->0x9A6 plain / 0xA56->0xA66 debug; tile_cache upstream of sound_api).
    // Then +0xDE both (pass-2 FillRow segment restructures: 1.1a nametable +0x88,
    // 1.1b collision +0x56, growing tile_cache; upstream of sound_api). Then +0x22
    // both (pass-2 1.2 Draw_TileRow_FromCache segment restructure, growing
    // plane_buffer; upstream of sound_api). Then +0x3E both (pass-2 1.3
    // CopyBlockColumn wrap-split, growing tile_cache; upstream of sound_api).
    // Then the silent-drop parcel (2026-07-17): +0x6C plain / +0xB6 debug — the
    // ONLY shape-different shift of the parcel. site-A (buffers/macro drop-carry,
    // +0x62 both, upstream of everything) + site-B (load_art's drop handler, which
    // is between section and sound_api): load_art grows +0xA plain (release
    // drain-retry) but +0x54 debug (the out-of-line RaiseError expansion), so
    // sound_api — the one region downstream of load_art — inherits the shape gap.
    // len unchanged (no sound_api content changed).
    // Then −0x10 BOTH shapes (pass-3 Parcel A dead-save deletions, 2026-07-22):
    // entity_window loses two full movem-pairs (−16 bytes); every engine-bank
    // region downstream of entity_window — including sound_api — slides −0x10 in
    // both shapes. (EndOfRom itself is unchanged: the −16 is re-absorbed by padding
    // before the ROM end, so ASSEMBLED_LEN below stays put; sound_api len unchanged.)
    // Then +0x90 BOTH shapes (pass-3 8b prefetch scan memoize, 2026-07-22): the
    // generation-word check/record alone grows tile_cache +0x90; every engine-bank
    // region downstream — including sound_api — slides +0x90 in both shapes.
    // (ASSEMBLED_LEN + the END-line MDDBG pins are unchanged: the +0x90 is
    // re-absorbed by engine-bank padding before the fixed high-address banks.)
    // Then +0x1C BOTH shapes (pass-3 8b move.l rider #1, NT segment copy): the
    // FillRow nametable copy loop becomes move.l pairs + a per-run odd-word tail,
    // growing tile_cache +0x1C; sound_api and every downstream region slide +0x1C.
    // Then +0x1C BOTH shapes (pass-3 8b move.l rider #2, plane_buffer drain): the
    // Draw_TileRow_FromCache .emit_row_run copy becomes move.l pairs + a per-run
    // odd-word tail, growing plane_buffer +0x1C; plane_buffer is upstream of the
    // whole level+sound block, so tile_cache/collision_lookup/section/sound_api
    // bases each slide +0x1C (their LENs unchanged).
    // Then −0xA BOTH shapes (pass-3 phase2.5 c4 Spawn_Count, 2026-07-22): core
    // loses 3 instructions (InitObjectRAM store −4, RunObjects moveq+store −6);
    // core is upstream of the whole engine bank, so sound_api and every region
    // downstream of core slide −0xA both shapes (their LENs unchanged).
    // Then −0xA BOTH shapes (pass-3 phase2.5 c5 CROSS_RESET_MAGIC, 2026-07-22): the
    // dead Cold_Boot `move.l #'INIT',(addr).l` store (−0xA) is removed from boot.asm,
    // which is upstream of EVERY gated engine region (vdp_init onward), so all engine
    // bases — sound_api included — slide another −0xA both shapes (LENs unchanged; no
    // RAM shift, the CROSS_RESET equates are fixed-addr `=`).
    // Then −0x8 BOTH shapes (pass-3 phase2.5 c6 ess_*_left_idx, 2026-07-22): the two
    // dead EntityScanState left-edge ratchet fields are cut mid-struct (len $1A→$16),
    // dropping their two `clr.w` inits from EntityWindow_InitSection (−0x8 code); every
    // region downstream of entity_window — sound_api included — slides −0x8 both shapes.
    // Then −0x18 BOTH shapes (t18 parallax step-2 Hscroll_Dirty pad cut, 2026-07-23):
    // the 4 dead `move.b #imm,(Hscroll_Dirty_*).w` stores are removed from
    // Parallax_Step4_Fill (−0x18 code); parallax is upstream of sound_api in the
    // engine bank, so sound_api and every downstream region slide −0x18 both shapes.
    // (No RAM shift: the two RAM symbols become a same-size 2-byte reserved pad, so
    // PLAYER_1/DYNAMIC_SLOTS below are unchanged.)
    // Then +0x10 BOTH shapes (t18 step-5 H2, 2026-07-23): the flat-fill 8x unroll
    // in Parallax_Fill_PerLine grows parallax +0x10 (lsr + 7 extra move.l);
    // parallax is upstream of sound_api, so sound_api's base slides +0x10 both
    // shapes (LENs unchanged — sound_api's own content is untouched).
    // Then +0x10 BOTH shapes (transition parcel B2 mode-contract, 2026-07-23): the
    // new Parallax_Active_Config accessor proc (+0x12) minus the Vscroll_Write
    // bsr.s routing that replaced a move.l (−0x2) grows parallax +0x10; parallax
    // is upstream of sound_api, so sound_api's base slides +0x10 both shapes
    // (LENs unchanged — sound_api's own content is untouched).
    // Then +0x8 BOTH shapes (transition parcel B3 frames-remaining ramp, 2026-07-23):
    // the band-loop lerp `asr.w #shift` (2 B) becomes `ext.l + moveq + move.b + divs.w`
    // (10 B) for exact convergence-by-frame-0 — parallax grows +0x8; sound_api's base
    // slides +0x8 both shapes (LENs unchanged).
    // Then +0x1C BOTH shapes (transition parcel B1 re-cross cancel branch, 2026-07-23):
    // StartTransition's a0==current no-op becomes a cancel branch (tst frames + clear
    // target/frames + snap-pending + mode-restore) — parallax grows +0x1C; sound_api's
    // base slides +0x1C both shapes (LENs unchanged).
    // Then −0x6 BOTH shapes (t19 camera step-2 branch modernization, 2026-07-24):
    // camera.emp goes bare-Bcc/jbra and the asl fixpoint relaxes the twin's three
    // conservative `.w` branches (bra.w .no_move / bne.w .clamp_y / bra.w .clamp_y,
    // in-range at 74/108/40) to `.s` — camera shrinks $16A→$164; camera is upstream
    // of parallax/bg/bg_anim/sound_api in the engine bank, so each base slides
    // −0x6 both shapes (LENs unchanged).
    // Then −0x2 BOTH shapes (t19 bg_anim step-2 branch modernization, 2026-07-24):
    // bg_anim.emp goes bare-Bcc/jbra — the bare `beq .exit` relaxes to `.s` (twin
    // shrunk in lockstep; the two jsr→jbsr sites re-emit as bsr.w, size-neutral
    // against the abs.w jsr they replace) — bg_anim shrinks $A0→$9E; sound_api's
    // base slides −0x2 both shapes (LEN unchanged).
    // Then +0x62 DEBUG ONLY (t19 pass-1 step-4 band-count assert, 2026-07-24):
    // BgAnim_Update gains `assert.w d7, ls, #BGANIM_MAX_BANDS` (defense-in-depth
    // against a table wider than BgAnim_LastStep); the assert self-gates to zero
    // bytes in the plain shape (rings/core precedent), so only the debug bases
    // downstream of bg_anim slide (+the assert code + its message blob = +0x62;
    // bg_anim debug_len $9E → $100, now shape-DEPENDENT).
    // Then +0x58 DEBUG ONLY (t19 dry-panel adjudication, 2026-07-24): BgAnim_Update
    // gains `assert.w d3, gt, #0` (piece-1 length — a drifted table row would send
    // QueueDMA length <= 0 = a 128KB VRAM spray; lens-C2 catch) + the twin's two
    // spanning branches take ifdef-__DEBUG__ `.w` widths (the .emp side stays bare,
    // relaxing per shape). Plain self-gates to zero bytes; bg_anim debug_len
    // $100 → $158; debug bases downstream slide +0x58.
    // Then −0x4 BOTH shapes (t20 load_art step-2 branch modernization, 2026-07-24):
    // load_art.emp goes bare-Bcc/jbra-jbsr and two conservative `bsr.w` calls relax
    // to `.s` (Art_Decompress — an in-region backward reach — and BG_Init — the
    // next placement; twin shrunk in lockstep) — load_art shrinks $68→$64 plain /
    // $B2→$AE debug; bg/bg_anim/sound_api bases slide −0x4 both shapes (LENs
    // unchanged).
    // Then −0x6 BOTH shapes (t24 children step-2 branch modernization, 2026-07-24):
    // children.emp goes bare-Bcc/jbra-jbsr; the asl fixpoint relaxes the twin's
    // three in-reach `.w` branches (bsr.w PopulateSpawnedPieceCount at −124,
    // beq.w .done at +126, bne.w .alloc_fail at +124) to `.s`, and the seven
    // jsr→bsr.w call conversions are size-neutral — children shrinks $30E→$308.
    // children is upstream of load_object/plane_buffer/tile_cache/…/sound_api in
    // the engine bank, so every base below slides −0x6 both shapes (LENs
    // unchanged). ASSEMBLED_LEN is UNCHANGED: the engine block's shrink is
    // absorbed by the `org $10000` ObjCodeBase shield.
    // Then the t24 step-5 wave (2026-07-24), which is SHAPE-SPLIT — the first
    // region to move in OPPOSITE directions per shape. PLAIN −0x6C: the six
    // alloc oversaves collapse to a single `move.l a1,-(sp)` (or nothing),
    // the five dead fail-path descriptor skip-walks are deleted, the chain-head
    // write hoists out of three loops, DeleteChildren's per-child movem pair
    // becomes one for the whole cascade, and `move.w #0` → `clr.w`. DEBUG
    // +0x46: the same −0x6C plus the TWO chain-contract `assert.w` sites and
    // their message blobs (which also push one `bsr` out of `.s` reach in the
    // debug shape only — the twin carries that width under `ifdef __DEBUG__`).
    // children is upstream of everything below, so plain bases slide −0x6C and
    // debug bases +0x46. The assert count is load-bearing: an earlier
    // five-assert version (+0x14E) pushed engine symbols across $8000, which
    // widened the player code's `jsr (Sym).w` call sites to abs.l and slid the
    // whole DEBUG object bank +0xC — breaking every harness fixture that
    // assumes the two banks coincide.
    // Then the t24 PANEL/RULING wave (2026-07-24): +0x3C plain / +0x94 debug on
    // children. Adds: the masked render_flags inherit (band + coordmode) at six
    // creators, the CreateChild_Linked parent_ptr assert (debug only), minus the
    // two deleted effect parent_ptr writes, minus the branchless flip mask
    // (−4 bytes), minus DeleteChildren's movem→move.l park (−4). $8000 BAR
    // CHECKED LIVE at this wave: TEST_STATIC_MAIN/TEST_PARENT_LABEL plain == debug,
    // so no engine symbol consumed by object-bank code crossed the boundary and
    // the game bank did not slide (contrast the earlier five-assert cut, which
    // did move it +0xC).
    // Then the t24 RE-PANEL corrections: the band half of the render_flags
    // inherit REVERTED (a 3-bit band cannot be or-ed — see the gap-ledger row),
    // the surviving COORDMODE read+mask HOISTED out of five of the six creator
    // loops (byte-neutral, −20 cycles/child), and PopulateSpawnedPieceCount's
    // one-register park moved to move.l/movea.l (−12 cycles/child, −4 bytes,
    // which also restored the plain-shape branch margin the wave had consumed).
    assert_eq!(pins::SOUND_API.plain_base, 0x6d0e);  // +0x50 item28-transpose: section +0x20 (per-column RedrawPlanes blit) + bg +0x30 (column blit + move.l tile guard) slide everything downstream  // +0x30 defect-batch-8  // +0x10 sst-fold  // -0x20 ltr-mul: tile_cache/section multiply shrink slides everything downstream  // +0x100 art-streaming-p2-task3 (folds a prior missed round): +0x80 the task-2 zx0_resume slide the baseline never took, plus +0x80 this task (vblank +0x30 + page_in +0x5A, aligned)  // +0x180 art-streaming-p2-task4: vblank +0x20 + page_in plain len 0x5A->0x1A6 (the FIFO/procs replacing the debug-only scaffold)  // +0x520 wave-f3-f1-f6  // +0x20 wave-panel-closing
    assert_eq!(pins::SOUND_API.debug_base, 0x92e0);  // +0x50 item28-transpose: same slide, both shapes  // -0x20 defect-batch-8  // +0x10 sst-fold  // -0x20 ltr-mul  // +0x2F0 art-streaming-p2-task3 (folds a prior missed round): +0x150 the task-2 slide the baseline never took, plus +0x1A0 this task (boot +0x10 + vblank +0x30 + page_in +0x166)  // +0x10 art-streaming-p2-task4: vblank +0x10 + load_art/page_in debug net (load_art len -0x40, page_in base -0x30 + len +0x40)  // +0xd90 wave-f3-f1-f6  // +0x90 wave-panel-closing
    // §D backlog c1+c2 (2026-07-23): the constant-flag spin-class fix (capture-then-
    // test in await_slot + wait_alive, +0x4 both shapes) + the DEBUG-only
    // SPIN_WATCHDOG rails on both spins (+0xB4 debug only). plain len 0x206 -> 0x20A
    // (+0x4); debug_len 0x2FC -> 0x3B4 (+0xB8 = +0x4 fix + +0xB4 watchdogs). The two
    // watchdogs + their raise_error blobs precede Sound_PlaySFX, so SOUND_PLAY_SFX_OFF
    // grows the same +0x4 plain / +0xB8 debug.
    assert_eq!(pins::SOUND_API.plain_len, 0x20A);
    assert_eq!(pins::SOUND_API.debug_len, 0x3B4);
    assert_eq!(pins::SOUND_PLAY_SFX_OFF, pins::ShapeOffset { plain: 0x126, debug: 0x28A });

    // rings_port.rs DEBUG.labels: the debug-only error-handler entries.
    // +0xCC (churn) +0xC (hook guards) both in the debug ROM, like DEBUG_ASSEMBLED_LEN.
    assert_eq!(pins::MDDBG_ERROR_HANDLER, 0x5_E8F2);  // +0x1EE art-streaming-p2-task3 (folds prior missed rounds): the debug ROM's symbol-appendix island rides the full debug-ROM growth (page_in + hook + boot-debug ripple) plus the baseline drift never taken
    assert_eq!(pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER, 0x5_F6B8);  // +0x1EE art-streaming-p2-task3: same debug-appendix slide

    // collision_port.rs: sign-extended RAM labels truncated to u32. debug +0x2:
    // Debug_Scene_Freeze's RAM byte+pad shifts every __DEBUG__-block-downstream
    // RAM symbol (Player_1 among them); plain shape unchanged.
    assert_eq!(pins::PLAYER_1, pins::Pin { plain: 0xFFFF_8A00, debug: 0xFFFF_8A2A });  // debug +0x2 art-streaming-p2-task3: the 2 DEBUG scaffold bytes (Dbg_PageIn_Test_Cycles/Done) in the @shape_divergent block slide Object_RAM in the debug shape. plain +0x12 folds a prior missed round (a RAM-growth parcel the baseline never took; the release bookmark record itself is at the RAM tail, not here)  // debug -0x2 art-streaming-p2-task4: those same 2 scaffold bytes are now DELETED, pulling Object_RAM back down in the debug shape (plain unchanged)
    // DYNAMIC_SLOTS also debug +0x2 (downstream of the __DEBUG__ block).
    assert_eq!(pins::DYNAMIC_SLOTS, pins::Pin { plain: 0xFFFF_8AA0, debug: 0xFFFF_8ACA });  // debug +0x2 art-streaming-p2-task3 (+ prior-round plain +0x12 catch-up), same as PLAYER_1  // debug -0x2 art-streaming-p2-task4: the deleted scaffold bytes, same as PLAYER_1 (plain unchanged)
}
