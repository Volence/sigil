; ARE FUNCTION ARGUMENTS EVALUATED WHEN THE BODY IGNORES THEM?
;
; AS's manual, section FUNCTION: "When the function is called, all parameters
; are calculated once and are then inserted into the function's formula. This is
; done to reduce calculation overhead and to avoid side effects." AS is STRICT
; in its arguments, and the insertion is TEXTUAL — "the arguments are textually
; inserted into the function's formula", with integer, float and string named as
; the types that have such a form.
;
; An expander that is LAZY instead — one that folds `fi(x)` to the body without
; evaluating `x` because the body never mentions it — never reaches the refusal
; AS reaches, and accepts a program AS rejects. That difference is invisible in
; a corpus where every argument is valid, which is why it needs its own probe.
;
; `fi` ignores its parameter; `fu` uses it. `zz` is never defined.
	cpu 68000
	padding off
fu	function p,(p*7)+$100
fi	function p,$3C7
	org $1000
	move.w	#$A101,d0
	move.w	#fu(zz),d0	; body USES p, argument undefined
	move.w	#$A202,d0
	move.w	#fi(zz),d0	; body IGNORES p, argument undefined
	move.w	#$A303,d0
	move.w	#fi(a1),d0	; body IGNORES p, argument a register
	move.w	#$A404,d0
	move.w	#fi(5),d0	; control: must be $3C7
	move.w	#$A505,d0
