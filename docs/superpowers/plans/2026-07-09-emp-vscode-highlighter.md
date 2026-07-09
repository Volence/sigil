# .emp VS Code Syntax Highlighter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A minimal local VS Code extension that syntax-highlights `.emp` files (both the emp declaration layer and embedded 68k/Z80 instruction lines).

**Architecture:** A single hand-written TextMate grammar plus a language-configuration file, packaged as an unpublished VS Code extension in `editors/vscode/` and installed via a symlink into `~/.vscode/extensions/`. No build step, no dependencies. Spec: `docs/superpowers/specs/2026-07-09-emp-vscode-highlighter-design.md`.

**Tech Stack:** TextMate grammar JSON (Oniguruma regexes), VS Code extension manifest. Validation via `python3 -m json.tool`.

**Repo:** all paths below are relative to `/home/volence/sonic_hacks/sigil/`.

---

### Grammar design notes (read before Task 2)

TextMate matching rule that shapes this grammar: at each position the pattern list is tried in order, **leftmost match wins, ties broken by list order**. Both the mnemonic rule and the macro-line rule anchor at `^\s*` (their match starts at column 0, before any keyword match on the same line), so:

- The mnemonic rule is listed **before** everything else that could match instruction lines — intended, an instruction line should read as an instruction.
- The macro-line rule (line-leading helper calls like `routine shoot`, `anim Ani.Shoot`, `facing_abs d0`, `despawn_below_level`) would beat the keyword rule for lines like `    const FOO` by the leftmost rule, so it carries an explicit negative lookahead excluding every emp keyword. If a keyword is ever added to the language, it must be added to **both** the keyword rules and that lookahead.
- Local-label matching uses a lookbehind `(?<![\w)])` so member access (`Ani.Idle`, `Def.art`) is not colored as a label.
- SCREAMING_CASE matches before PascalCase so `WAIT_TIME` reads as a constant, not a type.
- Z80 single-letter registers (`a b c d e h l i r`) are deliberately **not** matched — they false-positive on emp identifiers like `Size{ w: 16, h: 28 }`. Multi-letter Z80 registers (`af bc de hl ix iy`) are matched. Revisit when real Z80 sections exist.

---

### Task 1: Extension scaffold (manifest, language config, README)

**Files:**
- Create: `editors/vscode/package.json`
- Create: `editors/vscode/language-configuration.json`
- Create: `editors/vscode/README.md`

- [x] **Step 1: Create `editors/vscode/package.json`**

```json
{
  "name": "emp-language",
  "displayName": "Emp Language (.emp)",
  "description": "Syntax highlighting for the Sigil .emp language (Empyrean suite).",
  "version": "0.1.0",
  "publisher": "empyrean-local",
  "license": "UNLICENSED",
  "engines": {
    "vscode": "^1.75.0"
  },
  "categories": [
    "Programming Languages"
  ],
  "contributes": {
    "languages": [
      {
        "id": "emp",
        "aliases": [
          "Emp",
          "emp"
        ],
        "extensions": [
          ".emp"
        ],
        "configuration": "./language-configuration.json"
      }
    ],
    "grammars": [
      {
        "language": "emp",
        "scopeName": "source.emp",
        "path": "./syntaxes/emp.tmLanguage.json"
      }
    ]
  }
}
```

- [x] **Step 2: Create `editors/vscode/language-configuration.json`**

```json
{
  "comments": {
    "lineComment": "//",
    "blockComment": ["/*", "*/"]
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"]
  ],
  "autoClosingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "(", "close": ")" },
    { "open": "\"", "close": "\"", "notIn": ["string", "comment"] }
  ],
  "surroundingPairs": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
    ["\"", "\""]
  ]
}
```

Note: no auto-close for `'` — single quotes are char literals but `'` also has assembly-heritage uses; auto-closing it is more annoying than helpful.

- [x] **Step 3: Create `editors/vscode/README.md`**

```markdown
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
  literals, `//` + `///` + `/* */` comments.
- Embedded instruction lines: 68k and Z80 mnemonics (with size suffixes),
  registers, `#` immediates, `.local` labels.

