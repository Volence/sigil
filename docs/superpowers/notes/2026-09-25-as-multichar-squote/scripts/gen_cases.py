"""gen_cases.py: the test cases for tests/as_single_quoted_string.rs. Writes every
case's EXACT source as a probe file (tc_<name>.asm) and the list file; after
runall.sh has run asl on them, `gen_cases.py emit <log>` writes the Rust table from
asl's own bytes (accepted cases) or asl's error number (refused cases)."""
import os, re, sys
P = "/home/volence/sonic_hacks/.scratch/as-squote/probes"
H68 = "\tcpu 68000\n\tpadding off\n\torg 0\n"
HZ = "\tcpu z80\n\torg 0\n"
CS = "\tcharset 'A',$11\n\tcharset 'B',$22\n\tcharset 'C',$99\n\tcharset $23,$77\n"
ZCS = "\tcharset 'A',11h\n\tcharset 'B',22h\n\tcharset 'C',99h\n"
s3 = open('/home/volence/sonic_hacks/.scratch/as-squote/trees/sk-pristine/s3.asm', encoding='latin-1').read().split('\n')
S3BLK = "MessageData:\n" + '\n'.join(s3[392:420]) + "\n"

# (name, head, body). The source is head + body + "\tend\n".
CASES = [
    # the corpus: s3.asm lines 393..420 verbatim, the six refused lines among them
    ("s3_region_block", H68, S3BLK),
    # dc.b
    ("b_five", H68, "\tdc.b 'ABCDE'\n"),
    ("b_list", H68, "\tdc.b 'J',0,'UE'\n\tdc.b 6,'ABC',0\n\tdc.b 'Q'\n"),
    ("b_plus_int", H68, "\tdc.b 'AB'+1\n"),
    ("b_int_plus", H68, "\tdc.b 1+'AB'\n"),
    ("b_plus_carry", H68, "\tdc.b 'AB'+$100\n"),
    ("b_concat", H68, "\tdc.b 'A'+'B'\n"),
    ("b_mixed_quotes", H68, "\tdc.b 'AB',\"CD\",'E'\n"),
    ("b_empty", H68, "\tdc.b ''\n"),
    ("b_empty_then_char", H68, "\tdc.b '','A'\n"),
    ("b_semicolon_comma", H68, "\tdc.b 'A;B','A,B',1\n"),
    ("b_escapes", H68, "\tdc.b 'A\\x42C','A\\'B','A\"B'\n"),
    ("b_interpolation", H68, "\tdc.b 'A\\{1+1}B'\n"),
    ("b_sum_zero_is_empty", H68, "\tdc.b 'AB'+(0-$4142),$EE\n"),
    ("b_dup", H68, "\tdc.b [2]'AB'\n"),
    ("b_rept", H68, "\trept 2\n\tdc.b 'AB'\n\tendm\n"),
    ("b_macro_arg", H68, "m macro a\n\tdc.b a\n\tendm\n\tm 'ABC'\n"),
    ("b_strlen", H68, "\tdc.b strlen('ABC')\n"),
    # dc.w / dc.l: the operand rule
    ("w_fits", H68, "\tdc.w 'AB'\n"),
    ("w_one", H68, "\tdc.w 'A'\n"),
    ("w_three", H68, "\tdc.w 'ABC'\n"),
    ("w_empty", H68, "\tdc.w ''\n"),
    ("w_plus_int", H68, "\tdc.w 'AB'+1\n"),
    ("w_plus_int_zero", H68, "\tdc.w 'AB'+(1-1)\n"),
    ("w_minus_int", H68, "\tdc.w 'AB'-1\n"),
    ("w_concat", H68, "\tdc.w 'A'+'B'\n"),
    ("w_concat_parens", H68, "\tdc.w ('A'+'B')\n"),
    ("w_paren_head", H68, "\tdc.w ('A')+'B'\n"),
    ("w_paren_tail", H68, "\tdc.w 'A'+('B')\n"),
    ("w_double_tail", H68, "\tdc.w 'A'+\"B\"\n"),
    ("w_parens", H68, "\tdc.w ('AB')\n"),
    ("w_char_plus", H68, "\tdc.w 'A'+$100\n"),
    ("w_high_char", H68, "\tdc.w 'A\\x99\\x41'\n"),
    ("w_substr", H68, "\tdc.w substr('ABC',0,2)\n"),
    ("w_dup", H68, "\tdc.w [2]'ABC'\n"),
    ("w_list", H68, "\tdc.w \"A\",'BCD',$1234\n"),
    ("l_three", H68, "\tdc.l 'ABC'\n"),
    ("l_five", H68, "\tdc.l 'ABCDE'\n"),
    ("l_ends_quoted", H68, "\tdc.l 'A'+\"B\"+'C'\n"),
    ("l_ends_quoted_int", H68, "\tdc.l 'A'+1+'C'\n"),
    ("l_ends_quoted_paren", H68, "\tdc.l 'A'+('B')+'C'\n"),
    ("l_ends_quoted_call", H68, "\tdc.l 'A'+substr(\"BC\",0,1)+'D'\n"),
    ("l_group_head", H68, "\tdc.l ('A'+'B')+'C'\n"),
    ("l_minus", H68, "\tdc.l 'ABCD'-1\n"),
    # symbols carry the operand's quoting when named bare
    ("sym_equ", H68, "X equ 'AB'\n\tdc.w X\n\tdc.b X\n\tdc.w (X)\n"),
    ("sym_equ_long", H68, "X equ 'ABC'\n\tdc.w X\n"),
    ("sym_equ_concat", H68, "X equ 'A'+'B'\n\tdc.w X\n"),
    ("sym_equ_plus_int", H68, "X equ 'AB'+1\n\tdc.w X\n"),
    ("sym_in_concat", H68, "X equ 'A'\n\tdc.w X+'B'\n"),
    ("sym_copy", H68, "X equ 'AB'\nY equ (X)\n\tdc.w Y\n"),
    ("sym_set", H68, "X := 'AB'\n\tdc.w X\n\tmove.l #X,d0\n"),
    ("sym_double", H68, "X equ \"AB\"\n\tdc.w X\n"),
    # charset: a non-identity page with non-repeating targets
    ("cs_bytes", H68, CS + "\tdc.b 'ABC'\n\tdc.b 'CBA',0,'B'\n"),
    ("cs_word", H68, CS + "\tdc.w 'AB'\n\tdc.w 'ABC'\n"),
    ("cs_long_imm", H68, CS + "\tdc.l 'CAB'\n\tmove.l #'CAB',d0\n"),
    ("cs_concat", H68, CS + "\tdc.w 'A'+'B'\n\tdc.b 'AB'+'C'\n"),
    ("cs_plus_int_dropped", H68, CS + "\tdc.b 'AB'+1\n"),
    ("cs_plus_int_kept", H68, CS + "\tdc.b 'AB'+2\n"),
    ("cs_symbol_before_page", H68, "X equ 'CA'\n" + CS + "\tdc.b X\n\tdc.w X\n"),
    ("cs_symbol_after_reset", H68, CS + "X equ 'CA'\n\tcharset\n\tdc.b X\n\tdc.w X\n"),
    ("cs_escape", H68, CS + "\tdc.b 'A\\x42C'\n"),
    ("cs_target_char", H68, "\tcharset 'B',$77\n\tcharset 'A','B'\n\tcharset 'C',('B')\n\tdc.b \"AC\"\n"),
    ("cs_target_string", H68, "\tcharset 'B',$77\n\tcharset 'A','B'+0\n\tdc.b \"A\"\n"),
    ("cs_range_target", H68, "\tcharset 'A',$99\n\tcharset 'a','c','A'\n\tdc.b \"abc\"\n"),
    # z80
    ("z_db", HZ, "\tdb 'AB',0,'C'\n\tdb 'AB'+1\n\tdb ''\n"),
    ("z_dw", HZ, "\tdw 'AB'\n\tdw 'A'\n\tdw ''\n\tdw 'ABC'\n"),
    ("z_dw_plus", HZ, "\tdw 'AB'+1\n\tdw 'A'+80h\n"),
    ("z_dw_sign", HZ, "\tdw \"\\x99\\x41\"\n\tdw \"\\x7f\\x80\"\n"),
    ("z_dw_page", HZ, ZCS + "\tdw 'CAB'\n\tdw 'CA'\n\tdb 'CAB'\n\tld hl,'BC'\n"),
    ("z_ld", HZ, "\tld hl,'AB'\n\tld a,'A'\n\tcp 'A'\n"),
    # integer slots, functions, conditionals
    ("imm", H68, "\tmove.l #'ABCD',d0\n\tmove.w #'A'+'B',d0\n\tmove.w #'AB'+1,d0\n"),
    ("fn_substitutes", H68, "f function a,'a'\n\tdc.b f(66)\n"),
    ("switch_case", H68, "\tswitch 'A'\n\tcase 65\n\tdc.b 1\n\tcase \"A\"\n\tdc.b 2\n\telsecase\n\tdc.b 3\n\tendcase\n"),
    ("if_compare", H68, "\tif 65='A'\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif\n\tif 'AB'<>'AC'\n\tdc.b 3\n\tendif\n"),
    # refused by asl
    ("r_imm_empty", H68, "\tmove.w #'',d0\n"),
    ("r_imm_five", H68, "\tmove.l #'ABCDE',d0\n"),
    ("r_doubled_quote", H68, "\tdc.b 'A''B'\n"),
    ("r_unterminated", H68, "\tdc.b 'AB\n"),
    ("r_bad_escape", H68, "\tdc.b '\\q'\n"),
    ("r_charset_two", H68, "\tcharset $41,'BC'\n"),
    ("r_z80_ld_two", HZ, "\tld a,'AB'\n"),
    ("r_include", H68, "\tinclude 'p.inc'\n"),
]

