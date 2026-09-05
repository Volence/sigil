; Which input decides what `#name(<register>)` becomes: whether the function
; body uses its parameter, or whether an EQUATE of the same name also exists?
; Four cells, one discriminator each way. Every value is distinct and non-round
; so no two readings can produce the same word.
;   fa  function only,        body ignores p   -> $3C7 if expanded
;   fb  function only,        body uses p      -> (p*7)+$100 if expanded
;   ea  function AND equate,  body ignores p   -> $3C7 expanded / $2A equate
;   eb  function AND equate,  body uses p      -> (p*7)+$100 expanded / $71 equate
	cpu 68000
fa	function p,$3C7
fb	function p,(p*7)+$100
ea	=	$2A
ea	function p,$3C7
eb	=	$71
eb	function p,(p*7)+$100
	org $1000
	move.w	#fa(a1),d0
	move.w	#fb(a1),d0
	move.w	#ea(a1),d0
	move.w	#eb(a1),d0
	move.w	#fa(5),d0
	move.w	#fb(5),d0
	move.w	#ea(5),d0
	move.w	#eb(5),d0
