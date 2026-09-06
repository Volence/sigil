	cpu	68000
	padding off
	org	0

; ── THE CORPUS ARGUMENTS, EXACTLY AS `hud_counter` WRITES THEM ──────────────
; `s2.asm(87677)` is `.loop_counter = int(log(number))`, "total digits minus
; one", and `number` is one of these six. The answers must be 0..5; anything
; else is a HUD that counts wrong, so these are the load-bearing rows.
	dc.l	INT(LOG(1))		; 0
	dc.l	INT(LOG(10))		; 1
	dc.l	INT(LOG(100))		; 2 base 10 / 4 natural  <- BASE DISCRIMINATOR
	dc.l	INT(LOG(1000))		; 3
	dc.l	INT(LOG(10000))		; 4
	dc.l	INT(LOG(100000))	; 5

; ── LOG IS NOT LN, SHOWN FROM BOTH SIDES ────────────────────────────────────
; LOG(100)=2 already excludes natural log (ln 100 = 4.605, INT -> 4). This row
; shows the build HAS a natural log under a different name, so "LOG is base 10"
; is a real distinction here and not an absence.
	dc.l	INT(LN(100))		; 4  <- ln, for contrast
	dc.l	INT(LN(1000))		; 6

; ── AND LOG IS AN EXACT log10, NOT ln(x)/ln(10) ─────────────────────────────
; This is the row that decides the HUD's bytes. `ln(1000)/ln(10)` in binary64
; is 2.9999999999999996, one ULP short of 3, and INT() FLOORS, so an
; implementation that spells log10 that way answers 2 for `Hud_1000` and the
; counter is off by one. Subtracting the integer and scaling by 1e15 makes the
; residual visible: exactly 0 for a true log10, negative for the ULP-short
; form, positive for an ULP-long one.
	dc.l	INT((LOG(1)-0)*1e15)
	dc.l	INT((LOG(10)-1)*1e15)
	dc.l	INT((LOG(100)-2)*1e15)
	dc.l	INT((LOG(1000)-3)*1e15)
	dc.l	INT((LOG(10000)-4)*1e15)
	dc.l	INT((LOG(100000)-5)*1e15)

; ── INT() ON A NEGATIVE: FLOOR OR TRUNCATE-TOWARD-ZERO ──────────────────────
; No positive value can tell these apart. -3.2 goes to -4 under floor and -3
; under truncation; LOG(0.5) = -0.30103 goes to -1 under floor and 0 under
; truncation, which also exercises the negative through the new function.
	dc.l	INT(-3.2)		; -4 floor / -3 trunc
	dc.l	INT(LOG(0.5))		; -1 floor /  0 trunc
	dc.l	INT(LOG(0.5)*1e6)	; -301029 floor / -301029 (agree; value check)

; ── THE REST OF THE FLOAT SURFACE ───────────────────────────────────────────
; Each argument is picked so a plausible WRONG implementation gives a visibly
; different answer, never a coincidentally equal one.
	dc.l	INT(EXP(2)*1e6)		; e^2=7389056 / 2^2 would be 4000000
	dc.l	INT(SQRT(2)*1e6)	; 1414213
	dc.l	INT(SIN(1)*1e6)		; radians 841470 / degrees 17452
	dc.l	INT(COS(1)*1e6)		; radians 540302 / degrees 999847
	dc.l	INT(TAN(1)*1e6)		; radians 1557407 / degrees 17455
	dc.l	INT(ATAN(1)*1e6)	; radians 785398 / degrees 45000000
	dc.l	INT(ASIN(1)*1e6)	; radians 1570796 / degrees 90000000
	dc.l	INT(ACOS(0)*1e6)	; radians 1570796 / degrees 90000000
	dc.l	INT(SINH(1)*1e6)	; 1175201
	dc.l	INT(COSH(1)*1e6)	; 1543080
	dc.l	INT(TANH(1)*1e6)	; 761594
	dc.l	INT(ASINH(1)*1e6)	; 881373
	dc.l	INT(ACOSH(2)*1e6)	; 1316957
	dc.l	INT(ATANH(0.5)*1e6)	; 549306
	dc.l	INT(ABS(-3.25)*1e6)	; 3250000
	end
