#!/usr/bin/env python3
"""cart_check.py, the instrument for AB_PROTOCOL.md step 0: WHICH CART IS LOADED.

Every other hash in this directory covers RAM, VRAM, CRAM and the visible plane.
None of them covers the one input both arms of an A/B share: the cartridge. A stale
image therefore makes an A/B AGREE, and agreement is the direction nobody audits.
Both arms measure the identical wrong program, every region hash matches, every
screenshot `cmp`s pixel-identical, and the packet reports a clean verdict having
compared nothing. That is the standing bar *an A/B whose arms agree may have measured
nothing* arriving one layer BELOW where that bar looks: it asks what each arm
PRODUCED, and here both arms produce real, correct, identical measurements.

Step 0 was written into AB_PROTOCOL.md as prose on 2026-09-12. Prose rides on the
runner remembering to read step 0, and a hand-run instrument has no CI and no second
reader, which makes it worse to leave unwritten rather than better. This file is the
kill: the check becomes an INSTRUMENT.


WHY THE SERVER'S OWN CAVEAT IS NOT THE CHECK
--------------------------------------------
The wire contract already obliges the server to say a stale image out loud
(empyrean contract/protocol.md item 27, §11.37, CR-N, 2026-09-05): a server
advertising `status` with `romPath` MUST emit a `caveat` naming the image stale when
the file at `romPath` differs in SIZE, and the same when the sizes MATCH and the
BYTES differ, and NO ROM caveat when the bytes match, so the quiet state is a real
assertion.

That caveat answers exactly one question: *has the file at `romPath` changed on disk
since it was loaded?* It is SILENT on the question this instrument exists to answer:
*is the cart the emulator holds the cart THIS ARM MEANT?* If the emulator holds
`/a/old.bin` and this arm intended `/b/new.bin`, and `/a/old.bin` has not changed on
disk, there is NO CAVEAT AT ALL and every downstream hash still matches. So the
caveat is ONE INPUT AMONG SEVERAL here, never the check.


WHAT IS CHECKED, AND WHY EACH ROW IS LOAD-BEARING
-------------------------------------------------
1. `status.romPath` IS the intended file (`os.path.samefile`, so a symlink or a
   `..` spelling is not a false red). Paths cross a process boundary and are
   absolute on the wire (§11.30, CR-I). This row is the only one that catches the
   *held a different file that happens to be unchanged on disk* case above.
2. `status.romBytes` equals the size of the intended file on disk. Cheap, and it is
   the row that catches a truncated or half-written image.
3. THE CART'S OWN BYTES, read back over the bus and compared against the file. Two
   ways, and which one ran is reported, because they do not prove the same thing:

   - **TOTAL (preferred).** One `memory_hash {addr: 0, len: romBytes}` call: the
     server hashes the whole image and answers `crc32`, IEEE/zlib as 8 hex, "chosen
     so a hash whose `region` is exactly `"cartridge ROM"` equals CRC32 over the
     same slice of the ROM file". That is a WHOLE-IMAGE comparison in ONE round
     trip, which is why it is preferred over a readback: `read_memory` caps at 4096
     bytes a call, so pulling an 820 KB image back would be ~200 round trips, while
     the hash is one. Aeon's `tools/evict_witness.py` already runs exactly this
     shape as its stale-binary guard, and the contract names that use as the reason
     the crc32 sentence exists.
   - **SAMPLED (fallback, when the server does not serve `memory_hash`).** A byte
     sample read back through `read_memory` at offsets derived from the file's own
     size: a head window, a middle window and the final window.

4. `status.caveat`, refused when it names the cart at all. Item 27's last clause is
   that a conformant server emits NO ROM caveat when the bytes match, so any ROM
   caveat is either a stale image or a "could not check", and both are refusals
   here. A caveat about something else (§2.4 allows them) is reported, not refused.

Row 3 requires `region` to be EXACTLY `"cartridge ROM"`, on both paths. The
contract's M1 (mapper ruling, 2026-09-11) is explicit that a client comparing a
cartridge read against the ROM file MUST check for that spelling by EXACT equality
and MUST NOT assume it from the address: a bank-mapper window answers
`"cartridge ROM bank N"`, which begins with the same two words, and under it the bus
address is NOT an image offset. A prefix match would take a banked answer for an
image offset and compare the wrong bytes.


WHAT A GREEN MEANS, AND WHAT IT DOES NOT
----------------------------------------
This is the half that must not be left to a reader's charity, so the instrument says
it in its own passing line rather than only here.

- `coverage=total`: every byte of the image was compared. The only thing left
  unproven is that the server's own hash is honest.
- `coverage=sampled`: the cart is not one of the builds that differ INSIDE the
  sampled windows. **A targeted one-byte edit outside every window passes.** That is
  a real limit and not a hypothetical, and `one_byte_outside_every_window_escapes_
  the_sample` below is the case that holds it true: the sample is adequate against
  the hazard actually filed (a stale build differing in ~43% of its bytes) and is
  NOT a general identity proof. Set `CART_CHECK_REQUIRE_TOTAL=1` to make the
  fallback a refusal instead.

An instrument that overstates its own coverage is how the next version of this
defect gets written, so a passing line NEVER reads "cart verified" unqualified.


LOUD ON UNMEASURABLE, ON EVERY BRANCH
-------------------------------------
A missing `romPath`, a missing `romBytes`, a `read_memory` refusal, a short read, a
region that is not exactly `"cartridge ROM"`, an unreadable file on disk, a bus
error: ALL REFUSE. "Could not measure" is never rendered as a pass and never as 0.
The whole value of this file is that the quiet state becomes a real assertion, so an
ambiguous state returning green would reintroduce the defect under a new name.

Identity is this campaign's standard: CRC32 (IEEE/zlib) as 8 hex digits, alongside
the byte size, the form `region-hash.sh` prints. A decimal `cksum` is a DIFFERENT
checksum and a foreign convention in this lane.


USAGE
-----
As a module, from an instrument::

    from cart_check import verify_cart_bus, CartMismatch
    await verify_cart_bus(bus, ROM, label="step 0")          # before either arm
    await call(bus, "reload_rom", {"path": ROM, ...})
    await verify_cart_bus(bus, ROM, label="after reload")    # after each load

As a CLI, at step 0 of a hand-run A/B::

    python3 cart_check.py <rom-path>        # exit 0 proven, exit 1 refused

Its own fake-bus cases, which is what `cargo test -p sigil-harness --test
ab_cart_check` runs (there is no emulator in a test run, and an instrument whose
failing branch is never executed is an instrument nobody has seen work)::

    python3 cart_check.py --self-test            # every case
    python3 cart_check.py --self-test <case>     # one case
    python3 cart_check.py --list-cases
    python3 cart_check.py --list-controls

The cases live here rather than in a sibling file on purpose: the module and the
proof that it refuses must not be able to drift apart, and a runner that imports a
stale sibling would print `ok` either way.
"""