The grammar is hand-maintained against the parser keyword list in
`crates/sigil-frontend-emp/src/parser.rs` — when the language grows a keyword,
add it to `syntaxes/emp.tmLanguage.json` (both the keyword rules and the
macro-line negative lookahead).
```

- [x] **Step 4: Validate the JSON files parse**

Run:
```bash
python3 -m json.tool editors/vscode/package.json > /dev/null && python3 -m json.tool editors/vscode/language-configuration.json > /dev/null && echo BOTH-OK
```
Expected: `BOTH-OK`

- [x] **Step 5: Commit**

```bash
git add editors/vscode/package.json editors/vscode/language-configuration.json editors/vscode/README.md
git commit -m "feat(editors): VS Code extension scaffold for .emp highlighting"
```

---

### Task 2: The TextMate grammar

**Files:**
- Create: `editors/vscode/syntaxes/emp.tmLanguage.json`

- [x] **Step 1: Create `editors/vscode/syntaxes/emp.tmLanguage.json`**

Full content (this is the deliverable — copy verbatim):

```json
{
  "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  "name": "Emp",
  "scopeName": "source.emp",
  "patterns": [
    { "include": "#comments" },
    { "include": "#strings" },
    { "include": "#chars" },
    { "include": "#mnemonics" },
    { "include": "#keywords" },
    { "include": "#builtins" },
    { "include": "#types" },
    { "include": "#numbers" },
    { "include": "#immediates" },
    { "include": "#registers" },
    { "include": "#labels" },
    { "include": "#calls" },
    { "include": "#macro-lines" },
    { "include": "#operators" }
  ],
  "repository": {
    "comments": {
      "patterns": [
        {
          "name": "comment.block.documentation.emp",
          "match": "///(?!/).*$"
        },
        {
          "name": "comment.line.double-slash.emp",
          "match": "//.*$"
        },
        {
          "name": "comment.block.emp",
          "begin": "/\\*",
          "end": "\\*/"
        }
      ]
    },
    "strings": {
      "name": "string.quoted.double.emp",
      "begin": "\"",
      "end": "\"",
      "patterns": [
        {
          "name": "constant.character.escape.emp",
          "match": "\\\\."
        }
      ]
    },
    "chars": {
      "name": "string.quoted.single.emp",
      "match": "'(\\\\.|[^'\\\\])'"
    },
    "mnemonics": {
      "patterns": [
        {
          "match": "^\\s*((?:movea|movem|movep|moveq|move|adda|addi|addq|addx|add|suba|subi|subq|subx|sub|muls|mulu|divs|divu|andi|and|ori|or|eori|eor|not|negx|neg|clr|cmpa|cmpi|cmpm|cmp|tst|btst|bset|bclr|bchg|asl|asr|lsl|lsr|roxl|roxr|rol|ror|swap|ext|exg|lea|pea|link|unlk|jmp|jsr|jbra|jbsr|bra|bsr|bhi|bls|bcc|bcs|bne|beq|bvc|bvs|bpl|bmi|bge|blt|bgt|ble|dbra|dbf|dbt|dbhi|dbls|dbcc|dbcs|dbne|dbeq|dbvc|dbvs|dbpl|dbmi|dbge|dblt|dbgt|dble|shi|sls|scc|scs|sne|seq|svc|svs|spl|smi|sge|slt|sgt|sle|sf|st|rts|rte|rtr|trapv|trap|stop|reset|nop|illegal|tas|chk|abcd|sbcd|nbcd)(?:\\.[bwls])?)(?=\\s|$)",
          "captures": {
            "1": { "name": "support.function.mnemonic.m68k.emp" }
          }
        },
        {
          "match": "^\\s*((?:ldir|lddr|cpir|cpdr|inir|indr|otir|otdr|ldi|ldd|cpi|cpd|ini|ind|outi|outd|ld|push|pop|exx|ex|inc|dec|adc|sbc|cpl|ccf|scf|rlca|rrca|rla|rra|rlc|rrc|rl|rr|sla|sra|srl|sll|bit|set|res|jp|jr|djnz|call|ret|reti|retn|rst|out|di|ei|halt|daa|im|nop))(?=\\s|$)",
          "captures": {
            "1": { "name": "support.function.mnemonic.z80.emp" }
          }
        }
      ]
    },
    "keywords": {
      "patterns": [
        {
          "name": "keyword.control.emp",
          "match": "\\b(yield|wait_frames|return|if|else|match|loop|while|for|expect_error)\\b"
        },
        {
          "name": "storage.type.emp",
          "match": "\\b(module|proc|script|const|data|vars|var|offsets|section|struct|enum|newtype|bitfield|fn|let|use|pub|export|dispatch|patch|test|comptime|bind|equ|label|block|asm|fixed|register|mnemonic|name)\\b"
        },
        {
          "name": "storage.modifier.emp",
          "match": "\\b(in|clobbers|shows|falls_into|encoding|max_size|align|where|size|long_ptrs|word_offsets|rescale)\\b"
        }
      ]
    },
    "builtins": {
      "name": "support.function.builtin.emp",
      "match": "\\b(ensure_fatal|ensure|here|sizeof|offsetof|default|unreachable|todo)\\b"
    },
    "types": {
      "patterns": [
        {
          "name": "storage.type.primitive.emp",
          "match": "\\b[ui](8|16|32)\\b"
        },
        {
          "name": "constant.other.caps.emp",
          "match": "\\b[A-Z][A-Z0-9_]+\\b(?![a-z])"
        },
        {
          "name": "entity.name.type.emp",
          "match": "\\b[A-Z][A-Za-z0-9_]*\\b"
        }
      ]
    },
    "numbers": {
      "patterns": [
        {
          "name": "constant.numeric.hex.emp",
          "match": "\\$[0-9A-Fa-f][0-9A-Fa-f_]*"
        },
        {
          "name": "constant.numeric.binary.emp",
          "match": "\\b0b[01][01_]*\\b"
        },
        {
          "name": "constant.numeric.binary.emp",
          "match": "%[01]+\\b"
        },
        {
          "name": "constant.numeric.decimal.emp",
          "match": "\\b[0-9][0-9_]*\\b"
        }
      ]
    },
    "immediates": {
      "name": "keyword.operator.immediate.emp",
      "match": "#(?=[$%0-9A-Za-z_-])"
    },
    "registers": {
      "patterns": [
        {
          "name": "variable.language.register.m68k.emp",
          "match": "\\b(d[0-7]|a[0-7]|usp|sp|pc|sr|ccr)\\b"
        },
        {
          "name": "variable.language.register.z80.emp",
          "match": "\\b(af|bc|de|hl|ix|iy)\\b'?"
        }
      ]
    },
    "labels": {
      "name": "entity.name.label.emp",
      "match": "(?<![\\w)])\\.[A-Za-z_]\\w*"
    },
    "calls": {
      "name": "entity.name.function.emp",
      "match": "\\b[a-z_]\\w*(?=\\()"
    },
    "macro-lines": {
      "match": "^\\s*(?!(?:module|proc|script|const|data|vars|var|offsets|section|struct|enum|newtype|bitfield|fn|let|use|pub|export|dispatch|patch|test|comptime|bind|equ|label|block|asm|fixed|register|mnemonic|name|yield|wait_frames|return|if|else|match|loop|while|for|expect_error|in|clobbers|shows|falls_into|encoding|max_size|align|where|size|long_ptrs|word_offsets|rescale|ensure_fatal|ensure|here|sizeof|offsetof|default|unreachable|todo)\\b)([a-z_][a-z0-9_]*)\\b(?!\\s*[:(=,.\\[])",
      "captures": {
        "1": { "name": "entity.name.function.macro.emp" }
      }
    },
    "operators": {
      "name": "keyword.operator.emp",
      "match": "->|==|!=|<=|>=|<<|>>|&&|\\|\\||\\.\\.|[-+*/%&|^!<>=]"
    }
  }
}
```

- [x] **Step 2: Validate the JSON parses**

Run:
```bash
python3 -m json.tool editors/vscode/syntaxes/emp.tmLanguage.json > /dev/null && echo GRAMMAR-OK
```
Expected: `GRAMMAR-OK`

- [x] **Step 3: Sanity-check the keyword set against the parser**

Run:
```bash
grep -oE '"[a-z_]{2,}"' crates/sigil-frontend-emp/src/parser.rs | sort -u | tr -d '"' > /tmp/claude-1000/-home-volence-sonic-hacks-sigil/a69fcaa9-fdbf-4011-9f68-98175d88f2ad/scratchpad/parser-keywords.txt
for kw in $(cat /tmp/claude-1000/-home-volence-sonic-hacks-sigil/a69fcaa9-fdbf-4011-9f68-98175d88f2ad/scratchpad/parser-keywords.txt); do grep -q "\\b$kw\\b" editors/vscode/syntaxes/emp.tmLanguage.json || echo "MISSING: $kw"; done; echo CHECK-DONE
```
Expected: `CHECK-DONE` with no `MISSING:` lines. (Every parser keyword string must appear somewhere in the grammar — keyword rule, builtin rule, or mnemonic list.)

- [x] **Step 4: Commit**

```bash
git add editors/vscode/syntaxes/emp.tmLanguage.json
git commit -m "feat(editors): TextMate grammar for .emp (emp layer + embedded 68k/Z80)"
```

---

### Task 3: Install and verify

**Files:** none created in-repo (symlink outside the repo).

- [x] **Step 1: Symlink into the VS Code extensions folder**

Run:
```bash
ln -sfn /home/volence/sonic_hacks/sigil/editors/vscode ~/.vscode/extensions/emp-language && ls -l ~/.vscode/extensions/emp-language
```
Expected: symlink listing pointing at `/home/volence/sonic_hacks/sigil/editors/vscode`.

- [x] **Step 2: Manual verification (user checkpoint — requires VS Code)**

Restart VS Code (or "Developer: Reload Window"), then open and eyeball these three exhibits against how the 68k extension renders `.asm`:

1. `examples/game/badniks/pitcher_plant.emp` — check: `///` doc comments differ from `//`; `module`/`vars`/`const`/`offsets`/`data`/`proc` colored as keywords; `PitcherPlantV`/`ObjDef`/`Ani` as types; `WAIT_TIME`/`ATTACK_RANGE` as constants; `$60`/`-$200` numeric; `move.w`/`subq.b`/`bne`/`jbra` as mnemonics; `d0`/`a0` as registers; `#ATTACK_RANGE` immediate marker; `.draw:` and `bhi .draw` as labels; `routine shoot` / `anim Ani.Shoot` / `facing_abs d0` colored as macro calls; `timer(a0)` as call + register.
2. `examples/game/badniks/pitcher_plant_script.emp` — check: `script` body, `yield .watch` (control keyword + label), `wait_frames #WAIT_TIME`.
3. `examples/guards.emp` — check: `ensure`/`ensure_fatal` builtins, `"..."` strings with `{MAX_OBJS}` left plain inside string color, `section obj (cpu: m68000, vma: $8000)`, `(max_size: 8)` modifier.

