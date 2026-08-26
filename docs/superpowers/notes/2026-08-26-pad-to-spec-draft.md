# `pad(N)` / `pad_to(N)` — draft §4.3 spec text

**THIS IS A DRAFT.** It is written for the owner's agreement and for **empyrean** to land.
Sigil does not land `.emp` language spec — `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` is the
language contract and empyrean's file. Nothing here is implemented; no crate was touched by
the parcel that wrote it. The block under "§4.3 amendment text" is written so empyrean can
lift it verbatim.

**Provenance of the authority to draft this at all.** The ruling is `docs/OVERSEER.md` queue
item 4, **RULED ADOPT 2026-08-22** — an **empyrean overseer's** ruling made under a delegation
they report in the owner's words. **This lane did not witness that utterance.** It is a peer's
report of an owner grant, and it is reversible by him or by evidence. It authorizes **this
construct**, not a general licence to add `.emp` surface.

## Revisions read

| repo | revision | what was read |
|---|---|---|
| sigil (this worktree) | `67dc3aef` | `docs/OVERSEER.md` queue item 4; `crates/sigil-frontend-emp/src/parser.rs` (`struct_decl`, `struct_field_attrs`, `region_field`, `opt_align`); `crates/sigil-frontend-emp/src/layout.rs` (`check_struct_size`, `check_struct_offsets`, the `(align: N)` check, `check_struct_odd_fields`); `crates/sigil-frontend-emp/tests/eval_layout.rs` (`field_align_*`, `vars_form_align_on_a_struct_field_is_refused_by_name`) |
| empyrean | `origin/main` `4a575c0` | `docs/SIGIL_SPEC2_LANGUAGE.md` §4.3, §4.5, §4.6, §4.8, §8.3, §9, §10 — read via `git show origin/main:…`, **never** through the sibling working tree |
| aeon | `origin/master` `bc95e32e` | `engine/level/scene_dsl.emp` (`Scene`, `sc_pad_5D`, `sc_mask_raw`, `sc_v_deform_shift_raw`, `scene()`); `docs/DEFERRED_WORK.md` ("THE CLASS" / "ALIGN"); the four literal sites named by `git grep sc_pad_5D`. Read-only; nothing in aeon or empyrean was edited. |

---

## 1. Rationale

### 1.1 What is broken

A struct that wants a **specific total size** or a **specific following offset** can say so
today only by hand-counting the bytes above it into a pad field. The number the author writes
is a **width**, and a width is a function of every field above it — so it goes stale, silently,
the moment anything above changes. `(align: N)` (sigil `6fae4d6a`) makes the staleness
**detectable**; it does not remove the hand-counted constant.

The live subject is aeon's `Scene.sc_pad_5D`. Its own comment records the incident: the pad was
hand-computed for bridge offsets 94/96, four commits added fields above it, the bridges drifted
to 119/121, and `[layout.odd-field]` fired into a warning baseline nobody re-read. Since
`(align: 2)` landed on both bridges the drift is loud — but the width is **still** hand-computed,
and the field's *name* has itself gone stale: `$5D` = 93, while aeon's own
`docs/DEFERRED_WORK.md` records that the field now sits at offset **118**.

### 1.2 What `pad_to(N)` changes, stated narrowly