import asyncio
import os
import sys
import zlib

__all__ = [
    "CartMismatch",
    "verify_cart",
    "verify_cart_bus",
    "caller_for",
    "sample_offsets",
    "crc32_size",
    "crc32_hex",
]

# Cartridge space on the 68000 map, per the mapper ruling: `$000000-$3FFFFF`. A byte
# past that has no answer however long the image is, so neither the total hash nor a
# sample offset ever leaves it.
CART_SPACE_BYTES = 0x400000

# The exact `region` spelling that means "the bytes are the image's bytes at the same
# offset". Compared by EQUALITY, never by prefix: see M1 in the header.
REGION_FLAT = "cartridge ROM"

# One sample window's length. Well inside `read_memory`'s 4096 cap and inside every
# `limits.maxReadLen` this bus has advertised, so the instrument needs no handshake.
WINDOW_BYTES = 256

# Words that make a caveat a statement ABOUT THE CART. Item 27 requires no ROM caveat
# when the bytes match, so any caveat naming the cart is a refusal and the rest are not.
_CART_WORDS = ("rom", "cart", "image")

# The sampled fallback is a documented partial. A runner that wants the total or
# nothing exports this.
REQUIRE_TOTAL_VAR = "CART_CHECK_REQUIRE_TOTAL"


class CartMismatch(RuntimeError):
    """The loaded cart was not PROVEN to be the intended one.

    Raised for a mismatch and for an unmeasurable state alike, because a check that
    could not run is not a check that passed.
    """


def crc32_hex(data):
    """IEEE/zlib CRC-32 as 8 lowercase hex digits. The bus answers in this form too."""
    return "%08x" % (zlib.crc32(data) & 0xFFFFFFFF)


def crc32_size(data):
    """The campaign identity form: `"<crc32 as 8 hex> / <bytes>"`, as region-hash.sh prints."""
    return "%s / %d" % (crc32_hex(data), len(data))


