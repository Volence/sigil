	cpu 68000
	padding off
	org 0
palette macro {INTLABEL},path,path2
__LABEL__ label *
	dc.b "path"
    if "path2"<>""
	dc.b "path2"
    endif
__LABEL___End label *
	endm
Pal_SS1_2p:palette Special Stage 1 2p.bin ; Special Stage 1 2p palette
Pal_SS2:   palette Special Stage 2.bin,Sonic and 2p.bin ; two
	dc.l Pal_SS1_2p_End-Pal_SS1_2p
	dc.l Pal_SS2_End-Pal_SS2
	dc.b $EE
	end
