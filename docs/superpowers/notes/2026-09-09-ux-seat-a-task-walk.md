# UX seat A: the task walk

Pin: sigil `b8594cf9d25210a50596a4daaaa7580862806c1b`
Branch: `ux-seat-a-task-walk`
Seat: A (task walk). Subject: the product surface (diagnostics, `--help`, README, command-line shape).
Method constraint: attempt every job using only the README and `--help`. No source reading, no test reading.

## The five jobs, written down before anything in the repo was opened

Written and committed before the README, `--help`, or any file in the tree was read. I am a
programmer who knows assembly and command-line tools and has never used this assembler. These are
the things I would actually want to do on day one, in the order I would want to do them.

1. **Find out how to drive it at all.** What is the invocation shape? Is there a subcommand? What is
   required, what is optional, where does output go, what is the default? I want to answer this
   without guessing and without reading source.

2. **Assemble one minimal 68000 source file into a binary and confirm the bytes.** The smallest
   possible end to end run: a file with two or three instructions in, a binary out, and me able to
   check that the bytes are the opcodes I expected.

3. **Break the file on purpose and see whether the error is actionable.** An undefined label, a
   misspelled mnemonic, an operand that does not fit. I want the file, the line, ideally the column,
   the offending text, and enough of a statement of the rule that I can fix it without a manual.

4. **Assemble an existing AS-dialect source file unchanged.** I have a codebase written for the old
   assembler. Can I point this tool at it as is? How do I select the compatibility dialect, and what
   does it do when it meets something it does not support?

5. **Write a small file in the modern `.emp` dialect from documentation alone, and get a symbol map
   or listing out of the build.** Two halves of the same day-one need: learn the new syntax without
   reading the compiler, and get back something I can debug the output with.

(Sections below are filled in as the walk proceeds.)
