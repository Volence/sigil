# The enum parcel's landing evidence

Runner: `scripts/landing-run.sh`, copied to a run-unique path before each run,
`--aeon /home/volence/sonic_hacks/.aeon-f4ref --target` a dedicated on-disk
target dir. Both runs ended with a printed verdict, not a timeout.

## The suite, reconciled against master rather than read as a number

| | master `1078a076` | `parcel/as-enum-directives` |
|---|---|---|
| suites | 384 | **385** (+1, `as_enum_directives`) |
| passed | 4,428 | **4,441** (+13, exactly this parcel's tests) |
| failed | **1** | **1** (the same one) |
| ignored | 2 | 2 |
| skip lines | 0 | 0 |

## The one red test is master's, not this parcel's

`the_boot_read_is_inside_its_byte_bound` fails on BOTH trees:

```
boot read …/docs/OVERSEER.md is 100690 B / 100000 B: OVER by 690 B (1393 lines).
```

`docs/OVERSEER.md` is byte-identical at `1078a076` and on this branch
(`git diff 1078a076..HEAD -- docs/OVERSEER.md` is empty), and the gate was run
directly in a detached worktree of master to confirm it is red there. The remedy
is a lossless history move in that file, which is the overseer's own boot doc
and outside this parcel — it is reported, not reached for.

## `--expect-test` wants a test function name, not a suite file

The first run passed `--expect-test as_enum_directives` and the verdict said
`NAME(S) THAT DID NOT EXECUTE`. The suite HAD run — the log carries
`Running tests/as_enum_directives.rs` — but the matcher looks for a line
beginning `test ` that contains the name and ` ... `, which is a test FUNCTION
line. A suite file name appears only on the `Running` line and can never match.

The check was right and the usage was wrong. The second run named three real test
functions (`enumconf_is_not_retroactive`,
`corpus_pitch_and_note_tables_match_asl`,
`explicit_member_value_moves_the_counter`) and the expectation passed.

Worth knowing because the failure mode is the wrong way round: passing a suite
name gets you a LOUD false alarm rather than a silent false pass, which is the
safe direction, but it costs a run to discover.

## Four aeon shapes

Built twice — once from master's toolchain, once from this branch's — one shape
per `build.sh` invocation, each ROM removed before its own build and each row
stamped against the run's start time. Identical on CRC32 and size in all four
(`s4` plain `14ee2440`/719700, `s4` debug `142294b3`/737683, `demo` plain
`0c456778`/96474, `demo` debug `2e603d53`/101339).