Known acceptable quirks (do not "fix" without a reason): struct field names that shadow modifier keywords (`size:` in an ObjDef literal) take the modifier color; Z80 single-letter registers are uncolored by design.

- [x] **Step 3: Fix anything that reads wrong, re-validate, commit**

If eyeballing turns up misfires, adjust the offending regex in `emp.tmLanguage.json`, re-run the Step 2 validation command from Task 2, reload VS Code, and re-check. Commit fixes as:

```bash
git add editors/vscode/syntaxes/emp.tmLanguage.json
git commit -m "fix(editors): grammar touch-ups from exhibit eyeball pass"
```

---

## Self-review notes

- **Spec coverage:** scaffold (Task 1) ↔ spec Layout section; grammar (Task 2) ↔ spec Grammar table (all 58 parser keywords distributed across control/storage/modifier rules + `expect_error` in control; builtins; types; numbers; mnemonics; registers; immediates; labels; calls); install + eyeball (Task 3) ↔ spec Verification section. Doc-comment `////` edge, `%`-binary-only-before-binary-digit, and the Z80 single-letter register omission all match the spec's ground-truth notes.
- **Keyword double-check:** `here` is not a parser keyword (it's resolved in eval) but is highlighted as a builtin per spec; Task 2 Step 3's script checks parser → grammar direction only, so this is fine.
- **Type consistency:** the macro-line negative lookahead lists exactly the union of the three keyword rules plus builtins; `ensure_fatal` precedes `ensure` in both alternations (longest-first).
