# 2026-07-29 — seam-1 design: the resident-blob NATIVE-LINK (DESIGN GATE ONLY)

Status: **DESIGN NOTE — answers the seam-1 brief's five question sets. No
implementation, no twin deletion, no build change.** Sigil branch `seam1-design`,
worktree `.worktrees/seam1-design`. Masters: sigil `fb52654` / aeon `597ce06`.
Baseline VERIFIED read-only from the main aeon checkout: plain `s4.bin` =
**4b66cace**, debug `s4.debug.bin` = **1c256b3b** (matches canonical); aeon tree
clean, NOTHING modified. Strict 2888/0 (1 ignored) per the t40 close packet — not
re-run here (a design parcel; no aeon build).

The seam RETIRES the six sound twins (kill rows 70/71/78/83/87 + sound_api). That
deletion is a separate, Volence-visible event dispatched only after this design is
endorsed — this note designs it, it does not execute it.

Everything below is evidence-backed. `.asm`/`.emp` cites are `aeon` paths at `597ce06`;
`.rs`/ledger cites are `sigil` at `fb52654`; listing addresses are from the aeon main
checkout's `s4.lst` / `s4.debug.lst`.

---

## §1 — THE NATIVE-LINK SHAPE

### 1.1 What the blob IS today (AS-phased, one 68k `include`)

The resident sound blob is emitted by ONE line inside the 68k boot data table:

- `engine/system/boot_data.asm:47-49` — under `ifdef SOUND_DRIVER_ENABLED`,
  `include "engine/sound/z80_sound_driver.asm"`. It sits inside `BootData:`
  (`boot_data.asm:4`), the sequential `(a5)+` boot table; the boot loader copies the
  emitted Z80 bytes into Z80 RAM. The `else` arm (`:51-67`) is the no-sound idle
  program (`z80_init.asm` / the `SIGIL_EMP_Z80_INIT` mixed arm) — that arm is the
  single-file precedent for what seam-1 generalizes to five files.

- The driver file frames the blob: `z80_sound_driver.asm:43-46`
  `Z80_Sound_Start: / save / cpu z80 / phase 0`. Teardown at `:1476-1486`: an even-pad
  `if ($ & 1) <> 0 / db 0`, then `dephase / restore / Z80_Sound_End:`, then
  `Z80_SOUND_SIZE = Z80_Sound_End - Z80_Sound_Start` (`:1488`).

### 1.2 The TRUE blob order (verified from the includes + the listing)

The driver `include`s the other four files INSIDE its own `phase 0` framing, in this
order (`z80_sound_driver.asm`):

| # | file | `include` line | plain base (`s4.lst`, phase-0) | first label |
|---|---|---|---|---|
| 1 | z80_sound_driver (front) | — (framing) | `$0000` | `Z80_Sound_Start` (68k `$3DE`) |
| 2 | sound_sequencer | `:1421` | `$0565` | `Sequencer_Frame` @ `$0565` |
| 3 | sound_sfx | `:1428` | `$0CD7` | `Sfx_Frame` @ `$0D2A` |
| 4 | sound_fm | `:1437` | `$12C3` | `Fm_ReparkDac` @ `$12D9` |
| 5 | sound_psg | `:1446` | `$1660` | `Psg_SilenceAll` @ `$1807` |

Then the moved-out tables comment block (`:1448-1474` — the FM/PSG/pitch tables and
`DacSampleTable` now live in the `phase 08000h` bank = **seam-2**, not the resident
blob), the even-pad (`:1480`), `dephase`/`restore` (`:1484-1485`), `Z80_Sound_End`
(`:1486`).

**Order = driver → sequencer → sfx → fm → psg.** The brief's "+ api" is answered
NEGATIVE: **`sound_api` is NOT in the resident blob.** It is 68k engine code
(`engine/engine.inc:613`, `include "engine/sound/sound_api.asm"`, its own
`SIGIL_EMP_SOUND_API` gate at `:612`), the CALLER that pokes the Z80 RAM command slots.
It is a coupled-but-distinct flip (§3.5, §5).

### 1.3 Origin / phase / size (the geometry the native link must reproduce)