def sample_offsets(size, window=WINDOW_BYTES):
    """Head, middle and final windows, DERIVED FROM THE FILE'S OWN SIZE.

    Constants hard-coded here would sample a fixed place in every image and would
    stop covering the tail the moment a build changed length, which is exactly where
    the filed stale image differed (166 bytes, at the end).

    Clamped to cartridge space, de-duplicated and ordered. Returns `(offset, length)`
    pairs. An empty file yields nothing, and the caller refuses on that.
    """
    usable = min(size, CART_SPACE_BYTES)
    if usable <= 0:
        return []
    n = min(window, usable)
    mid = ((usable // 2) - (n // 2)) & ~1
    offs = set()
    for off in (0, mid, usable - n):
        off = max(0, min(off, usable - n))
        offs.add(off)
    return [(off, n) for off in sorted(offs)]


def caveat_names_the_cart(caveat):
    """True when a `status.caveat` is a statement about the cartridge image."""
    if not caveat:
        return False
    low = str(caveat).lower()
    return any(w in low for w in _CART_WORDS)


def caller_for(bus):
    """An async `call(method, params)` over an `aether.BusClient`.

    `BusClient.call` returns the result and raises `BusError`; `verify_cart` catches
    every exception a call can raise and turns it into a refusal, so nothing here
    needs the instruments' own `ok is False` wrapper.
    """

    async def _call(method, params=None):
        return await bus.call(method, params or {})

    return _call


class _BusRefused(RuntimeError):
    """A bus call did not answer. Carries the reason for whoever decides what it means."""


async def _bus_call(call, bare, params):
    """Call `bare`, and on a method-not-found retry the canonical `emulator/<bare>`.

    The eighteen instruments beside this file all speak the bare spellings to this
    server; the contract catalogues the canonical `emulator/` ones. Trying both keeps
    the instrument from being the one thing in the directory that cannot talk, and
    BOTH failing still raises, so the fallback cannot turn an unmeasurable state into
    a pass.
    """
    try:
        return await call(bare, params)
    except Exception as first:
        code = getattr(first, "code", None)
        if code is not None and code != -32601:
            raise _BusRefused("%s: %r" % (bare, first))
        try:
            return await call("emulator/" + bare, params)
        except Exception as second:
            raise _BusRefused(
                "%s: %r (and emulator/%s: %r)" % (bare, first, bare, second)
            )


def _read_intended(path):
    """`(abspath, bytes)` for the intended image, or a refusal naming the reason."""
    try:
        with open(path, "rb") as fh:
            data = fh.read()
    except OSError as e:
        raise CartMismatch(
            "cannot read the intended image on disk: %s (%s).\n"
            "       Nothing downstream can be trusted: the arm cannot even say what it\n"
            "       MEANT to load, so this refuses rather than skipping the comparison."
            % (path, e)
        )
    if not data:
        raise CartMismatch("the intended image %s is empty (0 bytes)" % path)
    return os.path.abspath(path), data


async def _prove_total(call, disk):
    """The whole image, by one server-side hash. `(True, detail)` when proven.

    Returns `(False, why)` when the TOTAL route is unavailable, which is a fall-back
    signal and not a verdict; raises `CartMismatch` when the route ran and DISAGREED,
    which is a verdict and must never degrade into a sample.
    """
    size = len(disk)
    if size > CART_SPACE_BYTES:
        return False, (
            "the image is %d B, past cartridge space (%d B), so no flat range covers it"
            % (size, CART_SPACE_BYTES)
        )
    try:
        r = await _bus_call(call, "memory_hash", {"addr": "0x0", "len": size})
    except _BusRefused as e:
        return False, "memory_hash did not answer (%s)" % e
    if not isinstance(r, dict):
        return False, "memory_hash did not answer an object: %r" % (r,)

    region = r.get("region")
    if region != REGION_FLAT:
        return False, (
            "memory_hash answered region %r, not %r, so its crc32 is not CRC32 over "
            "the file's own bytes" % (region, REGION_FLAT)
        )
    got_len = r.get("len")
    if got_len is not None and int(got_len) != size:
        return False, "memory_hash hashed %s B, not the image's %d B" % (got_len, size)
    got = r.get("crc32")
    if not isinstance(got, str):
        return False, "memory_hash answered no crc32 (%r)" % (got,)

    want = crc32_hex(disk)
    # The contract spells it 8-hex bare. A `0x` prefix is accepted rather than read as
    # a mismatch, and stripped by PREFIX: `lstrip("0x")` would eat a leading `0` of a
    # hash like `0abc1234` and turn an agreement into a false red.
    got = got.lower()
    if got.startswith("0x"):
        got = got[2:]
    if got != want:
        raise CartMismatch(
            "the loaded cart's WHOLE-IMAGE hash disagrees with the intended file.\n"
            "       cart crc32 %s over %d B\n"
            "       file crc32 %s over %d B\n"
            "       The path matched and the size matched, so this is the same-size\n"
            "       different-bytes case the contract calls out as required: NOTHING\n"
            "       above this line could have caught it." % (got, size, want, size)
        )
    return True, "memory_hash crc32 %s over %d B" % (want, size)


async def _prove_sampled(call, disk, window):
    """The documented PARTIAL: head, middle and final windows through `read_memory`.

    Returns the `(offset, length)` pairs actually compared. Raises `CartMismatch` on
    a mismatch and on every unmeasurable state.
    """
    size = len(disk)
    windows = sample_offsets(size, window)
    if not windows:
        raise CartMismatch("no sampleable window in a %d B image" % size)

    sampled = []
    for off, n in windows:
        try:
            r = await _bus_call(call, "read_memory", {"addr": "0x%X" % off, "len": n})
        except _BusRefused as e:
            raise CartMismatch(
                "read_memory refused the cart window at 0x%X+%d (%s).\n"
                "       The cart cannot be sampled, so it cannot be proven." % (off, n, e)
            )
        if not isinstance(r, dict):
            raise CartMismatch("read_memory did not answer an object: %r" % (r,))

        region = r.get("region")
        if region != REGION_FLAT:
            raise CartMismatch(
                "read_memory at 0x%X answered region %r, not %r.\n"
                "       Only that exact spelling means the bytes are the image's bytes at\n"
                "       the same offset; a \"cartridge ROM bank N\" answer is a bank window\n"
                "       where the bus address is NOT an image offset (contract M1, exact\n"
                "       equality), and comparing it against the file would compare the\n"
                "       wrong bytes." % (off, region, REGION_FLAT)
            )

        hexed = r.get("bytes")
        if not isinstance(hexed, str):
            raise CartMismatch(
                "read_memory at 0x%X answered no bytes (%r)" % (off, r.get("bytes"))
            )
        try:
            got = bytes.fromhex(hexed)
        except ValueError as e:
            raise CartMismatch("read_memory at 0x%X answered unparseable bytes: %s" % (off, e))
        if len(got) != n:
            raise CartMismatch(
                "read_memory at 0x%X answered %d B for a %d B window (a short read is\n"
                "       an unmeasurable window, never a partial pass)" % (off, len(got), n)
            )

        want = disk[off : off + n]
        if got != want:
            first = next(i for i in range(n) if got[i] != want[i])
            raise CartMismatch(
                "the loaded cart DIFFERS from the intended file at image offset 0x%X.\n"
                "       first differing byte at 0x%X: cart %02X, file %02X\n"
                "       window  cart %s\n"
                "       window  file %s\n"
                "       The path matched and the size matched, so this is the same-size\n"
                "       different-bytes case the contract calls out as required." % (
                    off, off + first, got[first], want[first],
                    crc32_size(got), crc32_size(want),
                )
            )
        sampled.append((off, n))
    return sampled


async def verify_cart(call, intended_path, *, label="", window=WINDOW_BYTES, echo=True,
                      require_total=None):
    """REFUSE unless the loaded cart is PROVABLY the file at `intended_path`.

    `call` is an async callable `call(method, params) -> result` over the Aether bus
    (see `caller_for`). Returns an evidence dict on success; raises `CartMismatch` on
    a mismatch AND on any state it could not measure.

    The evidence's `coverage` is `"total"` or `"sampled"`, and a caller that records
    a green MUST record which: they are different claims. See WHAT A GREEN MEANS.
    """
    if require_total is None:
        require_total = os.environ.get(REQUIRE_TOTAL_VAR, "") not in ("", "0")
    tag = ("cart-check %s" % label).strip()
    intended, disk = _read_intended(intended_path)
    size = len(disk)

    # --- status ---------------------------------------------------------------
    try:
        status = await _bus_call(call, "status", {})
    except _BusRefused as e:
        raise CartMismatch("the bus could not answer status (%s)" % e)
    if not isinstance(status, dict):
        raise CartMismatch("status did not answer an object: %r" % (status,))

    rom_path = status.get("romPath")
    if not rom_path:
        raise CartMismatch(
            "status carries no romPath, so the server will not say which image it\n"
            "       holds and the cart CANNOT be proven. This is an unmeasurable state,\n"
            "       not a pass: intended %s" % intended
        )

    try:
        same = os.path.samefile(rom_path, intended)
    except OSError as e:
        raise CartMismatch(
            "cannot compare the loaded romPath against the intended image (%s).\n"
            "       loaded   %s\n"
            "       intended %s\n"
            "       A path that cannot be resolved is an unmeasurable state."
            % (e, rom_path, intended)
        )
    if not same:
        raise CartMismatch(
            "the emulator holds a DIFFERENT file than this arm intended.\n"
            "       loaded   %s\n"
            "       intended %s\n"
            "       Note there may be NO stale-image caveat here: the server's caveat\n"
            "       only says whether the file it loaded has changed on disk, and a\n"
            "       wrong-but-unchanged file is exactly the silent case." % (rom_path, intended)
        )

    rom_bytes = status.get("romBytes")
    if rom_bytes is None:
        raise CartMismatch(
            "status carries no romBytes, so the loaded image's length is unmeasurable\n"
            "       (intended %s is %d B)" % (intended, size)
        )
    if int(rom_bytes) != size:
        raise CartMismatch(
            "the loaded image is %d B and the intended file is %d B (%s).\n"
            "       A size disagreement is a different build, full stop."
            % (int(rom_bytes), size, intended)
        )

    caveat = status.get("caveat")
    if caveat_names_the_cart(caveat):
        raise CartMismatch(
            "status carries a caveat about the cart: %s\n"
            "       A conformant server emits NO ROM caveat when the bytes match\n"
            "       (contract item 27 / §11.37), so this is a stale image or a\n"
            "       could-not-check, and both refuse." % caveat
        )

    # --- the cart's own bytes -------------------------------------------------
    total_ok, total_detail = await _prove_total(call, disk)
    if total_ok:
        coverage, sampled = "total", []
    else:
        if require_total:
            raise CartMismatch(
                "the WHOLE-IMAGE check could not run (%s) and %s is set, so the\n"
                "       partial sample is not accepted in its place."
                % (total_detail, REQUIRE_TOTAL_VAR)
            )
        coverage = "sampled"
        sampled = await _prove_sampled(call, disk, window)

    evidence = {
        "intended": intended,
        "romPath": rom_path,
        "romBytes": int(rom_bytes),
        "fileIdentity": crc32_size(disk),
        "coverage": coverage,
        "totalDetail": total_detail,
        "sampled": [{"offset": off, "len": n} for off, n in sampled],
        "sampledBytes": sum(n for _, n in sampled),
        "caveat": caveat,
    }
    if echo:
        if coverage == "total":
            sys.stderr.write(
                "# %s: PROVEN WHOLE-IMAGE  %s  %s  (%s)\n"
                % (tag, intended, crc32_size(disk), total_detail)
            )
        else:
            sys.stderr.write(
                "# %s: PROVEN BY SAMPLE, NOT WHOLE-IMAGE  %s  %s\n"
                "#   path and size agree; bytes compared only at %s (%d of %d B).\n"
                "#   A build differing ONLY outside those windows would pass. Whole-image\n"
                "#   check unavailable: %s\n"
                % (tag, intended, crc32_size(disk),
                   " ".join("0x%X+%d" % (off, n) for off, n in sampled),
                   evidence["sampledBytes"], size, total_detail)
            )
        if caveat:
            sys.stderr.write("# %s: server caveat (not about the cart): %s\n" % (tag, caveat))
    return evidence


async def verify_cart_bus(bus, intended_path, *, label="", window=WINDOW_BYTES, echo=True,
                          require_total=None):
    """`verify_cart` over an `aether.BusClient`, which is what the instruments hold."""
    return await verify_cart(
        caller_for(bus), intended_path, label=label, window=window, echo=echo,
        require_total=require_total,
    )


# ---------------------------------------------------------------------------
# The fake-bus cases. No emulator, no socket, no oracle process.
#
# What they CANNOT establish is stated in AB_PROTOCOL.md and in the parcel report:
# that a real oracle server answers these method spellings, with these keys, and
# that its `region` really is `"cartridge ROM"` for a flat image. That half is a
# live confirmation. What they DO establish is that every refusal branch above
# actually refuses, which is the half a green run would otherwise never exercise.
# ---------------------------------------------------------------------------


class _FakeBus:
    """A scripted `call(method, params)`: the server's answers, with faults injectable.

    `serve_hash=False` is the server that does not advertise `memory_hash`, which is
    how a case reaches the SAMPLED path.
    """

    def __init__(self, image, status=None, read_error=None, region=REGION_FLAT,
                 short=False, status_error=None, serve_hash=True, hash_region=None):
        self.image = image
        self.status = status if status is not None else {}
        self.read_error = read_error
        self.region = region
        self.short = short
        self.status_error = status_error
        self.serve_hash = serve_hash
        self.hash_region = hash_region or region
        self.calls = []

    async def __call__(self, method, params=None):
        self.calls.append((method, params))
        if method in ("status", "emulator/status"):
            if self.status_error:
                raise self.status_error
            return dict(self.status)
        if method in ("memory_hash", "emulator/memory_hash"):
            if not self.serve_hash:
                e = RuntimeError("-32601: method not found")
                e.code = -32601
                raise e
            off = int(params["addr"], 16)
            n = int(params["len"])
            blob = self.image[off : off + n]
            return {"addr": off, "len": n, "region": self.hash_region,
                    "crc32": crc32_hex(blob)}
        if method in ("read_memory", "emulator/read_memory"):
            if self.read_error:
                raise self.read_error
            off = int(params["addr"], 16)
            n = int(params["len"])
            blob = self.image[off : off + n]
            if self.short:
                blob = blob[: max(0, n - 1)]
            return {"addr": off, "len": n, "region": self.region, "bytes": blob.hex()}
        raise RuntimeError("fake bus was asked for %s" % method)


SKIPPED = "skipped"


def _write(tmp, name, data):
    path = os.path.join(tmp, name)
    with open(path, "wb") as fh:
        fh.write(data)
    return path


def _image(seed, size=4096):
    """A deterministic image whose head, middle and tail all differ from each other,
    so a case cannot pass by a window happening to hold the same bytes as another."""
    return bytes(((i * 31) ^ (i >> 7) ^ seed) & 0xFF for i in range(size))


def _outside_every_window(size, window=WINDOW_BYTES):
    """An offset covered by NO sample window, for the coverage cases below.

    Derived from `sample_offsets` rather than written down, so it stays true if the
    windows ever move.
    """
    covered = set()
    for off, n in sample_offsets(size, window):
        covered.update(range(off, off + n))
    for i in range(size):
        if i not in covered:
            return i
    raise AssertionError("the sample covers the whole %d B image" % size)


def _case_exact_match_passes(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p, "romBytes": len(img)}), p


def _case_sampled_fallback_passes(tmp):
    """A server with no `memory_hash`: the documented PARTIAL still passes, loudly."""
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p, "romBytes": len(img)}, serve_hash=False), p


