	cpu 68000
	padding off
	org 0
sgn equ 7
bitcnt equ 7
firstbit equ 7
bitpos equ 7
toupper equ 7
tolower equ 7
lastbit equ 7
abs equ 7
int equ 7
	dc.b sgn,sgn(8)
	dc.b bitcnt,bitcnt(8)
	dc.b firstbit,firstbit(8)
	dc.b bitpos,bitpos(8)
	dc.b toupper,toupper(8)
	dc.b tolower,tolower(8)
	dc.b lastbit,lastbit(8)
	dc.b abs,abs(8)
	dc.b int,int(8)
	dc.b $EE
	end
