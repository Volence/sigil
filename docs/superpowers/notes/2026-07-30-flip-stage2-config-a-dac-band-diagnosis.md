# 2026-07-30 — FLIP STAGE 2 · config_a DAC band — DIAGNOSIS (blocker 2)

Status: **DIAGNOSED with byte evidence. The overseer's mirror hypothesis is
REJECTED — the cause is an INTERNAL `align $8000` baked at the section's
AS-residual base, the general form of blocker 3 (whose fix only handled the
TRAILING align).** Evidence below; the fix + gate follow in the next commit.

## The symptom

config_b's assembled anchor `[0, 0x434d0)` is now **0 diffs** vs golden
(`92776720`) — the parallax `:=` capability closed it. config_a's anchor has
**32173 diffs**, in exactly two clusters:

```
[0x48000..0x48b4b]  2892 B  sig=0x00 gold=0x80
[0x50000..0x578c7] 30920 B  sig=0x00 gold=0x80
```

i.e. the two DAC sample banks (`dac_blip_bank` @ 0x48000, `dac_shared_bank` @
0x50000; the MovingTrucks bank @ 0x58000 is CLEAN).

## NOT the mirror

The overseer's first hypothesis — SOUND_DBG_MIRROR=1 reshapes the DAC/sound-blob
emission per GameProfile — is FALSIFIED two ways:
1. The config_a golden's DAC band is **byte-identical to the s4.debug golden**
   at 0x48000/0x50000/0x57800 (mirror is a 64-byte RAM copy proc `Sound_DebugMirror`
   at $FFB202, NOT a DAC bank feature).
2. `SIGIL_EMP_DAC_BODY_STUB` (the DSM in-memory arm) is NOT set by the native
   harness, so config_a takes the real `BINCLUDE` arm; `ensure_generated` emits
   `dac_blip_bank.bin`/`dac_shared_bank.bin` identically for every profile.

## The actual cause — internal align baked at the AS-residual base

The DAC banks are NOT their own sections. They live inside ONE big pure-data
section (`sec153530`, lma 0x257c6) that spans HeightMaps → the art BINCLUDEs →
`align $8000` → DAC blip → `align $8000` → DAC shared → `align $8000` →
MovingTrucks (main.asm:312-333, the BINCLUDE arm). Its resolved labels:

```
HeightMaps@0x257c6  ...  Dac_Temp_Blip@0x4800c  Dac_SharedBank_Start@0x5000c
MovingTrucks_Bank_Start@0x5800c
```

Every DAC label is **+0xC** off the true 0x48000/0x50000/0x58000. Proof the data
is present but shifted: native `[0x4800c..]` == golden `[0x48000..]` for 2816/2816
bytes (a clean +0xC shift); native `[0x48000..0x4800c]` is 12 bytes of 0x00.

WHY +0xC: the section's frozen/true base is HeightMaps 0x257c6 (asl-correct; the
chainer pins it there). But the sigil front end BAKED the section at its
AS-residual lma **0x257ba** (0xC lower — the residual gate skew, exactly blocker
3's "HeightMaps baked for 0x257ba"). `directive_align` (non-phase) bakes
`align $8000` as a fixed `Fill{0, pad}` with `pad = (N - pos%N)%N` computed
against the RESIDUAL base, so the pad lands DAC content on 0x48000-relative-to-
0x257ba. When the chainer re-pins the section +0xC higher, that fixed pad carries
the DAC content +0xC — to 0x4800c. asl would have recomputed the pad for the new
base (align targets ABSOLUTE N-multiples, base-independent); the baked Fill does
not.

Blocker 3's `trim_trailing_align_overshoot` recomputes only the TRAILING align.
The DAC banks need the INTERNAL bank aligns recomputed too — the general form.

## The fix (next commit)

Recompute a relocated pure-data section's internal BANK-boundary (`>= 0x8000`)
aligns in the chainer: replay baked-vs-true absolute positions in parallel; for
each minimal zero-Fill pad to a `>= 0x8000` boundary, rewrite its count so the
following content resumes on the SAME absolute boundary (`new_pad = baked_after -
true_pos`). Word-aligns (`align 2`) and bulk fills (boundary `< 0x8000`) are
untouched, so the section HEAD (HeightMaps at its true 0x257c6) is unmoved.
Byte-neutral for the pinned (sonic4) path (baked == true, no relocation). RED
gate: config_a anchor == golden `b4a6756d` prefix.
