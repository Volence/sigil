"""gen_probes.py: write one probe file per escape form / context into probes/.

One construct per file, because any asl error voids every value in the same
file. Each probe ends with a `dc.b $EE` sentinel line so the construct's own
bytes are isolated in the listing.
"""
import os, string

D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'probes')
os.makedirs(D, exist_ok=True)
HEAD68 = '\tcpu 68000\n\tpadding off\n\torg 0\n'
HEADZ80 = '\tcpu z80\n\torg 0\n'
TAIL = '\tdc.b $EE\n\tend\n'
TAILZ80 = '\tdb 0EEh\n\tend\n'

P = {}

def db(name, lit, pre=''):
    P[name] = HEAD68 + pre + '\tdc.b "%s"\n' % lit + TAIL

# A. dc.b, identity page: every letter, both cases
for c in string.ascii_letters:
    db('a_letter_%s%s' % ('lc' if c.islower() else 'uc', c), '\\' + c)
# punctuation
PUN = {'bslash': '\\\\', 'dquote': '\\"', 'squote': "\\'", 'qmark': '\\?', 'space': '\\ ',
       'dot': '\\.', 'comma': '\\,', 'semi': '\\;', 'dollar': '\\$', 'hash': '\\#',
       'at': '\\@', 'pct': '\\%', 'amp': '\\&', 'lpar': '\\(', 'rpar': '\\)',
       'star': '\\*', 'plus': '\\+', 'minus': '\\-', 'slash': '\\/', 'colon': '\\:',
       'lt': '\\<', 'eq': '\\=', 'gt': '\\>', 'lbrk': '\\[', 'rbrk': '\\]',
       'caret': '\\^', 'uscore': '\\_', 'btick': '\\`', 'pipe': '\\|', 'tilde': '\\~',
       'rbrace': '\\}', 'bang': '\\!'}
for k, v in PUN.items():
    db('a_punct_' + k, v)
# decimal / octal digit forms
for k in ['0', '1', '7', '8', '9', '00', '07', '08', '010', '012', '12', '99', '123',
          '255', '256', '999', '1234', '0123', '0000', '0377', '08A']:
    db('a_num_' + k, '\\' + k)
db('a_num_12then_x', '\\12x')
db('a_num_65B', '\\65B')
# hex forms
for k in ['x0', 'x4', 'x41', 'x7f', 'xff', 'xFF', 'X41', 'x414', 'x041', 'x', 'xG',
          'x4G', 'x100', 'xab', 'xAb', 'x41B']:
    db('a_hex_' + k, '\\' + k)
# a backslash as the last character of the string
P['a_end_bslash'] = HEAD68 + '\tdc.b "ab\\"\n' + TAIL
P['a_end_bslash_comma'] = HEAD68 + '\tdc.b "ab\\",$11\n' + TAIL
# an escaped quote does not end the string
P['a_dquote_mid'] = HEAD68 + '\tdc.b "a\\"b"\n' + TAIL
P['a_dquote_semi'] = HEAD68 + '\tdc.b "a\\";b"\n' + TAIL
P['a_dquote_comma'] = HEAD68 + '\tdc.b "a\\",b",$11\n' + TAIL
# a mixed run, the Sonic 2 shape
db('a_mixed_s2', '\\x3B\\2\\4\\6\\8\\xA\\xC\\xE\\x10')
db('a_mixed_hex_then_letter', '\\x4Z')