def _case_unrelated_caveat_passes(tmp):
    """A caveat about something else does not refuse: §2.4 allows them, and a check
    that reddened on every caveat is one a runner learns to wave through."""
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(
        img,
        {"romPath": p, "romBytes": len(img),
         "caveat": "the profiler dropped 3 frames off the shadow stack"},
    ), p


def _case_wrong_rom_path_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "intended.bin", img)
    other = _write(tmp, "held.bin", img)
    # Same BYTES at the other path on purpose: the size, the hash and the sample all
    # agree, so only the path row can catch this, and the server emits no caveat either.
    return _FakeBus(img, {"romPath": other, "romBytes": len(img)}), p


def _case_missing_rom_path_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romBytes": len(img)}), p


def _case_same_size_different_bytes_refuses(tmp):
    """The required row: two builds that share a byte count, at the same path."""
    intended = _image(0x11)
    held = bytearray(intended)
    held[len(held) // 2] ^= 0xFF
    p = _write(tmp, "rom.bin", intended)
    return _FakeBus(bytes(held), {"romPath": p, "romBytes": len(intended)}), p


def _case_same_size_different_bytes_refuses_sampled(tmp):
    """The same required row down the SAMPLED path, with the edit inside a window."""
    intended = _image(0x11)
    held = bytearray(intended)
    held[len(held) // 2] ^= 0xFF
    p = _write(tmp, "rom.bin", intended)
    return _FakeBus(bytes(held), {"romPath": p, "romBytes": len(intended)},
                    serve_hash=False), p


def _case_tail_bytes_differ_refuses(tmp):
    """The filed incident's shape: the difference lives at the END of the image."""
    intended = _image(0x11)
    held = bytearray(intended)
    held[-1] ^= 0xFF
    p = _write(tmp, "rom.bin", intended)
    return _FakeBus(bytes(held), {"romPath": p, "romBytes": len(intended)},
                    serve_hash=False), p


def _case_one_byte_outside_every_window_is_caught_by_the_total_hash(tmp):
    """The coverage case, TOTAL path. One byte, placed where no sample window looks."""
    intended = _image(0x11)
    held = bytearray(intended)
    held[_outside_every_window(len(intended))] ^= 0xFF
    p = _write(tmp, "rom.bin", intended)
    return _FakeBus(bytes(held), {"romPath": p, "romBytes": len(intended)}), p


def _case_one_byte_outside_every_window_escapes_the_sample(tmp):
    """The coverage case, SAMPLED path, and it is the ONE case here that PASSES on a
    cart that is genuinely not the intended file.

    That is not a defect in the check, it is the documented limit of a sample, and
    this case exists so the limit is a measured fact rather than a sentence in a
    docstring. It is why a sampled green says SAMPLE, NOT WHOLE-IMAGE in its own
    passing line, and why `CART_CHECK_REQUIRE_TOTAL=1` exists.
    """
    intended = _image(0x11)
    held = bytearray(intended)
    held[_outside_every_window(len(intended))] ^= 0xFF
    p = _write(tmp, "rom.bin", intended)
    return _FakeBus(bytes(held), {"romPath": p, "romBytes": len(intended)},
                    serve_hash=False), p


def _case_stale_caveat_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(
        img,
        {"romPath": p, "romBytes": len(img),
         "caveat": "the ROM image on disk has changed since it was loaded (stale)"},
    ), p


def _case_could_not_check_caveat_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(
        img,
        {"romPath": p, "romBytes": len(img),
         "caveat": "could not check the ROM image on disk"},
    ), p


def _case_missing_rom_bytes_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p}), p


