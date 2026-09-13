	cpu 68000
	padding off
	org 0
fa function n,"n"
	dc.b fa(3),$EE
fb function n,"\{n}"
	dc.b fb(3),$EE
fc function n,"n is \{n}"
	dc.b fc(3),$EE
fd function n,"none\{n}"
	dc.b fd(3),$EE
fe function num,"\{num+1}"
	dc.b fe(3),$EE
ff function n,substr("abcdef",n,1)
	dc.b ff(2),$EE
	end