# B. contexts, identity page
P['b_char_x41'] = HEAD68 + "\tdc.w '\\x41'\n" + TAIL
P['b_char_65'] = HEAD68 + "\tdc.w '\\65'\n" + TAIL
P['b_char_H'] = HEAD68 + "\tdc.w '\\H'\n" + TAIL
P['b_char_h'] = HEAD68 + "\tdc.w '\\h'\n" + TAIL
P['b_char_squote'] = HEAD68 + "\tdc.w '\\''\n" + TAIL
P['b_char_bslash'] = HEAD68 + "\tdc.w '\\\\'\n" + TAIL
P['b_char_n'] = HEAD68 + "\tdc.w '\\n'\n" + TAIL
P['b_char_dquote'] = HEAD68 + "\tdc.w '\\\"'\n" + TAIL
P['b_char_four'] = HEAD68 + "\tdc.l '\\x41\\x42\\x43\\x44'\n" + TAIL
P['b_char_mixed'] = HEAD68 + "\tdc.l 'A\\x42C\\68'\n" + TAIL
P['b_char_imm'] = HEAD68 + "\tmove.w #'\\x41',d0\n" + TAIL
P['b_char_db'] = HEAD68 + "\tdc.b '\\x41'\n" + TAIL
P['b_expr_str2'] = HEAD68 + '\tmove.l #"\\x41\\x42",d0\n' + TAIL
P['b_expr_strH'] = HEAD68 + '\tmove.w #"\\H",d0\n' + TAIL
P['b_expr_str_arith'] = HEAD68 + '\tmove.w #"\\x41"+1,d0\n' + TAIL
P['b_expr_str5'] = HEAD68 + '\tmove.l #"\\x41\\x42\\x43\\x44",d0\n' + TAIL
P['b_charset_tgt'] = HEAD68 + '\tcharset \'A\',"\\x10\\11\\x12"\n\tdc.b "ABC"\n' + TAIL
P['b_charset_tgt_bslash'] = HEAD68 + '\tcharset \'A\',"\\\\\\""\n\tdc.b "AB"\n' + TAIL
P['b_charset_src_H'] = HEAD68 + '\tcharset \'\\H\',"\\x39\\x37\\x38"\n\tdc.b "\'()"\n' + TAIL
P['b_charset_src_x'] = HEAD68 + '\tcharset \'\\x41\',$11\n\tdc.b "A"\n' + TAIL
P['b_charset_count'] = HEAD68 + '\tcharset $FD,"\\x10\\11\\x12"\n\tdc.b $FD\n' + TAIL
P['b_warning'] = HEAD68 + '\twarning "w\\x41\\66\\H\\\\z"\n' + TAIL
P['b_message'] = HEAD68 + '\tmessage "m\\x41\\66\\H\\\\z"\n' + TAIL
P['b_macro_arg'] = HEAD68 + 'm\tmacro a\n\tdc.b a\n\tendm\n\tm "\\x41\\66"\n' + TAIL
P['b_macro_arg_quote'] = HEAD68 + 'm\tmacro a,b\n\tdc.b a\n\tdc.b b\n\tendm\n\tm "\\"x,y",$11\n' + TAIL
P['b_macro_arg_instr'] = HEAD68 + 'm\tmacro a\n\tdc.b "<a>"\n\tendm\n\tm \\x41\n' + TAIL
P['b_strlen'] = HEAD68 + '\tdc.b strlen("\\x41\\66\\\\")\n' + TAIL
P['b_strcmp'] = HEAD68 + '\tdc.b "\\x41"="A"\n' + TAIL
P['b_substr'] = HEAD68 + '\tdc.b substr("\\x41\\x42\\x43",1,1)\n' + TAIL
P['b_set_str'] = HEAD68 + 's\t:= "\\x41\\66"\n\tdc.b s\n\tdc.b strlen(s)\n' + TAIL
P['b_equ_str'] = HEAD68 + 's\tequ "\\x41\\66"\n\tdc.b s\n' + TAIL
P['b_dcw_str'] = HEAD68 + '\tdc.w "\\x41\\66"\n' + TAIL
P['b_z80_db'] = HEADZ80 + '\tdb "\\x41\\66\\H"\n' + TAILZ80
P['b_z80_char'] = HEADZ80 + "\tld a,'\\x41'\n" + TAILZ80
P['b_interp_after_esc'] = HEAD68 + 'n\tequ 5\n\tdc.b "\\x41\\{n}"\n' + TAIL
P['b_bslash_brace'] = HEAD68 + 'n\tequ 5\n\tdc.b "\\\\{n}"\n' + TAIL

