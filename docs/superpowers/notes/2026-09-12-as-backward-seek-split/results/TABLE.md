| shape | asl: L_next / bytes @ | asl: L_after / bytes @ | sigil: L_next / bytes @ | sigil: L_after / bytes @ | verdict |
|---|---|---|---|---|---|
| c01_cpu_same | $C / $C | $12 / $12 | $C / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c02_phase | $8000 / $C | $12 / $12 | $8000 / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c03_dephase | $C / $C | $12 / $12 | $C / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c04_restore_z80_phased | $A / $A | $10 / $10 | $A / $C | $10 / $12 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 20 bytes |
| c05_org_forward_leaves | $40 / $40 | $46 / $46 | $40 / $40 | $46 / $46 | MATCH (images identical, 72 bytes) |
| c06_org_backward_leaves | $10 / $10 | $16 / $16 | $10 / $10 | $16 / $16 | MATCH (images identical, 40 bytes) |
| c07_seek_back_then_to_extent | $10 / $10 | $16 / $16 | $10 / $10 | $16 / $16 | MATCH (images identical, 24 bytes) |
| c08_two_seeks_short_of_extent | $C / $C | $12 / $12 | $C / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c09_seek_no_bytes_then_cpu | $A / $A | $10 / $10 | $A / $10 | $10 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 24 bytes |
| c10_cpu_z80_unphased | $C / $C | $E / $E | sigil exit 1 (refused) | - | SIGIL REFUSES (asl accepts) |
| c11_cpu_z80_phased | $8000 / $C | $10 / $10 | $8000 / $10 | $10 / $14 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 22 bytes |
| c12_section_endsection | $C / $C | $12 / $12 | sigil exit 1 (refused) | - | SIGIL REFUSES (asl accepts) |
| c13_z80_host_cpu_same | $6 / $6 | $A / $A | $6 / $8 | $A / $C | DISAGREE: labels agree, bytes land elsewhere, image 12 vs 14 bytes |
| c14_restore_out_of_host | $8000 / $C | $10 / $10 | $8000 / $10 | $10 / $14 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 22 bytes |
| c15_restore_same_cpu_control | $C / $C | $12 / $12 | $C / $C | $12 / $12 | MATCH (images identical, 20 bytes) |
CLASSIFY_ROWS=15