def rust_str(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n').replace('\t', '\\t') + '"'

def src(head, body):
    return head + body + "\tend\n"

if len(sys.argv) > 1 and sys.argv[1] == 'emit':
    log = open(sys.argv[2]).read()
    out = []
    for name, head, body in CASES:
        blk = log.split(f"== tc_{name}.asm\n")[1].split("PROBE_END")[0]
        m = re.search(r'asl   \(exit 0\): (\S*)', blk)
        if m:
            verdict = m.group(1)
        else:
            e = re.search(r'error #(\d+)', blk)
            verdict = 'refused #' + (e.group(1) if e else '?')
        out.append((name, head, body, verdict))
    rs = []
    for name, head, body, verdict in out:
        h = 'H68' if head == H68 else 'HZ'
        rs.append(f"    Case {{ name: \"{name}\", head: {h}, body: {rust_str(body)}, asl: \"{verdict}\" }},")
    open('/home/volence/sonic_hacks/.scratch/as-squote/cases.rs.txt', 'w').write('\n'.join(rs) + '\n')
    print(f"{len(out)} cases, {sum(1 for o in out if not o[3].startswith('refused'))} accepted")
    sys.exit(0)

names = []
for name, head, body in CASES:
    open(os.path.join(P, f"tc_{name}.asm"), 'w').write(src(head, body))
    names.append(f"tc_{name}")
open(os.path.join(P, 'list_cases.txt'), 'w').write(' '.join(names))
print(len(names))
