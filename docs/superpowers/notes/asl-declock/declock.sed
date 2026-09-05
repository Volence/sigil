# declock.sed — blank asl's wall-clock stamps out of a listing / diagnostic
# stream, so that hashing the stream across repeated runs measures THE ASSEMBLER
# and not the machine's clock.
#
#     "$RUNNER" p1.asm 2>&1 | sed -E -f <this file> | md5sum
#
# ── WHY ──────────────────────────────────────────────────────────────────────
# An oracle stability check runs one probe N times and compares the streams; N
# identical streams is the evidence that a measurement taken from one run is a
# property of asl. asl stamps the wall clock into that stream, and a clock
# reading is not a property of the program being assembled.
#
# The direction of the damage matters, because nothing already banked on an
# identical hash is withdrawn by this file: identical streams still prove
# identical content, so the false-PASS direction was never open. What was open
# is the false ALARM — a batch of runs straddling a tick reports the assembler
# disagreeing with itself when only the clock moved. The cost of a false alarm
# is that the reasonable response to one is to distrust the next real finding.
#
# ── THE OVER-STRIPPING BAR ───────────────────────────────────────────────────
# A filter that removes too much turns a stability check into one that CANNOT
# FAIL, which is a worse defect than the timer it was fixing. So every rule here
# is anchored on the stamp's own shape, and `selfcheck.sh` beside this file
# proves, on literal text taken from a real listing, that the filtered stream
# still separates two runs that differ in CONTENT. Run it before trusting a
# stability table taken through this filter.
#
# ── WHAT asl STAMPS, MEASURED ────────────────────────────────────────────────
# Reference binary `s1disasm/build_tools/Linux-x86_64/asl`, md5
# 61e672562465725a8c102288a7da9098, AS 1.42 Beta [Bld 212], flags
# `-xx -n -q -A -L -U -i .`. Three stamps, in two places:
#
#   1. the page banner, once per listing page:
#        AS V1.42 Beta [Bld 212] - Source File m1.asm - Page 1 - 09/05/2026 03:53:46 AM
#   2. the DATE and TIME builtins in the symbol table. These SHARE THEIR LINES
#      with real symbols, so the line cannot be dropped whole:
#        *Ce :                             1 - | *DATE :                "09/05/2026" - |
#        *RELAXED :                        0 - | *TIME :               "03:53:46 AM" - |
#   3. the assembly-time line in the trailer, which is the one that actually
#      bites, because it is a DURATION and so moves for reasons that have
#      nothing to do with the source.
#
# WHERE THE DURATION APPEARS: in the `.lst` file when `-L` is on, and on STDOUT
# when `-q` is absent. Never on stderr. A runner that passes `-q` and no `-L`
# never sees it — that is a property of its flags, not of asl, and adding `-L`
# to such a runner reintroduces the false alarm.
#
# ── THE DURATION'S SHAPE, MEASURED — IT IS NOT ALWAYS `N.NN seconds` ─────────
# Measured on this binary by giving asl real work (a `rept` of N iterations of
# `x set x+1`, listing off, trailer read off stdout):
#
#     0.00 seconds assembly time          a trivial probe
#     0.19 seconds assembly time          400k `dc.b` lines
#     0.75 seconds assembly time          3M `dc.b` lines
#    14.77 seconds assembly time          20M iterations
#     1 minute, 17.08 seconds assembly time     100M iterations
#     3 minutes, 0.97 seconds assembly time     220M iterations
#
# So past 60 seconds asl prefixes a minute field, and pluralises it. No leading
# whitespace in any measured form, even though the trailer's SIBLING lines are
# right-aligned (`     16 lines source file`). The message catalog `as.msg` ships
# ` hour`, ` minute`, ` second` as three separate singular entries and asl
# appends the `s`, so an hours field is expected to read
# `N hours, M minutes, S.SS seconds assembly time` — the rule below accepts it,
# but that form is INFERRED FROM THE CATALOG AND WAS NOT MEASURED; an hour-long
# asl run is outside anything this campaign does.
#
# The rule is anchored at BOTH ends of the line and names its unit words, so it
# can only ever consume a whole line that is exactly a duration stamp. A line
# that merely mentions the phrase — a comment in a probe, echoed into the
# listing body — is left alone; `selfcheck.sh` case 5 is that.
#
# ── A NOTE ON HASHES ALREADY PUBLISHED ───────────────────────────────────────
# `2026-09-05-as-macro-body-label.md` carries an eleven-probe hash table taken
# through this filter's predecessor. Rule 2 below fixes a defect in that
# predecessor, so the numbers this filter produces are DIFFERENT, and that table
# does not reproduce against this file. The change is a repair, not a drift:
# every hash it moves was moved by removing a clock reading, and case 3 of
# `selfcheck.sh` is what says the repair did not also remove content.
#
# ── 1. the page banner's date, and the DATE builtin ──
# Broad, and inherited unchanged. KNOWN LATENT OVER-STRIP: it blanks any
# `NN/NN/NNNN` anywhere in the stream, including one appearing in probe source
# echoed into the listing. No probe in the tree has one today, and narrowing it
# is a deliberate act with its own fixture, not a drive-by.
s#[0-9]{2}/[0-9]{2}/[0-9]{4}#DATE#g
# ── 2. the page banner's time, and the TIME builtin ──
# asl writes the clock as a 12-hour reading WITH A MERIDIEM:
#
#     - Page 1 - 09/05/2026 03:57:21 AM
#     *TIME :               "03:57:21 AM" - |
#
# The inherited rule blanked only `NN:NN:NN`, so `AM`/`PM` SURVIVED IT, and a
# stability batch straddling noon or midnight reported the assembler disagreeing
# with itself on four lines — a guaranteed false alarm twice a day, which the
# duration's tick-straddle is only the intermittent version of. MEASURED here,
# not reasoned: `selfcheck.sh` case 1 was RED against the inherited rules on a
# fixture pair differing only in the clock, and the residue was exactly
# `DATE TIME AM` against `DATE TIME PM`.
#
# The meridiem-aware rule runs FIRST so it consumes the whole reading; the bare
# rule below it stays as the fallback for a build that prints 24-hour time (this
# binary does not, so that arm is unexercised and is inherited, not measured).
s#[0-9]{2}:[0-9]{2}:[0-9]{2} (AM|PM)#TIME#g
s#[0-9]{2}:[0-9]{2}:[0-9]{2}#TIME#g
# ── 3. the assembly-time duration ──
# A plain `0.00 seconds assembly time` maps to exactly the same replacement text
# this rule's narrower predecessor produced, so widening it changes nothing for
# a probe that assembles in under a minute — which is every probe in the tree.
# The widening is for the forms measured above, and `selfcheck.sh` case 2 is the
# red-first: the predecessor anchor is required to MISS `run3`/`run4`.
s#^([0-9]+ hours?, )?([0-9]+ minutes?, )?[0-9]+\.[0-9]+ (seconds?) assembly time$#SECONDS \3 assembly time#
