; 2026-09-03-as-shift-macro-argument-walk.md, BLOCKED/ARGCOUNT: asl drops
; ARGCOUNT from 3 to 0 across one shift of a one-parameter macro called with
; three arguments (e[3] -> s[0]). Plus the note's main table, zt 1,2,3,4 on
; params pp,qq,rr, rendered between shifts.
	cpu 68000
	padding off
	org 0
ac	macro	pp
	dc.b	ARGCOUNT
	shift
	dc.b	ARGCOUNT
	endm
	ac	1,2,3
zt	macro	pp,qq,rr
	dc.b	"<pp|qq|rr|ALLARGS>"
	shift
	dc.b	"<pp|qq|rr|ALLARGS>"
	shift
	dc.b	"<pp|qq|rr|ALLARGS>"
	shift
	dc.b	"<pp|qq|rr|ALLARGS>"
	shift
	dc.b	"<pp|qq|rr|ALLARGS>"
	endm
	zt	1,2,3,4
