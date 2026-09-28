#!/usr/bin/env bash
# Writes one probe per form, each alone in a file under `cpu 68000`, into the
# directory this script lives in. Re-running rewrites them identically.
# Every probe is a SYMBOL (or a symbolic expression) followed by an
# absolute-size suffix, the form AS-SYMBOL-SIZE-SUFFIX covers.
set -eu
cd "$(dirname "$0")"
mk() { printf '\tcpu 68000\n%b\n' "$2" > "$1.asm"; }

# The five instruction shapes, symbol defined before use.
mk jmp_sym_w 'Foo = $1234\n\tjmp\tFoo.w'
mk jmp_sym_W 'Foo = $1234\n\tjmp\tFoo.W'
mk jsr_sym_l 'Foo = $1234\n\tjsr\tFoo.l'
mk jsr_sym_L 'Foo = $1234\n\tjsr\tFoo.L'
mk lea_sym_w 'Foo = $1234\n\tlea\tFoo.w,a0'
mk move_sym_w_src 'Foo = $1234\n\tmove.w\tFoo.w,d0'
mk move_sym_l_dst 'Foo = $1234\n\tmove.l\td0,Foo.l'
mk move_sym_w_dst 'Foo = $FFFFF600\n\tmove.w\td0,Foo.w'
mk move_sym_both 'Foo = $1234\nBar = $FFFF8000\n\tmove.w\tFoo.w,Bar.w'
mk jmp_sym_w_neg 'Foo = $FFFF8000\n\tjmp\tFoo.w'
mk jmp_sym_w_m2 'Foo = -2\n\tjmp\tFoo.w'
mk jmp_sym_expr_w 'Foo = $1000\n\tjmp\tFoo+$234.w'
mk jmp_sym_paren_w 'Foo = $1234\n\tjmp\t(Foo).w'

# Forward references: an equate and a label defined after use.
mk fwd_equ_w '\tjmp\tFoo.w\nFoo = $1234'
mk fwd_equ_l '\tjsr\tFoo.l\nFoo = $1234'
mk fwd_label_w '\tjmp\tT.w\n\tnop\nT:'
mk fwd_label_l '\tjmp\tT.l\n\tnop\nT:'
mk back_label_w 'T:\tnop\n\tjmp\tT.w'
mk fwd_equ_move '\tmove.w\tFoo.w,d0\nFoo = $FFFFF600'

# Out of range for a signed 16-bit short address.
mk jmp_sym_w_hi 'Foo = $8000\n\tjmp\tFoo.w'
mk jmp_sym_w_big 'Foo = $12345\n\tjmp\tFoo.w'
mk move_sym_w_hi 'Foo = $FF00\n\tmove.w\td0,Foo.w'
mk fwd_equ_w_hi '\tjmp\tFoo.w\nFoo = $8000'
mk fwd_label_w_hi '\tjmp\tT.w\n\tds.b\t$8000\nT:'

# Suffixes asl does not take as an absolute size.
mk jmp_sym_b 'Foo = $1234\n\tjmp\tFoo.b'
mk jmp_sym_s 'Foo = $1234\n\tjmp\tFoo.s'
mk undef_sym_w '\tjmp\tUndef.w'
mk dc_sym_w 'Foo = $1234\n\tdc.w\tFoo.w'
mk imm_sym_w 'Foo = $1234\n\tmove.w\t#Foo.w,d0'

# Names spelled with a dot, and composed temporary (`.local`) labels.
mk dot_equ_def 'Foo.bar = $1234\n\tdc.w\tFoo.bar'
mk dot_equ_w_def 'Foo.w = $5678\n\tdc.w\t0'
mk dot_equ_w_use 'Foo.w = $5678\n\tjmp\tFoo.w'
mk dot_equ_collide 'Foo = $1234\nFoo.w = $5678\n\tjmp\tFoo.w'
mk local_w_collide 'Foo:\tnop\n.w:\tnop\n\tjmp\tFoo.w'
mk local_l_collide 'Foo:\tnop\n.l:\tnop\n\tjsr\tFoo.l'
mk local_w_bare 'Foo:\tnop\n.w:\tnop\n\tjmp\t.w'
mk local_loop_w 'Foo:\tnop\n.loop:\tnop\n\tjmp\t.loop.w'
mk local_full_w 'Foo:\tnop\n.loop:\tnop\n\tjmp\tFoo.loop.w'
mk local_full_l 'Foo:\tnop\n.loop:\tnop\n\tjmp\tFoo.loop.l'
mk local_full_dc 'Foo:\tnop\n.loop:\tnop\n\tdc.w\tFoo.loop'

# Where a dotted name is read whole: data, immediates, displacements.
mk dc_dot_w_defined 'Foo.w = $5678\n\tdc.w\tFoo.w'
mk imm_dot_w_defined 'Foo.w = $5678\n\tmove.w\t#Foo.w,d0'
mk disp_dot_w_defined 'Foo.w = 4\n\tmove.w\tFoo.w(a0),d0'
mk disp_sym_w 'Foo = 4\n\tmove.w\tFoo.w(a0),d0'
mk paren_dot_w_defined 'Foo = $1234\nFoo.w = $5678\n\tjmp\t(Foo.w).w'