def _case_wrong_rom_bytes_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p, "romBytes": len(img) - 166}), p


def _case_read_memory_error_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(
        img, {"romPath": p, "romBytes": len(img)}, serve_hash=False,
        read_error=RuntimeError("-32004: address outside every region"),
    ), p


def _case_short_read_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p, "romBytes": len(img)},
                    serve_hash=False, short=True), p


def _case_banked_region_refuses(tmp):
    """`"cartridge ROM bank 9"` starts with the same two words as the flat spelling.
    A prefix match would take it for an image offset and compare the wrong bytes.
    The hash route declines the banked region and the sample route then refuses it."""
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p, "romBytes": len(img)},
                    region="cartridge ROM bank 9"), p


def _case_status_bus_error_refuses(tmp):
    img = _image(0x11)
    p = _write(tmp, "rom.bin", img)
    return _FakeBus(img, {"romPath": p, "romBytes": len(img)},
                    status_error=RuntimeError("bus not connected")), p


def _case_unreadable_file_refuses(tmp):
    img = _image(0x11)
    p = os.path.join(tmp, "not-written.bin")
    return _FakeBus(img, {"romPath": p, "romBytes": len(img)}), p


# Two real builds of the same engine, same byte count, different bytes: the shape the
# contract calls out and the shape the filed incident had. Copied out of whatever
# worktrees are on this machine, READ ONLY, and the case SKIPS LOUDLY when they are
# gone, because a test that requires another lane's worktree goes red when somebody
# prunes it. The synthetic cases are the gate; this one is corroboration on real data.
REAL_PAIR_VARS = ("CART_CHECK_REAL_PAIR_A", "CART_CHECK_REAL_PAIR_B")
REAL_PAIR_DEFAULT = (
    "/home/volence/sonic_hacks/.aeon-sp5/s4.bin",
    "/home/volence/sonic_hacks/.aeon-land-decouple/s4.bin",
)


