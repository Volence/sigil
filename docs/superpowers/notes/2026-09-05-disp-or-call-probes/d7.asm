; Does `#f(<register>)` have an answer at all, or only when the body ignores
; its parameter? `pfree` never mentions `p`; `puse` does. Same call shape.
	cpu 68000
pfree	function p,$3C7
puse	function p,(p*7)+$100
	org $1000
	move.w	#pfree(a1),d0
	move.w	#puse(a1),d0
	move.w	#puse(5),d0