# Displacement suffixes, spaces, and where in the operand the suffix is read.
mk disp_num_w '\tmove.w\t4.w(a0),d0'
mk disp_sym_l 'Foo = 4\n\tmove.w\tFoo.l(a0),d0'
mk disp_sym_b 'Foo = 4\n\tmove.w\tFoo.b(a0),d0'
mk disp_sym_s 'Foo = 4\n\tmove.w\tFoo.s(a0),d0'
mk disp_sym_W 'Foo = 4\n\tmove.w\tFoo.W(a0),d0'
mk disp_sym_w_big 'Foo = $8000\n\tmove.w\tFoo.w(a0),d0'
mk disp_collide 'Foo = 4\nFoo.w = 6\n\tmove.w\tFoo.w(a0),d0'
mk idx_sym_w 'Foo = 4\n\tmove.w\tFoo.w(a0,d0.w),d1'
mk idx_sym_b 'Foo = 4\n\tmove.w\tFoo.b(a0,d0.w),d1'
mk pcrel_sym_w 'Foo:\tnop\n\tmove.w\tFoo.w(pc),d0'
mk paren_disp_dot 'Foo.w = 4\n\tmove.w\t(Foo.w,a0),d0'
mk paren_disp_sym 'Foo = 4\n\tmove.w\t(Foo.w,a0),d0'
mk paren_abs_dot 'Foo = $1234\nFoo.w = $5678\n\tjmp\t(Foo.w)'
mk expr_mid_dot 'Foo.w = $10\n\tjmp\tFoo.w+2'
mk expr_tail_sym 'Foo = $10\n\tjmp\t2+Foo.w'
mk dot_w_w 'Foo.w = $5678\n\tjmp\tFoo.w.w'
mk dot_b_defined 'Foo.b = $1234\n\tjmp\tFoo.b'
mk sym_space_w 'Foo = $1234\n\tjmp\tFoo .w'
mk num_space_w '\tjmp\t$1234 .w'
mk dreg_w '\tmove.w\td0.w,d1'
mk areg_l '\tmove.l\ta0.l,d1'
mk sym_areg_name_w 'A9 = $1234\n\tjmp\tA9.w'
mk sym_imm_expr_w 'Foo = $1234\n\tjmp\t-(-Foo).w'

# The parenthesised forms, with the suffix inside.
mk paren_num_w '\tjmp\t($1234.w)'
mk paren_sym_w 'Foo = $1234\n\tjmp\t(Foo.w)'
mk paren_sym_l 'Foo = $1234\n\tjmp\t(Foo.l)'
mk paren_sym_b 'Foo = $1234\n\tjmp\t(Foo.b)'
mk paren_sym_w_hi 'Foo = $8000\n\tjmp\t(Foo.w)'
mk paren_move_sym_w 'Foo = $FFFFF600\n\tmove.w\t(Foo.w),d0'
mk paren_disp_num '\tmove.w\t(4.w,a0),d0'
mk paren_disp_sym_l 'Foo = 4\n\tmove.w\t(Foo.l,a0),d0'
mk paren_disp_sym_b 'Foo = 4\n\tmove.w\t(Foo.b,a0),d0'
mk paren_idx_sym_b 'Foo = 4\n\tmove.w\t(Foo.b,a0,d0.w),d1'
mk paren_idx_sym_w 'Foo = 4\n\tmove.w\t(Foo.w,a0,d0.w),d1'
mk paren_disp_collide 'Foo = 4\nFoo.w = 6\n\tmove.w\t(Foo.w,a0),d0'
mk idx_num_b '\tmove.w\t4.b(a0,d0.w),d1'
mk idx_collide 'Foo = 4\nFoo.b = 6\n\tmove.w\tFoo.b(a0,d0.w),d1'
mk pcrel_num_w '\tnop\n\tmove.w\t0.w(pc),d0'
mk pcidx_sym_b 'Foo:\tnop\n\tmove.w\tFoo.b(pc,d0.w),d1'

# Branch and dbcc targets, which are not absolute addresses.
mk bra_local_w 'T:\tnop\n.w:\tnop\n\tbra.s\tT.w'
mk bra_sym_w '\tbra.w\tT.w\n\tnop\nT:'
mk bsr_local_l 'T:\tnop\n.l:\tnop\n\tbsr.w\tT.l'
mk dbf_local_w 'T:\tnop\n.w:\tnop\n\tdbf\td0,T.w'
mk dbf_sym_w '\tdbf\td0,T.w\n\tnop\nT:'
mk bra_dot_loop_w 'T:\tnop\n.loop:\tnop\n\tbra.s\t.loop.w'

# Other effective-address users.
mk movem_sym_w 'Foo = $FFFFF600\n\tmovem.l\tFoo.w,d0-d1'
mk movem_sym_w_dst 'Foo = $FFFFF600\n\tmovem.l\td0-d1,Foo.w'
mk clr_sym_w 'Foo = $FFFFF600\n\tclr.w\tFoo.w'
mk btst_sym_w 'Foo = $FFFFF600\n\tbtst\t#1,Foo.w'
mk move_imm_sym_w 'Foo = $FFFFF600\n\tmove.w\t#1,Foo.w'
mk pea_sym_l 'Foo = $1234\n\tpea\tFoo.l'

# A macro parameter spelled before the suffix.
mk macro_param_w 'jw macro dst\n\tjmp\tdst.w\n\tendm\nFoo = $1234\n\tjw\tFoo'