def _real_pair():
    a = os.environ.get(REAL_PAIR_VARS[0]) or REAL_PAIR_DEFAULT[0]
    b = os.environ.get(REAL_PAIR_VARS[1]) or REAL_PAIR_DEFAULT[1]
    if not (os.path.isfile(a) and os.path.isfile(b)):
        return None
    try:
        with open(a, "rb") as fh:
            da = fh.read()
        with open(b, "rb") as fh:
            db = fh.read()
    except OSError:
        return None
    if len(da) != len(db) or da == db:
        return None
    return a, da, b, db


def _case_real_build_pair_refuses(tmp):
    pair = _real_pair()
    if pair is None:
        return SKIPPED, SKIPPED
    a, da, _b, db = pair
    p = _write(tmp, "intended.bin", da)
    # The emulator holds the OTHER build's bytes under the intended path and the
    # intended size, so path and size both agree and only the byte comparison can fire.
    return _FakeBus(db, {"romPath": p, "romBytes": len(da)}), p


def _case_control_exact_match_declared_refusing(tmp):
    """THE POSITIVE CONTROL for this runner, not for the instrument.

    An exact-match fixture declared `refuse`. The instrument passes it, the
    expectation says it must not, so the case reports MISMATCH and the CLI exits
    non-zero. Without it, a `--self-test` that reported everything as expected no
    matter what the module did would look identical to one that works.
    """
    return _case_exact_match_passes(tmp)


