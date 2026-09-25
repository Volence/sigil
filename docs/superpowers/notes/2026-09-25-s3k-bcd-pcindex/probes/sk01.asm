	cpu 68000
x_vel = $18
y_vel = $1A
	phase $8280C
		movem.w	word_82872(pc,d0.w),d2-d3
		move.w	d2,x_vel(a0)
		move.w	d3,y_vel(a0)

loc_8281A:
		tst.w	d0
		beq.s	loc_82828
		movem.w	word_82832(pc,d0.w),d4-d5
		add.w	d4,d2
		add.w	d5,d3

loc_82828:
		move.w	d2,x_vel(a1)
		move.w	d3,y_vel(a1)
		rts
; End of function sub_82772

; ---------------------------------------------------------------------------
word_82832:	; neither A, B, nor C are pressed
		dc.w      0,     0	; no D-pad
		dc.w      0, -$300	; up
		dc.w      0,  $300	; down
		dc.w      0,     0	; up + down (shouldn't happen)
		dc.w  -$300,     0	; left
		dc.w  -$21F, -$21F	; left + up
		dc.w  -$21F,  $21F	; left + down
		dc.w      0,     0	; left + up + down (shouldn't happen)
		dc.w   $300,     0	; right
		dc.w   $21F, -$21F	; right + up
		dc.w   $21F,  $21F	; right + down
		dc.w      0,     0	; right + up + down (shouldn't happen)
		dc.w      0,     0	; left + right(shouldn't happen)
		dc.w      0,     0	; left + right + up (shouldn't happen)
		dc.w      0,     0	; left + right + down (shouldn't happen)
		dc.w      0,     0	; left + right + up + down (shouldn't happen)
word_82872:	; either A, B, or C is pressed
		dc.w   $600,     0	; no D-pad
		dc.w      0, -$600	; up
		dc.w      0,  $600	; down
		dc.w      0,     0	; up + down (shouldn't happen)
		dc.w  -$600,     0	; left
		dc.w  -$43E, -$43E	; left + up
		dc.w  -$43E,  $43E	; left + down
		dc.w      0,     0	; left + up + down (shouldn't happen)
		dc.w   $600,     0	; right
		dc.w   $43E, -$43E	; right + up
		dc.w   $43E,  $43E	; right + down
		dc.w      0,     0	; right + up + down (shouldn't happen)
		dc.w      0,     0	; left + right (shouldn't happen)
		dc.w      0,     0	; left + right + up (shouldn't happen)
		dc.w      0,     0	; left + right + down (shouldn't happen)
		dc.w      0,     0	; left + right + up + down (shouldn't happen)
