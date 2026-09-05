	cpu	68000
	org	$1000
Foo:
	jmp	(Foo).l
	jmp	(strlen("abcdefghijkl")+Foo).l
	jmp	(val("Foo")).l
	move.l	#strlen("abcdefghijkl"),d0
	move.l	#val("$4142"),d0
	move.l	#val("4142"),d0
	move.l	#int(3.7),d0
	move.l	strlen("abcdefghijkl")+Foo,d0
	end
