# Emp Language — VS Code syntax highlighting

Local (unpublished) extension providing syntax highlighting for `.emp` files
(the Sigil assembler's language — see `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`).

## Install

Symlink this directory into your VS Code extensions folder and restart
VS Code (or run "Developer: Reload Window"):

    ln -s /home/volence/sonic_hacks/sigil/editors/vscode ~/.vscode/extensions/emp-language

## Uninstall

    rm ~/.vscode/extensions/emp-language

## What it covers

- emp declaration layer: keywords, builtins (`ensure`, `here`, `sizeof`, ...),
  primitive + PascalCase types, SCREAMING_CASE constants, `$`/`0b`/`%` number
  literals, `//` + `///` + `/* */` comments, `@attributes`, and declaration
  names — `proc`/`data`/`offsets`/`vars` names take the label-gold function
  scope (they are the emp analog of asm labels), `module`/`section` paths the
  namespace scope.
- Embedded instruction lines: 68k and Z80 mnemonics (with size suffixes),
  registers, `#` immediates, `.local` labels.

The grammar is hand-maintained against the parser keyword list in
`crates/sigil-frontend-emp/src/parser.rs` — when the language grows a keyword,
add it to `syntaxes/emp.tmLanguage.json` (both the keyword rules and the
macro-line negative lookahead).
