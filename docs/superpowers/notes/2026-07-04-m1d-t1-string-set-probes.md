# M1.D T1 — string-valued `set` / `__FSTRING` : live-asl probe matrix

**Tool:** `aeon/tools/asl` (asl 1.42 Beta Bld 212), `-cpu 68000 -q -L -U`, then `p2bin`.
**Date:** 2026-07-04. Re-runnable: probe sources reproduced inline below.

## Why this exists

T1 extends the front-end so `set` can hold a **string** value and the existing
string builtins (`substr`/`strstr`/`strlen`/`val`/`lowstring`) resolve a
**string-valued symbol** (not just a `Tok::Str` literal). The real driver is
`error_handler.asm`'s `__ErrorMessage` macros (NOT `__DEBUG__`-guarded → in the
**non-debug** ROM), which run `debugger.asm`'s `__FSTRING_GenerateArgumentsCode`
/ `__FSTRING_GenerateDecodedString`: they do `.__str: set string` and scan with
`strstr(.__str,"%<")` fanned out by a `while` loop. Probe-first: every semantic
below is established against the live binary before implementation.

## Core string-symbol semantics (all confirmed)

| probe | source (after `cpu 68000; padding off; org 0`) | bytes | meaning |
|---|---|---|---|
| p1 | `S: set "BUS ERROR"` / `dc.b substr(S,0,0)` / `dc.b 0` | `42 55 53 20 45 52 52 4F 52 00` | string-valued `set`; `substr(sym,0,0)` = whole string |
| p2 | `S: set "BUS ERROR"` / `P: set strstr(S,"%<")` / `dc.b P&$FF` | `FF` | `strstr(sym,…)` miss = **-1** |
| p3 | `S: set "BUS ERROR"` / `dc.b strlen(S)` | `09` | `strlen(sym)` = char count |
| p4 | `S: set "HELLO"` / `S: set substr(S,0,3)` / `dc.b strlen(S)` / `dc.b substr(S,0,0)` | `03 48 45 4C` | **reassign reads old value, stores new** ("HEL") |
| p5 | `S: set "$80"` / `dc.b val(S)` | `80` | `val(sym)` re-lexes+folds the symbol's string |
| p6 | `S: set ""` / `dc.b strlen(S)` / `dc.b $AA` | `00 AA` | empty string is a valid value; `strlen`=0 |

Consequence: with string symbols resolving, `.__pos: set strstr(.__str,"%<")` = -1
for a token-free string, so `while (.__pos>=0)` skips — **the loop converges**.
(Today, `strstr` on an unresolvable symbol errors → `Int(0)` → `.__pos=0` →
`while` runs to WHILE_CAP=10000 → the ~2M-diagnostic fan-out.)

## Infix `!` is **XOR**, not bitwise-OR (newly-exposed; corrects M1.C T9.1)

`__ErrorMessage`'s `.__align_flag: set (((*)&1)!1)*_eh_align_offset` needs `!` to
be XOR. Probed decisively (OR and XOR agree only when operands share no bits):

| `x!y` | bytes | XOR | OR |
|---|---|---|---|
| `(0)!1` | `01` | 1 | 1 |
| `(1)!1` | `00` | **0** | 1 |
| `(3)!1` | `02` | **2** | 3 |
| `(6)!1` | `07` | 7 | 7 |
| `5!3` | `06` | **6** | 7 |

`(1)!1=0`, `(3)!1=2`, `5!3=6` are all XOR, not OR. `expr.rs:38` (`Bang => (4,
BinOp::Or)`) is wrong; the only committed golden (`(3!4)&$FF`=7) can't tell them
apart (`3^4 == 3|4 == 7`). Fix: `BinOp::Xor`. In-scope for T1 because
`.__align_flag` sits directly on the `__ErrorMessage` emit path.

## Full `__ErrorMessage` reference bytes (representative golden)

`BusError: __ErrorMessage "BUS ERROR", _eh_default|_eh_address_error` with the
**real** `debugger.asm` macro bodies verbatim + stubs
(`MDDBG__ErrorHandler=$400`, `MDDBG__ErrorHandler_PagesController=$500`,
`DEBUGGER__EXTENSIONS__ENABLE=1`, `_eh_default=0`) emits:

```
4EB9 00000400              jsr  (MDDBG__ErrorHandler).l
42 55 53 20 45 52 52 4F 52 "BUS ERROR"
00                         null terminator (GenerateDecodedString)
A1                         (opts $01)+(_eh_return $20) | (.__align_flag $80)  [flag byte @ even PC $10]
00                         !align 2 pad byte
4EF9 00000500              jmp  (MDDBG__ErrorHandler_PagesController).l
```

`GenerateArgumentsCode "BUS ERROR"` emits **no bytes** (no `%<` token → outer
`while` skips). The flag byte lands at even PC ($10 = 6+9+1), so
`(((*)&1)!1) = (0 XOR 1) = 1` → `.__align_flag = $80`, telling the handler to
skip the pad byte the `!align 2` inserts before the `jmp`.

## Implementation checklist (semantics → code)

1. Front-end-local `str_env` (qualified-name → String); **strings never enter
   `sigil_ir::SymbolValue`** (§7.4). `SymbolValue` stays `Int | Poison` in IR.
2. `directive_set`: if RHS resolves via `eval_str`, store in `str_env` (and clear
   any int binding for that name); else the existing int path.
3. `eval_str`: resolve a bare `Ident` to its `str_env` string (qualified by the
   current scope, same as the int `set` path).
4. `BinOp::Xor` for infix `!` (+ distinguishing golden).
5. Convergence falls out of (1)-(3); keep WHILE_CAP as the backstop.
