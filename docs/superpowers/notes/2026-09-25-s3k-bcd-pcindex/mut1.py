import sys
f, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(f).read()
if s.count(old) != 1:
    print("pattern count %d in %s" % (s.count(old), f))
    sys.exit(1)
open(f, "w").write(s.replace(old, new))