# C. the charset interaction: a NON-identity page is live
PG = "\tcharset $41,$11\n\tcharset $27,$55\n\tcharset $5C,$66\n\tcharset $07,$77\n\tcharset $0A,$AA\n\tcharset $22,$BB\n"
P['c_db_x41'] = HEAD68 + PG + '\tdc.b "A\\x41\\65"\n' + TAIL
P['c_db_H'] = HEAD68 + PG + '\tdc.b "\'\\H"\n' + TAIL
P['c_db_bslash'] = HEAD68 + PG + '\tdc.b "\\\\"\n' + TAIL
P['c_db_A'] = HEAD68 + PG + '\tdc.b "\\A"\n' + TAIL
P['c_db_n'] = HEAD68 + PG + '\tdc.b "\\n"\n' + TAIL
P['c_db_dquote'] = HEAD68 + PG + '\tdc.b "\\""\n' + TAIL
P['c_char_x41'] = HEAD68 + PG + "\tdc.w '\\x41'\n" + TAIL
P['c_char_H'] = HEAD68 + PG + "\tdc.w '\\H'\n" + TAIL
P['c_char_plain'] = HEAD68 + PG + "\tdc.w 'A'\n" + TAIL
P['c_expr_x41'] = HEAD68 + PG + '\tmove.w #"\\x41",d0\n' + TAIL
P['c_charset_src_H'] = HEAD68 + PG + "\tcharset '\\H',$99\n\tdc.b \"'U\"\n" + TAIL
P['c_charset_tgt_raw'] = HEAD68 + PG + '\tcharset $50,"\\x41A"\n\tdc.b "PQ"\n' + TAIL
P['c_macro_arg'] = HEAD68 + PG + 'm\tmacro a\n\tdc.b a\n\tendm\n\tm "\\x41"\n' + TAIL
P['c_z80_db'] = HEADZ80 + PG + '\tdb "\\x41"\n' + TAILZ80
P['c_set_str'] = HEAD68 + PG + 's\t:= "\\x41"\n\tdc.b s\n' + TAIL

# D. follow-ups: octal and decimal details, Z80, interpolation, more contexts
for k in ['01234', '0400', '0777', '09', '018', '0x41', '00012', '1a', '18', '0a']:
    db('d_num_' + k, '\\' + k)
