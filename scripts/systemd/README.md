# The source-gate lane's runner

`nightly_source_gates.sh` is the backstop; these two units are what actually fire it.
They are committed because a `systemd --user` unit lives in `~/.config/systemd/user/`,
which is outside every repo — an installed-and-enabled timer that exists nowhere in
version control is invisible to any session that did not install it, and is lost with
the machine.

Install (or reinstall after editing either file here):

```sh
cp scripts/systemd/sigil-source-gates.{service,timer} ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sigil-source-gates.timer
systemctl --user list-timers          # confirm NEXT is populated
```

Verify the notification path without running the gates:

```sh
scripts/nightly_source_gates.sh --selftest-fail   # exits 1, notifies
```

`sigil-source-gates.timer` fires at 05:17, an hour behind aeon's
`aeon-effects-gates.timer` at 04:17, so the two lanes do not contend for the disk or
for the aeon repo's worktree lock. The aeon units are that repo's and are not touched
from here.

# The aeon-reference drift lane's runner

`nightly_ref_drift.sh` is SIGIL-DECOUPLE step 1's job: byte identity measured nightly and
blocking nothing. `sigil-ref-drift.{service,timer}` are what fire it, committed for the
same reason as the pair above — an installed `systemd --user` unit lives outside every
repo and is invisible to a session that did not install it.

```sh
cp scripts/systemd/sigil-ref-drift.{service,timer} ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sigil-ref-drift.timer
systemctl --user list-timers          # confirm NEXT is populated
```

Verify the notification path without running anything:

```sh
scripts/nightly_ref_drift.sh --selftest-fail   # exits 2, notifies
```

`sigil-ref-drift.timer` fires at 07:17, two hours behind `sigil-source-gates.timer` and
three behind aeon's `aeon-effects-gates.timer`. The gap is wider than the one-hour
stagger between those two because this lane compiles the assembler and builds both ROM
shapes, and it takes `git worktree` locks in both repositories while doing it.

Its exit codes are REPORTING codes and the unit declares all four as success:
0 quiet, 1 drift observed, 2 nothing measured, 3 an unverified change. A landing
consumes none of them — this lane blocks nothing by construction, and
`crates/sigil-cli/tests/drift_nightly_harness.rs` asserts that rather than trusting it.

# The switch-matrix sweep's runner

`nightly_switch_sweep.sh` runs `switch_matrix_sweep.py --cross`: every arm of every build
option Sonic 1 and Sonic 2 declare (their `.asm` ASSEMBLY OPTIONS and their `build.lua`
Settings block), built by each corpus's own `build.lua` and by sigil, compared byte for
byte, and every corner of the option space. It cannot be a cargo test: it needs `lua` and
both corpora. `sigil-switch-sweep.{service,timer}` are what fire it.

```sh
cp scripts/systemd/sigil-switch-sweep.{service,timer} ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sigil-switch-sweep.timer
systemctl --user list-timers          # confirm NEXT is populated
```

Verify the notification path without running anything:

```sh
scripts/nightly_switch_sweep.sh --selftest-fail   # exits 1, notifies
```

`sigil-switch-sweep.timer` fires at 02:17, ahead of every other lane: aeon's
`aeon-effects-gates.timer` (04:17), `sigil-source-gates.timer` (05:17) and
`sigil-ref-drift.timer` (07:17). It is placed two hours clear of the first of those
because it is the longest job of the four; read its measured wall time out of the
verdict line in `~/.local/state/sigil-switch-sweep/nightly.log`, not out of this sentence.

It builds its own sigil from a detached checkout of `origin/master` at
`~/sonic_hacks/.sigil-switch-sweep` into `~/sonic_hacks/.sigil-switch-sweep-target`, and
reads each corpus out of its committed `HEAD`, never its working tree. Exit codes follow
the source-gate lane: 0 green, 1 a finding, 2 could not run; the unit declares all three
as success because the job has already notified with the wording that tells them apart.