Listing facts (`s4.lst`):
- `Z80_Sound_Start` label value = `$3DE` (`s4.lst` "(3) 43/ 3DE"), i.e. **LMA = 68k
  ROM `$3DE` = `BootData + 54`** — guarded by `boot_data.asm` / `s4.lst:13551`
  `if (Z80_Sound_Start-BootData) <> 54 / fatal`.
- Under `phase 0` the INTERNAL labels are **VMA = `$0000`-relative Z80 addresses**
  (Sequencer_Frame `$0565`, etc. above) — this is the address space the copied blob
  runs at in Z80 RAM.
- `Z80_SOUND_SIZE = $181C` (`s4.lst:13489`, `Z80_Sound_End − Z80_Sound_Start =
  $1BFA − $3DE`) = **6172 bytes copied** (plain). `Z80_Sound_End = $1BFA` is the
  post-`restore` 68k address (`s4.lst:13487`); the driver-window figure `$0000..$0565
  = 1381 B` in the `.emp` header (`z80_sound_driver.emp:8`) is the driver's own slice,
  not the whole blob.
- The size is used at `s4.lst:6031` `move.w #Z80_SOUND_SIZE-1, d1` (boot copy count)
  and budget-guarded at `s4.lst:13495-13496` (`if Z80_SOUND_SIZE > SND_STATE_BASE /
  fatal`). So the blob's length is load-bearing in the 68k boot path — the native
  link must define `Z80_SOUND_SIZE` (or an equivalent) for those AS sites while the
  AS engine survives.

### 1.4 The native-link design (region / manifest form)

**One Z80 module, five files, lowered together.** The five `.emp` files
(`z80_sound_driver.emp` + the four it currently `include`s) are lowered as a SINGLE
`lower_module` over an import graph — NOT five separately-lowered modules concatenated.
This is forced by ledger row 1639 (`campaign-gap-ledger.md`): two independently
lowered `.emp` modules collide on their default `sec0` at LMA `0x0` when their section
lists are concatenated. The resident blob must therefore be ONE lowering whose internal
cross-file references are `import`s (§3.4), producing ONE Z80 section.

- **CPU / phase**: the module lowers `cpu: z80`; the section carries **VMA base `$0000`
  (phase)** and **LMA `$3DE`** (the `BootData+54` placement). Every internal label and
  every intra-blob `call`/`jp` resolves against the `$0000` VMA; the emitted bytes land
  at ROM `$3DE`. This is the phased-placement the AS `save/cpu z80/phase 0 … dephase/
  restore` bracket does today — the native link expresses LMA≠VMA directly.
- **Ordering**: the module's emission order MUST be driver → sequencer → sfx → fm → psg
  (§1.2) to reproduce every internal address. In `.emp` terms the driver file is the
  ordering root; the four others are appended in include order (an explicit ordered
  manifest, since `.emp` `import` is symbol-resolution, not emission-ordering — the
  order is a layout fact the blob module must pin, analogous to the AS include order).
- **The even-pad + `Z80_SOUND_SIZE`**: the trailing `if ($ & 1) db 0` (`:1480`) and
  `Z80_SOUND_SIZE` (`:1488`) are blob-tail concerns that live on the driver/module,
  reproduced by the link (the pad falls out of the emitted length; `Z80_SOUND_SIZE`
  is exported for the surviving 68k boot sites, §1.3).

**The 68k-side inclusion path.** Replace `boot_data.asm:49`'s `include` with the
campaign's standing mixed-link arm — the `ifndef SIGIL_EMP_<GATE> … else … org <resume>`
pattern already shipped TWICE for placed code:
- the idle-program precedent RIGHT BELOW it: `boot_data.asm:51-67`
  (`ifndef SIGIL_EMP_Z80_INIT` … `else … Z80_IDLE_SIZE = $3FE-$3D8 / org $3FE`);
- the sound_api precedent: `engine.inc:612-620` (`ifndef SIGIL_EMP_SOUND_API` …
  `else` org-resume at the region end).

