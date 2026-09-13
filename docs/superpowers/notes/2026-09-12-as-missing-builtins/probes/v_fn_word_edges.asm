	cpu 68000
	padding off
	org 0
fg function n,"a.n n.b n"
	dc.b fg(3),$EE
fh function n,"n_1 1n n1 _n"
	dc.b fh(3),$EE
fi function n,"N-n"
	dc.b fi(3),$EE
fj function n,"(n)"
	dc.b fj(3),$EE
fk function n,m,"m n"
	dc.b fk(1,2),$EE
fl function n,"\{n}n"
	dc.b fl(n2),$EE
n2 equ 7
	end
