; 2026-07-05-spec2-plan5-sandbox-float-handoff.md names its compatibility target
; as "asl 1.42 Bld 212's numeric routines". If the builds' float routines
; differed, that target would name no single behaviour. A spread of the
; functions a frequency or sine table uses, each scaled so rounding shows.
	cpu 68000
	padding off
	org 0
	dc.l	int(sin(0.5)*1000000000)
	dc.l	int(cos(1.25)*1000000000)
	dc.l	int(sqrt(2.0)*1000000000)
	dc.l	int(ln(10.0)*100000000)
	dc.l	int(exp(3.3)*1000000)
	dc.l	int(2.0^(7.0/12.0)*1000000000)
	dc.l	int(3579545.0/(32.0*440.0)*10000)
	dc.l	int(atan(0.3)*1000000000)
	dc.l	int(log(7.0)*1000000000)
	dc.l	int(-2.5)
	dc.l	int(1.0/3.0*3000000000.0/2.0)
