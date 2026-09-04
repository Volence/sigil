	cpu	68000
	padding off
; --- verbatim from s2.macrosetup.asm (chkop, _btst, _beq, _bne) ---
chkop function op,ref,(substr(lowstring(op),0,strlen(ref))<>ref)

_btst macro x,y
last_btst_converted := ~~chkop("x","#render_flags.on_screen") || ~~chkop("x","#status.npc.no_balancing") || ~~chkop("x","#status_secondary.sliding")

	if last_btst_converted
		tst.b	y
	else
		btst	x,y
	endif
    endm

_beq macro x
	if last_btst_converted
		bpl.ATTRIBUTE	x
	else
		beq.ATTRIBUTE	x
	endif
    endm

_bne macro x
	if last_btst_converted
		bmi.ATTRIBUTE	x
	else
		bne.ATTRIBUTE	x
	endif
    endm

render_flags.on_screen = 7
status.npc.no_balancing = 3
status_secondary.sliding = 1
render_flags = 1
status = 34
status_secondary = 42
	org	$1000
; --- the six operand shapes the corpus actually calls (81 sites) ---
Lt:
	_btst	#render_flags.on_screen,render_flags(a0)
	_bne.s	Lt
	_btst	#status_secondary.sliding,status_secondary(a0)
	_beq.s	Lt
	_btst	#render_flags.on_screen,render_flags(a1)
	_bne.w	Lt
	_btst	#status.npc.no_balancing,status(a0)
	_beq.w	Lt
	_btst	#status.npc.no_balancing,status(a1)
	_bne.s	Lt
	_btst	#status_secondary.sliding,status_secondary(a1)
	_beq.s	Lt
; --- a NON-matching operand: the shape the corpus never writes ---
	_btst	#4,status(a0)
	_bne.s	Lt
	end
