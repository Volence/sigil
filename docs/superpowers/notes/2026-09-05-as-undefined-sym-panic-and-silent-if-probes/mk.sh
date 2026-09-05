#!/bin/bash
D=/tmp/claude-1000/-home-volence-sonic-hacks-sigil/1a93ba92-b503-43b3-8939-b5973f7954ac/scratchpad/p
cd "$D" || exit 9

# fwd_equ: `if` on an equ defined LATER in the same file.
cat > fwd_equ.asm <<'EOF'
	cpu	68000
	if	Later
	dc.l	$11111111
	endif
	dc.l	$22222222
Later	equ	1
EOF

# fwd_label: `if` on a LABEL defined later in the same file.
cat > fwd_label.asm <<'EOF'
	cpu	68000
	if	Later>0
	dc.l	$11111111
	endif
	dc.l	$22222222
Later:
	dc.l	$33333333
EOF

# fwd_include: `if` on a symbol defined in a file included AFTER the if.
cat > fwd_inc_sub.inc <<'EOF'
Later	equ	1
EOF
cat > fwd_include.asm <<'EOF'
	cpu	68000
	if	Later
	dc.l	$11111111
	endif
	dc.l	$22222222
	include	"fwd_inc_sub.inc"
EOF

# back_include: `if` on a symbol defined in a file included BEFORE the if.
cat > back_include.asm <<'EOF'
	cpu	68000
	include	"fwd_inc_sub.inc"
	if	Later
	dc.l	$11111111
	endif
	dc.l	$22222222
EOF

# fwd_set: `if` on a name a later `set` defines.
cat > fwd_set.asm <<'EOF'
	cpu	68000
	if	Later
	dc.l	$11111111
	endif
	dc.l	$22222222
Later	set	1
EOF

# defined_const: the plain shape a refusal must never fire on.
cat > defined_const.asm <<'EOF'
	cpu	68000
K	equ	1
	if	K
	dc.l	$11111111
	endif
	dc.l	$22222222
EOF

# expr_cond: a legitimate compound expression condition.
cat > expr_cond.asm <<'EOF'
	cpu	68000
K	equ	3
J	equ	4
	if	(K*2)=6&&(J<>3)
	dc.l	$11111111
	endif
	dc.l	$22222222
EOF

# elseif_undef: an undefined symbol in an ELSEIF arm that is REACHED.
cat > elseif_undef.asm <<'EOF'
	cpu	68000
	if	0
	dc.l	$11111111
	elseif	Nowhere
	dc.l	$33333333
	endif
	dc.l	$22222222
EOF

# elseif_unreached: an undefined symbol in an ELSEIF arm the taken first arm skips.
cat > elseif_unreached.asm <<'EOF'
	cpu	68000
	if	1
	dc.l	$11111111
	elseif	Nowhere
	dc.l	$33333333
	endif
	dc.l	$22222222
EOF

# if_in_skipped: an undefined `if` nested inside a NOT-taken outer arm.
cat > if_in_skipped.asm <<'EOF'
	cpu	68000
	if	0
	if	Nowhere
	dc.l	$11111111
	endif
	endif
	dc.l	$22222222
EOF

# ifdef_undef: `ifdef` on an undefined name is legitimate, never a refusal.
cat > ifdef_undef.asm <<'EOF'
	cpu	68000
	ifdef	Nowhere
	dc.l	$11111111
	endif
	dc.l	$22222222
EOF

# momcpu_str: the string-comparison condition shape.
cat > momcpu_str.asm <<'EOF'
	cpu	68000
	if	MOMCPUNAME="Z80"
	dc.l	$11111111
	endif
	dc.l	$22222222
EOF

# jsr_fwd: jsr to a label defined later (must stay a working program).
cat > jsr_fwd.asm <<'EOF'
	cpu	68000
	jsr	Later
Later:
	rts
EOF
echo "made"
/bin/ls *.asm