# name -> (builder, expectation, is_control)
CASES = {
    "exact_match_passes": (_case_exact_match_passes, "pass", False),
    "sampled_fallback_passes": (_case_sampled_fallback_passes, "pass", False),
    "unrelated_caveat_passes": (_case_unrelated_caveat_passes, "pass", False),
    "one_byte_outside_every_window_escapes_the_sample": (
        _case_one_byte_outside_every_window_escapes_the_sample, "pass", False),
    "wrong_rom_path_refuses": (_case_wrong_rom_path_refuses, "refuse", False),
    "missing_rom_path_refuses": (_case_missing_rom_path_refuses, "refuse", False),
    "same_size_different_bytes_refuses": (
        _case_same_size_different_bytes_refuses, "refuse", False),
    "same_size_different_bytes_refuses_sampled": (
        _case_same_size_different_bytes_refuses_sampled, "refuse", False),
    "tail_bytes_differ_refuses": (_case_tail_bytes_differ_refuses, "refuse", False),
    "one_byte_outside_every_window_is_caught_by_the_total_hash": (
        _case_one_byte_outside_every_window_is_caught_by_the_total_hash, "refuse", False),
    "stale_caveat_refuses": (_case_stale_caveat_refuses, "refuse", False),
    "could_not_check_caveat_refuses": (_case_could_not_check_caveat_refuses, "refuse", False),
    "missing_rom_bytes_refuses": (_case_missing_rom_bytes_refuses, "refuse", False),
    "wrong_rom_bytes_refuses": (_case_wrong_rom_bytes_refuses, "refuse", False),
    "read_memory_error_refuses": (_case_read_memory_error_refuses, "refuse", False),
    "short_read_refuses": (_case_short_read_refuses, "refuse", False),
    "banked_region_refuses": (_case_banked_region_refuses, "refuse", False),
    "status_bus_error_refuses": (_case_status_bus_error_refuses, "refuse", False),
    "unreadable_file_refuses": (_case_unreadable_file_refuses, "refuse", False),
    "real_build_pair_refuses": (_case_real_build_pair_refuses, "refuse", False),
    "control_exact_match_declared_refusing": (
        _case_control_exact_match_declared_refusing, "refuse", True),
}


