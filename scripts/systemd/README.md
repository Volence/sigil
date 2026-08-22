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
