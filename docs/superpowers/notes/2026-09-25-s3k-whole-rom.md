# S3K whole ROM: sigil builds `buildSK.lua`'s image byte-identical

2026-09-25, measurement parcel, branch `measure/s3k-whole-rom`, base `ec860c7e`
(master, read from the tree at start). **No sigil source was changed**, so no suite
run was needed. Evidence, scripts and run records are in `2026-09-25-s3k-whole-rom/`
beside this note. Scratch: `/home/volence/sonic_hacks/.scratch/s3k-whole-rom/`.

(Work in progress; later sections are added as findings land.)

## Finding 1: identical, 0 bytes, with the planted control

sigil `ec860c7e`, wrapper root, `buildSK.lua`'s p2bin instruction passed as written:

```text
sigil wrapper.asm -o image.bin -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
```

exit 0, 0 stderr lines, image md5 `4ea493ea4e9f6c9ebfccbdb15110367e`, CRC32 `0658f691`,
2,097,152 bytes. `compare.py` against the reference: 0 differing bytes, no length
mismatch; control planted at `[0, 1048576, 2097151]`, reported exactly.
