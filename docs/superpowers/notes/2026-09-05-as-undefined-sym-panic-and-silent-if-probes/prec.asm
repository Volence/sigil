	cpu	68000
K	equ	3
J	equ	4
	dc.l	(K*2)=6&&(J<>3)
	dc.l	((K*2)=6)&&((J<>3))
	dc.l	(K*2)=6
	dc.l	6&&(J<>3)