def run_case(name):
    """`(as_expected, verdict, detail)` for one case. Never raises for a case outcome.

    A case whose fixture is not on this machine answers `SKIPPED`, which counts as
    expected and PRINTS AS SKIPPED: it is never folded into the pass count.
    """
    import tempfile

    if name not in CASES:
        raise SystemExit("cart_check: no such case %r (see --list-cases)" % name)
    builder, expect, _ = CASES[name]
    with tempfile.TemporaryDirectory(prefix="cart-check-") as tmp:
        bus, intended = builder(tmp)
        if bus is SKIPPED:
            return True, SKIPPED, "fixture not on this machine"
        try:
            ev = asyncio.run(verify_cart(bus, intended, label=name, echo=False,
                                         require_total=False))
            verdict = "pass"
            detail = "%s coverage=%s %s" % (
                ev["fileIdentity"], ev["coverage"],
                ev["totalDetail"] if ev["coverage"] == "total" else
                "sampled %d B" % ev["sampledBytes"])
        except CartMismatch as e:
            verdict, detail = "refuse", str(e).splitlines()[0]
    return verdict == expect, verdict, detail


def _self_test(argv):
    names = argv or [n for n, (_, _, ctl) in CASES.items() if not ctl]
    bad = skipped = 0
    for name in names:
        ok, verdict, detail = run_case(name)
        expect = CASES[name][1]
        print("case %-56s expected %-6s got %-7s %s  %s"
              % (name, expect, verdict, "OK" if ok else "MISMATCH", detail))
        if verdict == SKIPPED:
            skipped += 1
        elif not ok:
            bad += 1
    print("SELFTEST %d cases, %d as expected, %d SKIPPED, %d MISMATCH"
          % (len(names), len(names) - bad - skipped, skipped, bad))
    return 1 if bad else 0


def _main(argv):
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    if argv[0] == "--list-cases":
        for n, (_, _, ctl) in CASES.items():
            if not ctl:
                print(n)
        return 0
    if argv[0] == "--list-controls":
        for n, (_, _, ctl) in CASES.items():
            if ctl:
                print(n)
        return 0
    if argv[0] == "--self-test":
        return _self_test(argv[1:])

    rom = argv[0]
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from suite_paths import add_empyrean_clients  # noqa: E402

    add_empyrean_clients()
    from aether import BusClient  # noqa: E402

    async def go():
        bus = BusClient(client_id="cartchk", client_name="ab-cart-check",
                        client_version="1", want_events=False)
        await bus.connect()
        try:
            return await verify_cart_bus(bus, rom, label="cli")
        finally:
            await bus.close()

    try:
        ev = asyncio.run(go())
    except CartMismatch as e:
        sys.stderr.write("CART CHECK REFUSED\n       %s\n" % e)
        return 1
    print("CART PROVEN (coverage=%s)  %s  %s"
          % (ev["coverage"], ev["intended"], ev["fileIdentity"]))
    return 0


if __name__ == "__main__":
    sys.exit(_main(sys.argv[1:]))