So seam-1's 68k arm = `ifndef SIGIL_EMP_Z80_SOUND / include z80_sound_driver.asm /
else` place the native-linked blob at `$3DE` / `org Z80_Sound_End` (`$1BFA` plain,
`$1C78` debug = `$1BFA+$7E`) — plus a numeric `Z80_SOUND_SIZE =` carrier for the boot
sites (the `Z80_IDLE_SIZE = $3FE-$3D8` precedent at `boot_data.asm:66`).

**The combined-link fix classes this rides.** Placing a `.emp` blob and resuming AS
placement in one whole-ROM link is exactly what the combined-link stale-fold fix
hardened — sigil `03d29cd` ("Merge fix-combined-link-locals"). The fix keeps AS-frontend
label refs **relax-safe symbolic on the deferral pass** across ALL operand classes:
(a) **branches / `dc`**, (b) **immediates**, (c) **label-referencing `equ`s incl.
reverse chains via `equ_sym` export**, (d) **explicit-width `jmp`/`jsr` abs**. Root cause
was **cross-seam `JmpJsrSym` growth vs baked constants** (the dup-local-labels hypothesis
in ledger row 1759 was a RED HERRING — corrected in `2026-07-29-t35-step0-recon.md:18-23`
and the commit body). t35 then landed the FIRST full-strength tranche under it
(`pre-t18-roadmap.md:45` — player files pass FULL mixed whole-ROM identity both shapes,
no windowed-only concession). Seam-1 is a Z80-CPU instance of the same mixed-link path,
so the fix's guarantee (an AS module + a placed sigil region combine byte-exactly) is
the load-bearing dependency and it is LANDED.

---

## §2 — THE IDENTITY BAR

The first native link must reproduce the EXACT current blob bytes, in BOTH shapes,
with the two shape-dependent phenomena falling out of the REAL link (not pins).

### 2.1 The whole-blob byte gate (both shapes)

- **Comparand**: the blob region extracted from the reference ROM at LMA:
  - plain: ROM `$3DE .. $1BFA` (= `$181C` bytes), from `s4.bin` (4b66cace).
  - debug: ROM `$3DE .. $1C78` (= `$181C + $7E = $189A` bytes), from `s4.debug.bin`
    (1c256b3b). (`Z80_Sound_End` debug = `$1BFA + $7E`; verify the exact value against
    `s4.debug.lst` at execution — the +$7E is the sequencer growth, §2.2.)
- **Subject**: the sigil native-link blob emission (the one combined Z80 module at
  VMA `$0000` / LMA `$3DE`), assembled with `DEBUG` as a REAL comptime flag per shape.
- **Gate**: `native_blob_matches_reference_{plain,debug}` — byte-equal, whole blob.
  This SUPERSEDES the five windowed oracles' AS-reassembly halves (§3.2): the comparand
  is the reference ROM slice, not an AS re-assembly of a `.asm` twin (which no longer
  exists after retirement).

### 2.2 The +$7E sequencer growth must fall out of the real link

The debug shape is `$7E` longer because `sound_sequencer.emp` carries 16 internal
`if DEBUG == 1` bodies (the Seq_Trace body + 15 `call Seq_Trace` sites — the FIRST
resident Z80 `.emp` with `if DEBUG==1` blocks; `t36-close-packet.md:39-44`, kill row 78).
In the windowed world these are proven by shape-parameterized oracles; in the NATIVE
link `DEBUG` is a genuine comptime flag, so:
- the sequencer emits `$7E` more bytes in debug (growth EMITTED, not pinned);
- every file AFTER the sequencer (sfx `$0CD7→$0D55`, fm `$12C3→$1341`, psg `$1660→…`)
  re-bases `+$7E` because its LMA/VMA slides — these bases are LINK OUTPUTS, and the
  byte gate confirms them against the debug reference. This closes the row-70/71/83
  "shape-variant base" proofs structurally (they were windowed assumptions; now they
  are link facts).

### 2.3 The driver's 9 cross-seam operand bytes must fall out of real imports

The driver window is the SAME SIZE both shapes but NOT byte-identical: five `call`
sites target three callees that live after the sequencer's +$7E growth
(`Sequencer_StopAll`, `Sfx_StopAll`, `SfxDispatch` all shift `+$7E` in debug), so
**9 operand bytes differ** plain vs debug; `Sequencer_Frame` (`$0565`) is the one
shape-invariant target (`z80_sound_driver.emp:14-22`, `t40-close-packet.md:80-87`).

TODAY these addresses are FED to the windowed oracle as per-shape equ carriers —
`z80_sound_driver_port.rs:125-147` `link_seam(shape, doctor)` computes
`shift = if debug { 0x7E }` and hands `Sequencer_StopAll = after(0x0CB2)` etc. to both
the `.emp` (`-D`/equ) and the AS twin (equ). **In the native link these are IMPORTS**
(§3.4): the driver's four outbound `extern proc`s (`z80_sound_driver.emp:112-115`
Sequencer_Frame/Sequencer_StopAll/Sfx_StopAll/SfxDispatch) resolve to the REAL in-module
label addresses. So the 9 operand bytes are DERIVED by the linker in each shape — the
byte gate's job is precisely to prove the derivation equals the reference, retiring
`link_seam`'s hand-computed `+$7E`.

### 2.4 The downstream-unchanged proof (the existential bar)

The driver is the blob FRONT: one byte here slides the WHOLE corpus (the t40 existential
bar, held throughout — `t40-close-packet.md:11-13`). Because the native-link blob is the
SAME length (`$181C` plain / `$189A` debug), everything after it is byte-identical:
- **whole-ROM CRC** vs 4b66cace / 1c256b3b — the strongest downstream proof (a single
  byte of blob drift changes the CRC and slides `$10000` engine + `$58000` banks).
- The 68k engine (`org $10000`), the banked sound data (`align $8000` / `org $58000`
  region, `main.asm:319/338/416-425`), and the ROM tail must all reproduce. Since seam-1
  is CODE-length-neutral by construction, "downstream-unchanged" reduces to "the blob is
  byte-exact AND the same length" — both proven by §2.1.

**Identity-bar risk (open question OQ-2):** the native link must place a single combined
Z80 section at VMA `$0000` / LMA `$3DE` and resolve five files' cross-file imports within
that `$0000` space. Single-file Z80 `.emp` placement at a phase is proven
(z80_init.emp, `boot_data.asm:51`); the FIVE-file combined-module lowering at phase 0
(dodging the row-1639 `sec0` collision by lowering-as-one) is NEW at this scale and is
the first thing the execution parcel must stand up.

---

## §3 — RETIREMENT MECHANICS (every kill-row disposition)

### 3.1 The five resident `.asm` twins — DIE

| row | twin | disposition |
|---|---|---|
| 70 | `sound_psg.asm` | DELETE; `include` at `z80_sound_driver.asm:1446` removed |
| 71 | `sound_fm.asm` | DELETE; `include` at `:1437` removed |
| 78 | `sound_sequencer.asm` | DELETE; `include` at `:1421` removed |
| 83 | `sound_sfx.asm` | DELETE; `include` at `:1428` removed |
| 87 | `z80_sound_driver.asm` | DELETE; the `include` at `boot_data.asm:49` becomes the `ifndef SIGIL_EMP_Z80_SOUND` native-link arm (§1.4) |

The `.emp` files are promoted from windowed-proven to the CANONICAL build source.

### 3.2 The windowed oracles — SURVIVE TRANSFORMED

`sound_psg_port` / `sound_fm_port` / `sound_sequencer_port` / `sound_sfx_port` /
`z80_sound_driver_port` (kill-list rows name each test set). Their **AS-reassembly
halves DIE** (no `.asm` twin to `assemble()` — e.g. `z80_sound_driver_port.rs:164-177`
`driver_body()` slicing, `:237` the AS-twin oracle). They **survive transformed** into
the §2.1 whole-blob native-link gates: the byte gate stays, the comparand flips from
"AS re-assembly of the twin" to "the reference-ROM blob slice". The `plain_and_debug_
shapes_differ` and t24 positive-control / doctored-both-equal probes carry over against
the native-link output. The per-shape `link_seam` machinery (`z80_sound_driver_port.rs:
125-147`) DIES with §2.3 (addresses become imports).

### 3.3 The equ carriers / `-D` defines

- **Cross-file CALL-TARGET carriers** (the 47 intra-blob externs, §3.4) — DIE; become
  imports resolved by the linker.
- **Genuinely-external comptime constants** — the `const_seam` `-D` set
  (`z80_sound_driver_port.rs:70`, the SND_* RAM/reg map, cycle consts, the banked
  `DacSampleTable` window ptr) are NOT intra-blob. They are 68k-side
  `sound_constants.asm`-owned. Post-seam they resolve as bare link symbols /
  `extern()` consts against the surviving constants owner (or `-D` while the AS
  constants file survives) — the sound_api row-10 mirror class, unchanged by seam-1.

### 3.4 The cross-file `extern proc` decls — become IMPORTS (the 47 census)

Verified counts (grep `^extern proc` at `597ce06`):

| file | count | targets |
|---|---|---|
| `sound_psg.emp:89-91` | 3 | Snd_ChanClass←fm, Mod_ReArm/Mod_Advance←sequencer |
| `sound_fm.emp:84` | 1 | Mod_ReArm←sequencer |
| `sound_sequencer.emp:76-101` | 26 | Fm_*←fm, Psg_*/VolEnv←psg/sequencer, Snd_ChanClass←fm, Snd_StartSample/Snd_DacLookup←driver, Sfx_Frame←sfx |
| `sound_sfx.emp:70-85` | 13 | 11 co-resident (ModUpdate/Sequencer_Channel←seq, Fm_*←fm, Psg_*←psg) + 2 driver die-at-port (SndDrv_SetBank/Snd_RouteClassFlags) |
| `z80_sound_driver.emp:112-115` | 4 | Sequencer_Frame/Sequencer_StopAll/Sfx_StopAll/SfxDispatch (all resolve into sequencer/sfx) |

**Total 47 intra-blob `extern proc` decls → module-to-module `import`s.** All targets are
IN the combined module (§1.4), so every one resolves internally. The brief's tally
(psg 3 + sfx 13 + driver 4) is a subset of this 47; the sequencer's 26 and fm's 1 are the
rest. The two sfx "driver die-at-port" externs (`sound_sfx.emp:84-85`) are already
resolved (driver ported at t40); they become plain imports.

### 3.5 sound_api (rows 10 / 24 / 36 / 43) — COUPLED, DISTINCT

`sound_api` is the 68k caller, NOT in the Z80 blob (§1.2). Its retirement is a normal
68k `.emp` cutover via the ALREADY-SCAFFOLDED `SIGIL_EMP_SOUND_API` gate + org-resume arm
(`engine.inc:612-620`). Its coupled kill conditions:
- **row 10** — the SND_* immediate mirrors (SND_ALIVE_MARKER/…): split outcome already
  ruled (5 untyped retire at next touch; 2 typed SFXID_RING_* blocked on typed-extern
  grammar). Bare link names once `sound_constants.asm`/`sound_ids.asm` own them.
- **rows 24 / 36** — `stop_z80()`/`start_z80()` templates (now `engine.z80_bus.emp`,
  row 36): DIE when the LAST AS invoker of `stopZ80`/`startZ80` macros is gone (Spec-5),
  not at seam-1.
- **row 43** — the `sr_masked(code)` bracket: dies with the inline twin spellings (Spec-5).

**Disposition: sound_api can flip on its own commit (its machinery exists) either with or
before seam-1, but it is NOT part of the resident-blob native link.** Grouping it under
"the six sound twins" is a retirement-bundle convenience, not an architectural coupling.
Recommend flipping it as its own small 68k parcel to keep seam-1 purely the Z80 link.

### 3.6 Drift guards

Intra-blob drift guards die where no surviving AS reader remains — and seam-1 retires ALL
five Z80 `.asm` twins, so the intra-blob guards die. sound_api's mirror guards die when
`sound_api.asm` retires (§3.5). No guard survives INTO the resident-blob module (it has no
surviving AS twin to guard against).

---

## §4 — DRIFT DIAGNOSTICS: WHAT SHIPS WITH THE SEAM

The 5-face drift-diagnostic set is the checklist (the five faces the campaign surfaced:
extern-decl-vs-def [t36] · transitive-clobbers-completeness `[call.clobbers-incomplete]`
[t37] · the extern-preserves-overstate half · the CFG `bsr`-classifier lie [t38, the 5th
face] · cross-module contract closure). Seam-1's effect per face:

1. **extern-decl-vs-def consistency (t36, `t36-close-packet.md:107-121`)** — becomes
   **MOOT for the deleted decls.** Once the 47 externs are imports, there is no separate
   decl to drift from the def; the check is structurally unnecessary inside the blob.
   Disposition: **retire the ask for the intra-blob set** (do NOT ship a diagnostic for a
   class the link makes unrepresentable). It stays relevant only for genuinely-external
   boundary decls elsewhere in the corpus.

2. **transitive-clobbers-completeness `[call.clobbers-incomplete]` (t37,
   `t37-close-packet.md:60-80`)** — becomes **BUILDABLE and should SHIP with the seam.**
   The demanded fixpoint is `declared clobbers ⊇ reachable-callee-union ∪ local writes`.
   Today it is checker-invisible because clobbers-completeness is enforced LOCALLY, not
   transitively across `call`s (proven checker-invisible in t37). The native link is
   exactly the precondition that makes it computable: **every callee body is present in
   ONE linked module**, so the reachable-callee-union is finite and in-scope. This is the
   payoff the whole seam exists to unlock (the t37 under-claim `Sfx_Frame` iy would be
   CAUGHT). **Design: ship it as seam-1's headline diagnostic**, run over the combined
   module. It subsumes the honest-contract rule's prose fixpoint (port-loop step-2 item 9)
   into a machine check.

3. **extern-preserves-overstate half** — the symmetric partner (a stale extern claims a
   register survives that the def clobbers). Same as face 1: MOOT for intra-blob imports.

4. **CFG `bsr`-classifier lie (t38, the 5th face)** — orthogonal to the seam (a CFG
   modeling bug, not a cross-module concern). STAYS LEDGERED (kill = the next CFG toucher),
   unaffected by seam-1.

5. **cross-module contract closure** — the 47-externs→imports collapse (§3.4) IS the
   closure; with `[call.clobbers-incomplete]` (face 2) it is now verified, not asserted.

**Deferral with reasons.** `[call.clobbers-incomplete]` has one scoping question
(OQ-4): the resident blob's callees are all in-module, so the fixpoint closes over the
CODE. But a few procs `call` into the BANKED side (e.g. the driver's `Snd_StartSample`
descriptor read, `z80_sound_driver.emp:43-49` — a DATA read, not a code call, so no
clobbers edge). If any resident proc code-calls a seam-2 target, the fixpoint needs
seam-2's contracts too. From the census the resident code calls are ALL intra-blob (§3.4)
— the banked side is DATA the blob READS, not code it CALLS — so the fixpoint is complete
against seam-1 alone. Ship it; if a banked code-call surfaces at execution, defer that one
edge to seam-2 with a boundary import.

---

## §5 — SEQUENCING (one seam or two; what seam-2 owns; the generator)

**Two seams.** The board's "2 seams" splits cleanly along the phase boundary the blob
already uses (`phase 0` vs `phase 08000h`).

### Seam-1 = THE RESIDENT BLOB (this design)
The five `.emp` files (driver/sequencer/sfx/fm/psg) link as ONE native Z80 module at
VMA `$0000` / LMA `$3DE`, replacing `boot_data.asm:49`'s `include`. Retires rows
70/71/78/83/87 + the 47 intra-blob externs; ships `[call.clobbers-incomplete]`.
Code-length-neutral → downstream byte-identical.

### Seam-2 = THE BANKED / DATA SIDE
The `phase 08000h` window content the resident blob READS via banking — the `.emp`
files already ported but NOT yet natively linked, placed at pinned bank addresses:
- `dac_samples.emp` — `SIGIL_EMP_DAC`, `main.asm:312-320` (`org $58000` resume, pins
  `$48000/$50000`).
- `mt_bank.emp` — `SIGIL_EMP_MT`, `main.asm:355-425` (the Moving-Trucks song + pitch +
  patch bank; per-shape `org $5BAE8`/`$5D53A` resume).
- `sfx_bank.emp` — `SIGIL_EMP_SFX`, `main.asm:433` (the SFX blobs).
- `dac_sample_tab.emp` + `seq_opcode_tab.emp` — the engine-table head + the sequencer's
  opcode jump table, emitted in the `soundBankHead` `phase 08000h` block
  (`main.asm:349-354`; `dac_sample_tab.asm` cited at `z80_sound_driver.asm:1463-1469`).
- the `soundBankHead` tables (movingtrucks_pitchtable, sfx_blob_win_tab, `main.asm:352`).

Seam-2 is a DATA-PLACEMENT seam (bank-aligned pins, `$8000`-window addressing, the
no-straddle / window-top guards in `song_table.asm`), architecturally distinct from
seam-1's code link. Its `SND_*` window-ptr constants are the scale-2 complication ledgered
at gap-ledger row ~1623 (Z80 `db` can't carry a link symbol; the DAC-descriptor cells stay
comptime). Seam-2 owns closing that.

### sound_api = a THIRD, 68k flip (§3.5)
Independent of both seams; rides `SIGIL_EMP_SOUND_API`. Fold into seam-1's commit OR run
first as its own parcel — recommend the latter (keep seam-1 purely the Z80 link).

### The generator, relative to the flips
"The generator" = the Python sound-data tools (`tools/zyrinx_player.py
--emit-native-song`, `song_hcz2.py`, `sfx_transcode.py` — cited `main.asm:329-331/392-395`,
`:427`). They EMIT the song/patch/SFX `.asm` DATA that **seam-2's banked files** consume.
So the **generator conversion sits AFTER / ALONGSIDE seam-2** (the data side it feeds), NOT
before seam-1 — the resident code blob is generator-independent (it reads the tables at
runtime, it is not generated). And the whole sound arc (seam-1 → seam-2 → generator)
precedes the **Spec-5 main/config flip** (the twin-deletion that removes the AS toolchain
entirely). Ordering: **seam-1 (code) → seam-2 (data) → generator (data source) → Spec-5**;
sound_api can slot anywhere before Spec-5.

---

## OPEN QUESTIONS (for the execution parcel / the gate)

- **OQ-1 (mechanism)** — does sigil's Z80 lowering + link support a single combined
  module at VMA `$0000` / LMA `$3DE` with five files' cross-file imports resolving in the
  `$0000` space? Row 1639 forces lower-as-ONE (no concat). This is the first machinery the
  execution parcel stands up; the z80_init single-file placement is the proof-of-concept,
  the five-file scale is new. **This is the parcel's primary risk.**
- **OQ-2 (identity)** — confirm `Z80_Sound_End` debug = `$1BFA + $7E` exactly from
  `s4.debug.lst` (the +$7E growth); confirm the even-pad (`:1480`) parity lands the same
  in both shapes (odd/even blob length can differ by shape).
- **OQ-3 (68k survivors)** — the native-link arm must still furnish `Z80_SOUND_SIZE` (boot
  copy count, `s4.lst:6031`) and satisfy the `(Z80_Sound_Start-BootData) <> 54` +
  `Z80_SOUND_SIZE > SND_STATE_BASE` guards (`s4.lst:13551/13495`) while the AS engine
  survives — a numeric carrier per the `Z80_IDLE_SIZE = $3FE-$3D8` precedent.
- **OQ-4 (diagnostic scope)** — confirm no resident proc CODE-calls a seam-2 target
  (census says all resident calls are intra-blob; the banked side is data-read only), so
  `[call.clobbers-incomplete]` closes against seam-1 alone (§4).
- **OQ-5 (ordering pin)** — the driver→seq→sfx→fm→psg emission order is a layout fact the
  blob module must PIN (`.emp` `import` is symbol resolution, not emission ordering); how
  the module declares an ordered five-file manifest is an execution-parcel detail.
