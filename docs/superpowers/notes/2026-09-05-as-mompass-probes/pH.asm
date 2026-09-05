	cpu 68000
	padding off
	supmode on
	org 0
	message "SEEN pass=\{MOMPASS}"
	dc.w MOMPASS
	move.w	d0,Sym
Later:
	dc.w	Later
Sym	equ	$FFFF8000
	end
