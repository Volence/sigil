	cpu 68000
	org $1200
A1:	nop
$$x:	nop
+	dc.w $$x
-	nop
	dc.w $$x