P['d_z80_db'] = HEADZ80 + '\tdb "\\x41\\66\\H"\n' + TAILZ80
P['d_z80_char'] = HEADZ80 + "\tld a,'\\x41'\n" + TAILZ80
P['d_z80_page'] = HEADZ80 + "\tcharset 41h,11h\n\tdb \"A\\x41\\65\"\n\tld a,'\\x41'\n" + TAILZ80
P['d_macro_arg_quote'] = HEAD68 + 'mq\tmacro pa,pb\n\tdc.b pa\n\tdc.b pb\n\tendm\n\tmq "\\"x,y",$11\n' + TAIL
P['d_macro_arg_quote2'] = HEAD68 + 'mq\tmacro pa\n\tdc.b pa\n\tendm\n\tmq "a\\";b"\n' + TAIL
P['d_error_text'] = HEAD68 + '\terror "e\\x41\\66\\H"\n' + TAIL
P['d_val'] = HEAD68 + '\tdc.b val("\\x31\\x32")\n' + TAIL
P['d_lowstring'] = HEAD68 + '\tdc.b lowstring("\\x41")\n' + TAIL
P['d_char_interp'] = HEAD68 + "n\tequ 5\n\tdc.w '\\{n}'\n" + TAIL
P['d_expr_interp'] = HEAD68 + 'n\tequ 5\n\tmove.w #"\\{n}",d0\n' + TAIL
P['d_set_bslash_brace'] = HEAD68 + 'n\tequ 5\ns\t:= "\\\\{n}"\n\tdc.b s\n' + TAIL
P['d_message_bslash_brace'] = HEAD68 + 'n\tequ 5\n\tmessage "\\\\{n}|\\{n}"\n' + TAIL
P['d_name_brace_esc'] = HEAD68 + 'nameA\tequ $33\n\tdc.b name{"\\x41"}\n' + TAIL
P['d_charset_tgt_interp'] = HEAD68 + 'n\tequ $12\n\tcharset \'A\',"\\{n}"\n\tdc.b "AB"\n' + TAIL
P['d_set_interp_then_esc'] = HEAD68 + 'n\tequ 5\ns\t:= "\\x41\\{n}\\x42"\n\tdc.b s\n\tdc.b strlen(s)\n' + TAIL
P['d_esc_in_interp_expr'] = HEAD68 + 'n\tequ 5\n\tmessage "<\\{n+\\x31}>"\n' + TAIL
P['d_squote_in_dq'] = HEAD68 + '\tdc.b "it\'s"\n' + TAIL
P['d_dquote_in_sq'] = HEAD68 + "\tdc.w '\"'\n" + TAIL
P['d_char_bad_escape'] = HEAD68 + "\tdc.w '\\c'\n" + TAIL
P['d_expr_bad_escape'] = HEAD68 + '\tmove.w #"\\c",d0\n' + TAIL
P['d_charset_bad_escape'] = HEAD68 + '\tcharset \'A\',"\\c"\n' + TAIL
P['d_message_bad_escape'] = HEAD68 + '\tmessage "\\c"\n' + TAIL
P['d_set_bad_escape'] = HEAD68 + 's\t:= "\\c"\n' + TAIL
P['d_if0_bad_escape'] = HEAD68 + '\tif 0\n\tdc.b "\\c"\n\tendif\n' + TAIL
P['d_macro_unused_bad_escape'] = HEAD68 + 'mz\tmacro\n\tdc.b "\\c"\n\tendm\n' + TAIL
P['d_num_range_in_expr'] = HEAD68 + '\tmove.w #"\\256",d0\n' + TAIL
PGC = "\tcharset $41,$11\n"
P['e_charset_src_str'] = HEAD68 + PGC + '\tcharset "\\x41",$99\n\tdc.b "A\\x11"\n' + TAIL
P['e_charset_src_plainstr'] = HEAD68 + PGC + '\tcharset "A",$99\n\tdc.b "A\\x11"\n' + TAIL
P['e_strcmp_page'] = HEAD68 + PGC + '\tdc.b "\\x41"="A"\n' + TAIL
P['e_strlen_page'] = HEAD68 + PGC + '\tdc.b strlen("\\x41\\x41")\n' + TAIL
P['e_octal_page'] = HEAD68 + PGC + '\tdc.b "\\0101"\n' + TAIL

P['f_irpc_str'] = HEAD68 + '\tirpc c,"\\x41B"\n\tdc.b "c"\n\tendr\n' + TAIL
P['f_irpc_bare'] = HEAD68 + '\tirpc c,\\x41B\n\tdc.b "c"\n\tendr\n' + TAIL
P['f_include_path'] = HEAD68 + '\tinclude "p_inc\\x41.inc"\n' + TAIL
open(os.path.join(D, 'p_incA.inc'), 'w').write('\tdc.b $77\n')
P['g_message_unterminated_interp'] = HEAD68 + 'n\tequ 5\n\tmessage "a\\{n"\n' + TAIL
P['g_dcb_unterminated_interp'] = HEAD68 + 'n\tequ 5\n\tdc.b "a\\{n"\n' + TAIL
P['g_if0_bad_char'] = HEAD68 + "\tif 0\n\tdc.w '\\c'\n\tendif\n" + TAIL
P['g_dcb_interp_undefined'] = HEAD68 + '\tdc.b "\\{nope}"\n' + TAIL
P['g_dcb_interp_forward'] = HEAD68 + '\tdc.b "\\{later}"\nlater\tequ 7\n' + TAIL
P['g_set_esc_nested_substr'] = HEAD68 + 's\t:= substr("\\x41\\x42\\x43",1,0)\n\tdc.b s\n' + TAIL
P['g_quote_roundtrip'] = HEAD68 + 'mq\tmacro pa\n\tdc.b pa\n\tdc.b strlen(pa)\n\tendm\n\tmq "a\\\\b\\"c"\n' + TAIL

for f in os.listdir(D):
    if f.endswith('.asm'):
        os.remove(os.path.join(D, f))
for k, v in P.items():
    open(os.path.join(D, 'p_%s.asm' % k), 'w', encoding='latin-1').write(v)
print('probes written:', len(P))