It changes **which number the author writes**, not whether a written number can be wrong.
Today the author writes a *width* (volatile under every insertion above). Under `pad_to(N)` the
author writes a *target offset* (stable under every insertion above that fits the pad's slack),
and the compiler derives the width. When an insertion does **not** fit, the target is
unreachable and the build fails by name at that commit — instead of a warning drifting for four.

### 1.3 Why `(align: N)` stays MANDATORY — the decisive argument

The objection that governs this design is **silence, not authorship**. Auto-derivation converts
a **detected** defect class into an **absorbed** one: a wrong hand-counted width is loud today;
a wrong *derived* width is simply a different width, and the struct still compiles. That
argument bites `pad_to(N)` **too**, not only blanket auto-padding, because the target `N` is
authored and can be wrong for exactly the reason the width could be wrong.

So the assertion is what makes the derivation safe, and therefore **cannot be the derivation's
casualty**. `pad_to(N)` and `(align: N)` state two independent facts:

- `pad_to(N)` says **where the pad ends**. It is a placement instruction with an authored target.
- `(align: N)` says **the next field's offset has a property**. No target number can prove that;
  it is the thing the author actually cares about, and it survives every legitimate insertion.

This is a **pair, not a replacement**. `pad_to(N)` must never be read as making `(align: N)`
redundant, and `(align: N)` must never be quietly derived from a neighbouring `pad_to`.

### 1.4 The falsifier — stated so a future reader can apply it

*(Stated by the empyrean overseer at the ruling, so it is a falsifier and not a preference.)*

> **If keeping both makes the common case actively worse to write — an author supplying an
> alignment AND a pad marker where one number used to do — the ergonomics argument wins and
> the cost should be brought back.**

How to apply it: after the first ten `pad_to` sites exist in the aeon corpus, count how many
carry a `(align: N)` on the following field whose *only* purpose is to guard that pad, and ask
whether the author would rather have written one number. If the answer is yes at a majority of
sites, this design is falsified and the right construct is an alignment-derived pad — see open
question **Q1**, which is deliberately *not* decided here.

**An honest report against that falsifier, from the one real subject.** `sc_pad_5D`'s intent is
**parity**, not a target offset. Under this draft the author writes `pad_to(120)` *and*
`(align: 2)` where they previously wrote `u16` *and* `(align: 2)` — the same two numbers, one of
them now stable instead of volatile. So this subject does **not** trip the falsifier (no number
was added), but it does show that `pad_to` is a partial fit for a parity intent. The construct
is exactly right for the **total-size** and **target-offset** intents the ruling names; the
parity intent wants Q1.

### 1.5 The aeon lane's two requirements, and where this draft does not simply comply

The aeon lane own the live subject and will migrate it the day this ships, so their input is
weighed as evidence. Recorded here with the verdict on each.

**(1) The meaning of `N`: "pad until the NEXT FIELD STARTS AT N".** **Adopted, and it is what
this draft had already chosen** — arrived at independently, from a different argument (a
final pad's end offset *is* the total size, so one meaning covers both intents; and it is the
same coordinate `@ 0xNN` and `offsetof` already use). Their argument is stronger than that one
and is now the spec text's headline reading: the author writes the offset they want and
computes nothing.

**(2) A fixed-width `pad(N)` "reproduces the hazard with new syntax", so perhaps it should not
exist at struct level — or should be documented as discouraged.** **Half-adopted, and this is
where the draft argues rather than complies.**

Their claim is true of exactly one intent and false of another, and the two are not
distinguishable by syntax:

- When the width **was counted off the fields above** — `sc_pad_5D`'s case, and the whole
  motivating class — they are right: `pad(N)` is the hazard with new syntax.
- When the width **is itself the fact** — three reserved bytes, a wire record's declared
  filler, a byte the author is holding for a flag — the hazard is *inverted*. The width (3)
  is stable forever; the *offset* moves with every upstream insertion. Forcing that author to
  write `pad_to(N)` makes them maintain a number that has nothing to do with what they meant,
  and every upstream insertion turns a correct declaration into a build failure they must
  re-target by hand. That is worse than the status quo, not better.

So **both forms are specified**, and this draft does not label either "discouraged" in prose.
Two reasons prose would be the wrong instrument. First, one word must not mean two things
across two bodies: `pad(N)` already exists in `vars` region bodies (`parser.rs::region_field`),
so a struct body that *refuses* `pad` rejects a word the language has, and one that accepts it
with a different meaning is worse still — the spec has to say how the two relate either way,
and "identical meaning" is the only answer that costs nothing. Second, and this is the point
the whole file turns on: **discouraging something in prose is a warning baseline**. Documented
discouragement is precisely the mechanism that let `[layout.odd-field]` drift for four commits.

The instrument that actually works is a **lint at the site**, and the hazard has a
machine-detectable signature: a `pad(N)` immediately followed by a field carrying
`(align: N)`. A pad whose only job is to produce a property of the field below it had its
width counted off the fields above. That is `[layout.pad-hand-counted]` in the spec text
below — default-on warning, machine-applicable fix-it naming the exact `pad_to(N)` to write,
`@allow`-able for the honest reserved-bytes case. It gives the aeon lane the outcome they
asked for (a `pad(N)` in the hazard shape is called out, every time, at the line) without
taking away the form the other intent needs.

The tier of that lint — and whether it should exist at all — is **Q6/Q7** below; it is a
decision a reasonable person could make the other way.

---

## 2. §4.3 amendment text — for empyrean to lift verbatim

> **Everything between the rules below is the proposed spec text.** It is written as an addition
> to `docs/SIGIL_SPEC2_LANGUAGE.md` §4.3 plus one clarifying sentence on the existing
> no-implicit-padding bullet. It does **not** amend that bullet's claim — see §3 of this note.

---

### Structs: pad fields (`pad`, `pad_to`) — D2.NN

*(Decision number to be assigned by empyrean, who own the register. `D2.36` is the highest in
`origin/main` `4a575c0`; note the unlanded `(align: N)` text will want one too — see §9.)*

A struct body may carry **anonymous pad markers** between its fields. They are the one
construct that emits struct bytes without a field name, and they exist because a hand-counted
pad width is a constant that goes stale silently.

```
struct Scene {
    ...
    sc_transition:         u8,
    pad_to(120),                           // width DERIVED: bytes until offset 120
    sc_mask_raw:           i16 (align: 2),
    sc_v_deform_shift_raw: i16 (align: 2),
}

struct SeqChannel (size: 58) {
    ...
    pad(3),                                // width FIXED: exactly 3 bytes
}
```

- **`pad_to(expr)`** — a pad whose width the compiler **derives**. `expr` is **the offset at
  which the next declared field starts**. (Fields lay out at the next byte, so that is
  identically the offset at which the pad *ends*; the two readings can never disagree.) When
  the pad is the **last** field in the struct, its end offset **is** the struct's total size, so
  the total-size intent is expressed by the same construct in the same coordinate. Nothing is
  computed by the author: `N` is the offset they want, written down.
- **`pad(expr)`** — a fixed-width pad of `expr` bytes: the `vars`-region `pad(N)` form (§4.6)
  spelled in a struct body, identical meaning, one word one meaning. It exists for the intent
  where **the width itself is the fact** — a run of reserved bytes, a wire record whose spec
  says "3 reserved", a pad the author will later spend. It is **the wrong tool** whenever the
  width was arrived at by counting the fields above, which is the staling-constant hazard this
  section exists to kill; `[layout.pad-hand-counted]` (below) says so at the site.

Both are **contextual** (§10's headroom rule): `pad` and `pad_to` are pad markers only in field
position when immediately followed by `(`. A field *named* `pad` or `pad_to` (`pad: u8`) opens
`ident :` and keeps working, as does either identifier in expression position.

**Derivation rule.** Let `cursor` be the byte offset at which the pad begins — the running
next-byte offset of §4.3's declaration-order layout.

| case | rule |
|---|---|
| `pad(n)` | width = `n`. `n` must comptime-evaluate to an int `>= 0`. |
| `pad_to(n)`, `n > cursor` | width = `n - cursor`. |
| `pad_to(n)`, `n == cursor` | width = 0. **Legal and inert**, and still an assertion: it states that the next field already begins at `n`. This is the shape an author reaches after deleting a field above, and making it an error would fail the construct precisely when the layout is already correct — forcing the author to delete the line and lose the assertion with it. It is silent, like `align 2` at an already-even position and like `(align: 1)`. |
| `pad_to(n)`, `n < cursor` | **`[layout.pad-overflow]`, ERROR tier.** The fields above already reach past the target; there is no width that can satisfy it. |
| `pad(n)` / `pad_to(n)` with `n < 0` or non-int | **`[layout.pad-count]`, ERROR tier.** |

Pads fill with **`$00`**, matching `align`'s AS-parity fill (§4.8). Under `@as_compat` a pad
reproduces the `dc.b 0,…` run it replaces byte for byte.

**The hand-counted-pad lint.** `[layout.pad-hand-counted]` — a default-on **WARNING** on a
`pad(N)` **immediately followed by a field carrying `(align: N)`**. That pairing is the
machine-detectable signature of the hazard: a fixed-width pad whose only job is to produce a
property of the field below it had its width counted off the fields above, so it will go stale
the moment anything above changes. The fix-it is machine-applicable and names the exact
replacement. `@allow("layout.pad-hand-counted")` covers the honest case where a genuine
reserved-bytes run happens to precede an aligned field.

**Diagnostics** (§9). **These strings are a cross-repo interface — aeon fixtures assert on
exact text — so they are enumerated here and any later change to them is a cross-repo change.**
All three are modelled on the `(align: N)` diagnostic, which aeon reports is the reason that
migration was painless: name the field, name **both** offsets, give the delta with a direction,
and name a remedy that does not re-introduce a hand-counted number.

```
[layout.pad-overflow]                                        ERROR

  struct Scene: pad_to(120) before field sc_mask_raw, but the fields above it already reach
  offset 143 — over by 23 byte(s). Raise the target to 143, or remove 23 byte(s) above it.
  Do not convert this to a hand-counted width; that is the number that goes stale.

  // final-pad variant, where there is no following field to name:
  struct Scene: pad_to(120) at the end of the struct, but the fields above it already reach
  offset 143 — over by 23 byte(s). Raise the target to 143, or remove 23 byte(s) above it.
  Do not convert this to a hand-counted width; that is the number that goes stale.

[layout.pad-count]                                           ERROR

  struct Scene: pad(-1) — a pad count must be a non-negative comptime int
  struct Scene: pad_to(-1) — a pad target must be a non-negative comptime int

[layout.pad-hand-counted]                                    WARNING (default-on)

  struct Scene: pad(2) is followed by field sc_mask_raw, which declares (align: 2) — this
  width was counted off the fields above it and goes stale when any of them changes. Write
  pad_to(120) instead; the compiler computes the width and the assertion still proves it.
```

On `[layout.pad-overflow]` the pad's width is taken as **0** and layout continues, so a
`(size: N)` assertion on the same struct still prints its full field-by-field diff rather than
being suppressed behind the pad error.

**What pads do NOT do.**

- **No automatic inter-field padding.** The compiler still never inserts alignment or padding
  anywhere (the bullet above). A pad's bytes exist because the author wrote a pad line, at the
  position declaration order gives it. A struct with no pad line lays out exactly as before.
- **No reordering.** Declaration order is unchanged and remains the only layout rule.
- **Nothing implicit.** `pad_to` supplies a declared pad's **width**; it never supplies a pad's
  **existence** or its **position**. The language already lets a field's width come from a
  comptime expression (`[u8; N]`, `sizeof(T)`) — this is that, in the coordinate the author
  cares about.
- **No alignment derivation.** `pad_to(N)` targets an absolute offset, never a modulus. A pad
  whose width is derived from an alignment is a separate construct and needs its own ruling.

**Interaction with the other §4.3 mechanisms.**

- **`(align: N)` on the following field stays MANDATORY where the intent is a property.**
  `pad_to(N)` derives a width from an **authored** target, so a wrong target yields a different
  width and still compiles — the derivation moves which number can be wrong, it does not remove
  the possibility. `(align: N)` is the error-tier, per-field assertion that the property the pad
  exists to produce actually holds; it is not made redundant by the derived width and must not
  be derived from a neighbouring pad. *(This is the ruling's decisive argument: auto-derivation
  turns a detected defect class into an absorbed one, so the assertion is what makes derivation
  safe and therefore cannot be derivation's casualty.)*
- **`(size: N)` is unchanged and stays an ASSERTION.** It is never satisfied by auto-padding the
  struct's tail. A struct may carry both — `pad_to` derives during layout, `(size: N)` checks the
  result — and they compose without either becoming the other.
- **Struct literals (§4.5) are untouched.** A pad has no name, so the "name every declared field,
  always" rule has nothing to say about it and no literal gains, loses, or renames a line when a
  pad's width changes.
- **`offsetof`/`sizeof`.** A pad contributes its bytes to `sizeof`; `offsetof` cannot name one,
  because a pad is not a field you read — the language makes reading it inexpressible.
- **`[layout.odd-field]` exempts pads.** A pad is a byte run with no access width, so its size
  never makes a parity claim. (This retires a spurious subject: a 2-byte pad spelled `u16` trips
  the lint today purely because of the type its width was borrowed from.)
- **`vars` regions are unchanged in v1.** They keep `pad(N)` and the cursor-moving `@align(N)`
  (§4.6). `pad_to` is a struct-body construct only: a region's coordinate is a VMA, not a
  struct-relative offset, and `@align(N)` already moves that cursor.

**Reserved-word policy (§10).** `pad`/`pad_to` add no statement-leading keyword. They are
contextual field-position openers, the same policy `offsets` and `align` entered under.

---

## 3. Is this an exception to §4.3's no-implicit-padding rule? **No — it needs explaining, not amending.**

§4.3 says today, and this sentence is load-bearing:

> The compiler never inserts alignment or padding — Aeon runs `padding off` globally and
> hand-pads; an auto-aligning struct would silently break byte-exact ports.

That claim **stands verbatim** under this proposal, and here is precisely why:

1. **The bytes sit on a line the author wrote, in declaration order.** The rule's subject is a
   pad that appears *between two fields the author wrote adjacently*, invisibly, and differently
   from AS. `pad_to(N)` produces bytes only at a position the source names.
2. **The compiler SIZES a declared pad; it does not INSERT an undeclared one.** Existence and
   position stay entirely with the author. What is derived is a width — and a field's width has
   always been allowed to come from a comptime expression (`[u8; N]`, `sizeof(T)`); nothing in
   §4.3 requires a width to be a literal.
3. **A struct with no such line is unchanged**, so byte-identity is untouched. §8.3's
   struct-for-struct offset identity with AS `struct/endstruct`, and `@as_compat` byte-exactness,
   are both properties of structs that contain no pad marker, and neither moves.
4. **It is not alignment.** The rule's named hazard is an *auto-aligning* struct. `pad_to(N)`
   targets an absolute offset, not a modulus, so it cannot silently re-parity anything: the
   target does not change when the fields above do; the *width* does, and if it cannot, the build
   fails.

The only change §4.3 needs is **one clarifying sentence** appended to that bullet, so a reader
does not have to reconstruct the argument above:

> *(Proposed clarifying sentence, for the existing bullet:)* A `pad`/`pad_to` field is not an
> exception to this: the bytes sit on a line the author wrote, in declaration order, and the
> compiler sizes a **declared** pad rather than inserting an undeclared one.

If a future reader concludes otherwise, the falsifiable claim to attack is (2): show a case where
deriving a declared pad's width moves bytes in a struct whose source contains no pad line.

---

## 4. Byte-identity — why no existing struct changes size

- **Nothing appears where nothing is written.** Every struct in both games today contains zero
  pad markers, so every struct's layout, total size, and every field offset are bit-for-bit what
  they are now. Byte-identity is not "expected to hold"; it is vacuous.
- **`pad(N)` conversions are byte-neutral by construction.** Replacing a hand-written
  `name: [u8; N]` (or a `u16` used as two pad bytes) with `pad(N)` emits the same `N` zero bytes
  at the same offset. This is the safe first migration step and it is provable by the byte gate.
- **`pad_to(N)` conversions have a mechanical, verifiable recipe.** Choose
  `N = offsetof(T, <the field below the pad>)` read **off the build**, never counted by hand.
  That target is by definition the pad's current end offset, so the derived width equals the
  width being replaced and the conversion is byte-identical. The gate proves it; the target is
  then asserted forever after instead of recomputed.
- **`@as_compat` files.** Ported files contain no pad markers unless someone adds one, and a
  `pad(N)` added in place of a `dc.b`-run pad reproduces the run exactly (`$00` fill, §4.8
  parity). The relaxer, the diff harness, and §8.3's guarantees see no change.

---

## 5. Worked example — aeon `Scene` (`engine/level/scene_dsl.emp`, aeon `origin/master` `bc95e32e`)

Cited by symbol, per protocol; aeon is the owner's live tree and was read-only here.

### 5.1 Before (on `origin/master` today)

```
    sc_precision:          u8,
    sc_transition:         u8,
    // [~20 lines of comment explaining that this width is not self-evident, must not
    //  be trusted by eye, is a function of every field above it, and sprang once already]
    sc_pad_5D:             u16 = 0,
    sc_mask_raw:           i16 (align: 2),
    sc_v_deform_shift_raw: i16 (align: 2),
```

…and, because §4.5 names every declared field always, `sc_pad_5D: default,` is spelled at
**four** literal sites: `scene()` in `scene_dsl.emp`, twice in
`games/sonic4/test/poison/poison_budget_axis1.emp`, and once in
`games/sonic4/test/poison/poison_scene_twinkey_anchor.emp`.

Two constants are stale or staling here, not one. The width (`u16` = 2 bytes) is a function of
every field above it. And the **name** is already wrong: `$5D` = 93, while the field now sits at
offset **118** — recorded as a residual nit in aeon's own `docs/DEFERRED_WORK.md` and left
unfixed there specifically because renaming it would touch three poison fixtures.

### 5.2 After

```
    sc_precision:          u8,
    sc_transition:         u8,
    // The bridges below must start on an EVEN offset. 120 is that offset; the compiler
    // computes the width, and `(align: 2)` is the independent proof that it did what was
    // intended — a wrong target here would still compile.
    pad_to(120),
    sc_mask_raw:           i16 (align: 2),
    sc_v_deform_shift_raw: i16 (align: 2),
```

…and `sc_pad_5D: default,` is **deleted from all four literal sites** — an anonymous pad has no
name for a literal to spell.

Note what the *intermediate* spelling does here. A migration that stopped at `pad(2)` would be
byte-identical and would still be the hazard — and it would say so: a `pad(2)` immediately
followed by `sc_mask_raw: i16 (align: 2)` is exactly `[layout.pad-hand-counted]`'s signature,
so the compiler names the site and hands over `pad_to(120)` as the fix-it. The half-migration
cannot be left in place quietly.

*(120 = 118 + 2, from aeon's own record that the field sits at offset 118 and is two bytes wide.
A landing parcel must re-read the live offset off the build rather than off this draft.)*

### 5.3 What the change buys, measured against the real incident

| | before | after |
|---|---|---|
| number the author writes | a **width** (2) — a function of every field above it | a **target** (120) — unchanged by any insertion above that fits the pad's slack |
| stale second constant | the field **name** `sc_pad_5D` (`$5D` = 93, actual 118) | none — the pad is anonymous |
| the 2026-08-18→08-22 incident replayed on today's layout: four commits add 25 bytes above (94/96 → 119/121 is a 25-byte drift, per the source comment) | width silently wrong; `[layout.odd-field]` fires into a warning baseline for four commits | `[layout.pad-overflow]` at the **first** commit: cursor 118 + 25 = **143** against target **120**, over by **23**, naming `sc_mask_raw` and both offsets |
| a *wrong target* (the new failure mode) | — | still caught, by the `(align: 2)` that this construct does **not** retire |
| literal sites touched when the pad's width changes | 4 | 0 |
| `[layout.odd-field]` subjects | 1 spurious (a `u16` that is not a word) | 0 |

Note the third and fourth rows together: the derivation removes the **arithmetic**, not the
maintenance. When an insertion genuinely overflows the target, the author still updates a
number — but it is a number the diagnostic hands them, not one they compute by summing every
field above.

---

## 6. Open questions for the owner

Phrased as questions; none of these was decided here.

- **Q1 — Should there be an alignment-derived sibling (`pad_align(N)` or similar)?**
  `sc_pad_5D`'s real intent is *parity*, not a target offset, and `pad_to(120)` expresses that
  intent only indirectly. A `pad_align(2)` would say exactly what is meant and would never need
  updating. It is deliberately **not** proposed here, because it is the one form that would put
  genuinely alignment-derived padding on a source line, which is the closest thing to §4.3's
  named hazard and deserves its own ruling rather than riding in on this one. Worth having?
- **Q2 — Should `pad_to(N)` also exist in `vars` region bodies?** v1 says no (a region's
  coordinate is a VMA and `@align(N)` already moves that cursor), so `vars` keeps `pad(N)`
  unchanged. Symmetry is available later at no cost; is the asymmetry acceptable?
- **Q3 — Fill byte.** Pads fill `$00`, matching `align`. A `fill:` knob is already ledgered for
  `align` (S2-D16(c)). Any real demand for a non-zero pad fill, or is `$00` forever?
- **Q4 — Tier call: should a zero-width `pad_to` be silent?** This draft says silent, and treats
  it as a still-live assertion rather than dead code. A lint saying "this pad is inert" is the
  reasonable other answer; it would fire on a correct layout, which is why it was not taken.
- **Q5 — Spelling.** `pad_to` is the ruling's own word and is what this draft uses.
  `pad_until` / `pad_to_offset` are the alternatives that read least ambiguously about *what*
  `N` is. Keep `pad_to`?
- **Q6 — Should a fixed-width `pad(N)` exist in struct bodies at all?** The aeon lane, who own
  the live subject, argue it "reproduces the hazard with new syntax" and ask whether it should
  be omitted or documented as discouraged. This draft keeps it and argues why in §1.5 — the
  hazard is inverted for a reserved-bytes intent, and `pad(N)` already means exactly this in
  `vars` bodies. Their concern is real for the intent they have; the disagreement is only about
  whether *any* fixed-width pad is honest. Ruling wanted.
- **Q7 — Tier of `[layout.pad-hand-counted]`.** This draft answers Q6's concern with a
  default-on **warning** on the `pad(N)` + following `(align: N)` pairing, rather than with
  prose. The other reasonable answers are **error** (making the hazard shape inexpressible, at
  the cost of a false positive on a genuine reserved-bytes run before an aligned field) or
  **no lint** (prose only). Note the standing evidence against "no lint": a documented
  discouragement is a warning baseline, and one of those is what let the original drift run
  four commits. Warning, error, or nothing?

---

## 7. Grammar alternatives considered and rejected

| # | alternative | rejected because |
|---|---|---|
| 1 | **Named pad field** — `sc_pad: pad_to(120)` | Puts a non-type in type position. Worse, §4.5 names every declared field always, so the name must be spelled at every literal site (four, for `Scene`) and the *name itself* becomes a second staling constant — `sc_pad_5D` is the live proof. |
| 2 | **Attribute on the FOLLOWING field** — `sc_mask_raw: i16 @start(120)` | Reads well, but the pad bytes then belong to **no line** — they exist between two fields with no declaration of their own, which is exactly the §4.3 insertion the rule bars. It also looks identical to `@ offset` (an assertion) while moving the cursor — the precise `@align(N)`-vs-`(align: N)` confusion `parser.rs::struct_field_attrs` already refuses by name. |
| 3 | **Make `(size: N)` auto-pad the tail** | `(size: N)` is an assertion, and the ruling's whole content is that assertions must not become derivations. Make it pad and the "too small" arm of `struct S: declared size D but fields total C` stops firing forever — the exact detected→absorbed conversion this design exists to avoid. |
| 4 | **`pad_to(N)` meaning the struct's TOTAL SIZE** | Meaningless for a non-final pad and undefined for two pads; duplicates `(size: N)`, letting one struct carry the same number twice with no rule for which wins. The chosen meaning covers the total-size intent anyway — a **final** pad's end offset *is* the total size. |
| 5 | **`pad_to(N)` meaning "N bytes past the previous field"** | That is `pad(N)` with extra words. |
| 5b | **Omit the fixed-width `pad(N)` from struct bodies entirely** (the aeon lane's ask) | Weighed and declined, with the full argument in §1.5: the "hand-counted width" hazard is real for their intent and **inverted** for a reserved-bytes intent, where the width is stable and the offset is the volatile number. `pad(N)` also already means exactly this in `vars` bodies, so refusing the word in a struct body rejects a word the language has. Their outcome is delivered instead by `[layout.pad-hand-counted]`, which fires on the hazard *shape* rather than on the *form*. Open as **Q6**. |
| 6 | **Reuse the item-form `align N` (§4.8) inside a struct body** | One word would carry two coordinate systems — a section's link address and a struct-relative offset — and `align 2` inside a struct body would be alignment-derived padding on a line, which is Q1's question and not this construct's. |
| 7 | **`@align(N)` on a struct field** | Refused **by name** today (`parser.rs::struct_field_attrs`, test `eval_layout.rs::vars_form_align_on_a_struct_field_is_refused_by_name`): it is the `vars`-region cursor-mover. Any new struct-field spelling must not re-open that collision; `pad_to(…)` in *field* position collides with nothing. |

---

## 8. Parseability — checked against the real parser, not assumed

`crates/sigil-frontend-emp/src/parser.rs::struct_decl` opens every struct field with
`expect_ident("field name")` followed by `expect(&Tok::Colon)`. The proposed markers are
therefore unambiguous with **one token of lookahead and no backtracking**:

- `Ident("pad" | "pad_to")` with `peek2() == Tok::LParen` → pad marker;
- anything else, including `Ident("pad") Colon` → an ordinary field named `pad`.

This is the identical shape `region_field` already uses for the `vars` form
(`self.at_kw("pad")` at line-start). No collision with the trailing attribute list
(`struct_field_attrs` runs only after `ident : ty`), and none with `@align(N)`'s by-name refusal.

**Conclusion: the proposed grammar is implementable as specified.** Invariant 5 does not fire.

## 9. BLOCKED

Nothing. Every source named in the brief was read at a committed revision, and no item was
abandoned.

One thing worth flagging rather than treating as blocked: `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`
at `origin/main` `4a575c0` **does not yet document the `(align: N)` struct-field form at all** —
it landed in sigil at `6fae4d6a` and the spec's §4.3 is behind. The amendment text above refers
to `(align: N)` as an existing mechanism, which is true of the *compiler* and not yet of the
*spec*. Whoever lands this should land the `(align: N)` §4.3 text in the same pass, or the
mandatory-pairing rule will reference a construct the spec never introduced.
