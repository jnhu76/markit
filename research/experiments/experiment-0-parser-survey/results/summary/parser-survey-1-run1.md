# parser-survey run summary

scenarios measured: 395 (skipped: 13)

oracle (incremental == clean rebuild): all scenarios pass

## T1 — taxonomy at synth-1m (mid position)

| case | pred | family | chg_B | inc_us | full_us | R | blocks_re | conv_Δ | surv_shift | rec_moved | proj_blk | D_content | oracle |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| blank_delete | I2 | BlockBoundary | 1 | 87.5 | 6472.8 | 306 | 1 | 4 | 6809 | 6809 | 1 | 306 | ok |
| blank_insert | I2 | BlockBoundary | 1 | 66.1 | 5865.4 | 155 | 2 | 4 | 6810 | 0 | 1 | 2.0 | ok |
| codespan_delim_insert | I1 | DelimiterEdit | 1 | 306.2 | 6097.6 | 73.0 | 2 | 2 | 13656 | 0 | 1 | 72.0 | ok |
| emphasis_close_completion | I1 | DelimiterEdit | 2 | 459.1 | 6447.2 | 30.5 | 2 | 2 | 13654 | 0 | 1 | 30.0 | ok |
| emphasis_delim_insert | I1 | DelimiterEdit | 1 | 148.6 | 5871.4 | 73.0 | 2 | 2 | 13656 | 0 | 1 | 72.0 | ok |
| fence_body_char | I0 | ContentEdit | 1 | 213.2 | 5903.5 | 35.0 | 1 | 3 | 13643 | 0 | 1 | 35.0 | ok |
| fence_closer_delete | I4 | StatePropagating | 4 | 1771.6 | 1231.3 | 262063 | 1 | 20467 | 0 | 0 | 1 | 262063 | ok |
| fence_info_edit | I1 | StatePropagating | 13 | 195.2 | 6409.8 | 2.6 | 2 | 4 | 13643 | 0 | 1 | 2.5 | ok |
| fence_len_grow | I4 | StatePropagating | 1 | 1948.7 | 1196.9 | 1048257 | 3 | 20469 | 0 | 0 | 2 | 1048256 | ok |
| fence_opener_break | I4 | StatePropagating | 2 | 1742.4 | 1281.1 | 524128 | 3 | 20469 | 0 | 0 | 2 | 524128 | ok |
| fence_opener_delete | I4 | StatePropagating | 11 | 1935.2 | 1218.3 | 95295 | 3 | 20468 | 0 | 0 | 2 | 95295 | ok |
| heading_char | I0 | ContentEdit | 1 | 164.2 | 6321.7 | 24.0 | 1 | 1 | 13658 | 0 | 1 | 24.0 | ok |
| heading_from_para | I2 | BlockBoundary | 3 | 88.3 | 5473.6 | 52.0 | 2 | 2 | 6809 | 6809 | 2 | 52.0 | ok |
| large_delete_mid | - | Stress | 209748 | 154.7 | 5075.7 | 0.0 | 1 | 2 | 5447 | 5447 | 1 | 0.0 | ok |
| large_paste_1k | - | Stress | 52890 | 191.6 | 5903.1 | 1.0 | 1 | 1002 | 6809 | 0 | 1 | 1.0 | ok |
| list_indent_add | I3 | Container | 2 | 205.4 | 5627.6 | 47.0 | 1 | 5 | 13650 | 0 | 1 | 47.0 | ok |
| list_marker_add | I3 | Container | 2 | 138.2 | 5721.9 | 47.0 | 1 | 5 | 13650 | 0 | 1 | 47.0 | ok |
| list_marker_remove | I3 | Container | 2 | 177.3 | 6563.2 | 45.0 | 1 | 5 | 13650 | 0 | 1 | 45.0 | ok |
| mermaid_body_char | I6 | ContentEdit | 1 | 178.9 | 5806.3 | 35.0 | 1 | 3 | 13643 | 0 | 1 | 35.0 | ok |
| para_delete_char | I0 | ContentEdit | 1 | 62.9 | 6328.2 | 152 | 1 | 2 | 6809 | 0 | 1 | 152 | ok |
| para_insert_char | I0 | ContentEdit | 1 | 69.6 | 6412.9 | 154 | 1 | 2 | 6809 | 0 | 1 | 154 | ok |
| para_split | I2 | BlockBoundary | 3 | 161.2 | 6284.8 | 51.3 | 3 | 4 | 6809 | 6809 | 3 | 51.3 | ok |
| para_sub_char | I0 | ContentEdit | 2 | 4.3 | 6606.0 | 76.5 | 1 | 2 | 0 | 0 | 1 | 76.5 | ok |
| quote_add | I3 | Container | 2 | 234.7 | 5582.8 | 31.5 | 1 | 3 | 13647 | 13647 | 1 | 31.5 | ok |
| quote_char | I3 | Container | 1 | 231.4 | 5514.9 | 38.0 | 1 | 2 | 13648 | 0 | 1 | 38.0 | ok |
| quote_remove | I3 | Container | 2 | 232.8 | 6916.3 | 29.5 | 2 | 3 | 13647 | 0 | 2 | 29.5 | ok |
| ref_def_add | I5 | SemanticGlobal | 11 | 4.6 | 5587.7 | 1.1 | 3 | 4 | 0 | 0 | 3 | 1.1 | ok |
| ref_def_edit | I5 | SemanticGlobal | 1 | 218.3 | 5908.3 | 30.0 | 2 | 2 | 13652 | 0 | 1 | 29.0 | ok |
| ref_user_edit | I5 | SemanticGlobal | 9 | 193.7 | 5574.5 | 8.1 | 2 | 2 | 13656 | 0 | 1 | 8.0 | ok |
| setext_add | I2 | BlockBoundary | 4 | 61.7 | 5962.2 | 39.2 | 1 | 3 | 6809 | 0 | 1 | 39.2 | ok |

## T2 — hidden-O(N) gate (offset rewrites vs parse radius)

| case | corpus | pos | R | survivor_shifted | records_moved | proj_bytes | inc_us |
|---|---|---|---:|---:|---:|---:|---:|
| bof_insert_char | synth-100k | bof | 24.0 | 1362 | 0 | 24 | 12.2 |
| eof_append_char | synth-100k | eof | 2.0 | 0 | 0 | 1 | 1.2 |
| para_delete_char | synth-100k | bof | 71.0 | 1360 | 0 | 70 | 13.7 |
| para_delete_char | synth-100k | eof | 150 | 1 | 0 | 150 | 1.5 |
| para_delete_char | synth-100k | mid | 151 | 673 | 0 | 150 | 6.5 |
| para_delete_char | synth-100k | q1 | 150 | 1011 | 0 | 150 | 9.3 |
| para_delete_char | synth-100k | q3 | 150 | 337 | 0 | 150 | 3.8 |
| para_insert_char | synth-100k | bof | 73.0 | 1360 | 0 | 72 | 14.0 |
| para_insert_char | synth-100k | eof | 152 | 1 | 0 | 152 | 1.7 |
| para_insert_char | synth-100k | mid | 153 | 673 | 0 | 152 | 6.9 |
| para_insert_char | synth-100k | q1 | 152 | 1011 | 0 | 152 | 9.1 |
| para_insert_char | synth-100k | q3 | 152 | 337 | 0 | 152 | 4.4 |
| bof_insert_char | synth-10k | bof | 24.0 | 148 | 0 | 24 | 1.8 |
| eof_append_char | synth-10k | eof | 2.0 | 0 | 0 | 1 | 0.8 |
| para_delete_char | synth-10k | bof | 71.0 | 146 | 0 | 70 | 3.1 |
| para_delete_char | synth-10k | eof | 148 | 1 | 0 | 148 | 0.9 |
| para_delete_char | synth-10k | mid | 148 | 69 | 0 | 148 | 1.4 |
| para_delete_char | synth-10k | q1 | 148 | 103 | 0 | 148 | 1.7 |
| para_delete_char | synth-10k | q3 | 149 | 33 | 0 | 148 | 1.3 |
| para_insert_char | synth-10k | bof | 73.0 | 146 | 0 | 72 | 2.9 |
| para_insert_char | synth-10k | eof | 150 | 1 | 0 | 150 | 1.0 |
| para_insert_char | synth-10k | mid | 150 | 69 | 0 | 150 | 1.5 |
| para_insert_char | synth-10k | q1 | 150 | 103 | 0 | 150 | 1.7 |
| para_insert_char | synth-10k | q3 | 151 | 33 | 0 | 150 | 1.4 |
| bof_insert_char | synth-1k | bof | 24.0 | 26 | 0 | 24 | 0.9 |
| eof_append_char | synth-1k | eof | 2.0 | 0 | 0 | 1 | 0.6 |
| para_delete_char | synth-1k | bof | 71.0 | 24 | 0 | 70 | 1.7 |
| para_delete_char | synth-1k | eof | 146 | 1 | 0 | 146 | 0.8 |
| para_delete_char | synth-1k | mid | 147 | 7 | 0 | 146 | 1.0 |
| para_delete_char | synth-1k | q1 | 60.0 | 15 | 0 | 23 | 1.1 |
| para_delete_char | synth-1k | q3 | 147 | 3 | 0 | 146 | 0.9 |
| para_insert_char | synth-1k | bof | 73.0 | 24 | 0 | 72 | 1.7 |
| para_insert_char | synth-1k | eof | 148 | 1 | 0 | 148 | 0.8 |
| para_insert_char | synth-1k | mid | 149 | 7 | 0 | 148 | 1.0 |
| para_insert_char | synth-1k | q1 | 62.0 | 15 | 0 | 25 | 1.1 |
| para_insert_char | synth-1k | q3 | 149 | 3 | 0 | 148 | 0.9 |
| bof_insert_char | synth-1m | bof | 24.0 | 13658 | 0 | 24 | 152.2 |
| eof_append_char | synth-1m | eof | 2.0 | 0 | 0 | 1 | 3.4 |
| para_delete_char | synth-1m | bof | 71.0 | 13656 | 0 | 70 | 180.5 |
| para_delete_char | synth-1m | eof | 152 | 1 | 0 | 152 | 3.4 |
| para_delete_char | synth-1m | mid | 152 | 6809 | 0 | 152 | 62.9 |
| para_delete_char | synth-1m | q1 | 153 | 10213 | 0 | 152 | 119.8 |
| para_delete_char | synth-1m | q3 | 152 | 3405 | 0 | 152 | 43.7 |
| para_insert_char | synth-1m | bof | 73.0 | 13656 | 0 | 72 | 218.9 |
| para_insert_char | synth-1m | eof | 154 | 1 | 0 | 154 | 4.2 |
| para_insert_char | synth-1m | mid | 154 | 6809 | 0 | 154 | 69.6 |
| para_insert_char | synth-1m | q1 | 155 | 10213 | 0 | 154 | 252.0 |
| para_insert_char | synth-1m | q3 | 154 | 3405 | 0 | 154 | 31.4 |

## T3 — adversarial & encoding-variant corpora

| corpus | case | pred | chg_B | R | blocks_re | conv_Δ | surv_shift | rec_moved | inc_us | D_content | oracle |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| adv-crlf-mixed | blank_delete | I2 | 2 | 22.0 | 1 | 2 | 3 | 3 | 0.8 | 22.0 | ok |
| adv-crlf-mixed | para_insert_char | I0 | 1 | 26.0 | 2 | 2 | 3 | 0 | 0.7 | 24.0 | ok |
| adv-deep-quote | quote_char | I3 | 1 | 42093 | 1 | 200 | 2 | 0 | 131.2 | 42093 | ok |
| adv-deep-quote | quote_remove | I3 | 2 | 21045 | 1 | 200 | 2 | 0 | 128.1 | 21045 | ok |
| adv-duplicate-refs | ref_def_edit | I5 | 1 | 13.0 | 1 | 1 | 7 | 0 | 0.6 | 13.0 | ok |
| adv-fence-near-bof | fence_body_char | I0 | 1 | 37.0 | 1 | 4 | 3 | 0 | 0.4 | 37.0 | ok |
| adv-fence-near-bof | fence_closer_delete | I4 | 4 | 12.2 | 1 | 6 | 0 | 0 | 0.5 | 12.2 | ok |
| adv-half-written | emphasis_close_completion | I1 | 2 | 11.5 | 2 | 2 | 7 | 0 | 0.9 | 11.0 | ok |
| adv-huge-paragraph | para_insert_char | I0 | 1 | 100003 | 2 | 2 | 3 | 0 | 215.2 | 100002 | ok |
| adv-lazy-continuation | para_insert_char | I0 | 1 | 44.0 | 2 | 2 | 3 | 0 | 0.8 | 26.0 | ok |
| adv-many-refs | ref_def_edit | I5 | 1 | 16788 | 2 | 1001 | 1 | 0 | 55.6 | 16787 | ok |
| adv-unclosed-fence | fence_body_char | I0 | 1 | 571 | 1 | 54 | 0 | 0 | 1.4 | 571 | ok |
| synth-cjk | blank_delete | I2 | 1 | 0.0 | 0 | 0 | 135 | 135 | 1.5 | 0.0 | ok |
| synth-cjk | blank_delete | I2 | 1 | 328 | 1 | 4 | 93 | 93 | 2.2 | 328 | ok |
| synth-cjk | blank_delete | I2 | 1 | 328 | 1 | 4 | 61 | 61 | 2.0 | 328 | ok |
| synth-cjk | blank_delete | I2 | 1 | 328 | 1 | 4 | 31 | 31 | 1.7 | 328 | ok |
| synth-cjk | blank_insert | I2 | 1 | 2.0 | 1 | 2 | 135 | 0 | 1.6 | 2.0 | ok |
| synth-cjk | blank_insert | I2 | 1 | 166 | 2 | 4 | 94 | 0 | 1.9 | 2.0 | ok |
| synth-cjk | blank_insert | I2 | 1 | 166 | 2 | 4 | 62 | 0 | 1.7 | 2.0 | ok |
| synth-cjk | blank_insert | I2 | 1 | 166 | 2 | 4 | 32 | 0 | 1.5 | 2.0 | ok |
| synth-cjk | bof_delete_char | - | 1 | 20.0 | 1 | 1 | 136 | 0 | 1.7 | 20.0 | ok |
| synth-cjk | bof_insert_char | - | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.7 | 22.0 | ok |
| synth-cjk | bof_insert_newline | - | 1 | 22.0 | 2 | 2 | 136 | 136 | 2.2 | 1.0 | ok |
| synth-cjk | codespan_delim_insert | I1 | 1 | 76.0 | 2 | 2 | 134 | 0 | 2.9 | 75.0 | ok |
| synth-cjk | emphasis_close_completion | I1 | 2 | 31.5 | 2 | 2 | 132 | 0 | 2.6 | 31.0 | ok |
| synth-cjk | emphasis_delim_insert | I1 | 1 | 76.0 | 2 | 2 | 134 | 0 | 2.8 | 75.0 | ok |
| synth-cjk | eof_append_char | - | 1 | 2.0 | 2 | 2 | 0 | 0 | 0.8 | 1.0 | ok |
| synth-cjk | fence_body_char | I0 | 1 | 39.0 | 1 | 3 | 123 | 0 | 1.4 | 39.0 | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.4 | 35.0 | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.4 | 35.0 | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.4 | 35.0 | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.4 | 35.0 | ok |
| synth-cjk | fence_closer_delete | I4 | 4 | 2476 | 1 | 184 | 0 | 0 | 13.7 | 2476 | ok |
| synth-cjk | fence_info_edit | I1 | 13 | 2.6 | 2 | 4 | 121 | 0 | 1.4 | 2.5 | ok |
| synth-cjk | fence_len_grow | I4 | 1 | 9910 | 3 | 186 | 0 | 0 | 14.6 | 9909 | ok |
| synth-cjk | fence_opener_break | I4 | 2 | 4954 | 3 | 186 | 0 | 0 | 15.0 | 4954 | ok |
| synth-cjk | fence_opener_delete | I4 | 11 | 900 | 3 | 185 | 0 | 0 | 15.2 | 900 | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.7 | 22.0 | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.8 | 22.0 | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.7 | 22.0 | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.8 | 22.0 | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 2.1 | 22.0 | ok |
| synth-cjk | heading_from_para | I2 | 3 | 56.0 | 3 | 3 | 61 | 61 | 1.9 | 55.7 | ok |
| synth-cjk | large_delete_mid | - | 2059 | 0.1 | 1 | 3 | 49 | 49 | 1.7 | 0.1 | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 134.0 | 1.0 | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 132.3 | 1.0 | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 131.3 | 1.0 | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 131.9 | 1.0 | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 129.9 | 1.0 | ok |
| synth-cjk | list_indent_add | I3 | 2 | 46.0 | 1 | 5 | 128 | 0 | 2.5 | 46.0 | ok |
| synth-cjk | list_marker_add | I3 | 2 | 46.0 | 1 | 5 | 128 | 0 | 2.7 | 46.0 | ok |
| synth-cjk | list_marker_remove | I3 | 2 | 44.0 | 1 | 5 | 128 | 0 | 2.2 | 44.0 | ok |
| synth-cjk | mermaid_body_char | I6 | 1 | 35.0 | 1 | 3 | 121 | 0 | 3.0 | 35.0 | ok |
| synth-cjk | para_delete_char | I0 | 1 | 74.0 | 2 | 2 | 134 | 0 | 4.5 | 73.0 | ok |
| synth-cjk | para_delete_char | I0 | 1 | 164 | 2 | 3 | 93 | 0 | 3.2 | 163 | ok |
| synth-cjk | para_delete_char | I0 | 1 | 164 | 2 | 3 | 61 | 0 | 2.7 | 163 | ok |
| synth-cjk | para_delete_char | I0 | 1 | 163 | 1 | 2 | 31 | 0 | 1.3 | 163 | ok |
| synth-cjk | para_delete_char | I0 | 1 | 163 | 1 | 2 | 1 | 0 | 1.0 | 163 | ok |
| synth-cjk | para_insert_char | I0 | 1 | 76.0 | 2 | 2 | 134 | 0 | 3.0 | 75.0 | ok |
| synth-cjk | para_insert_char | I0 | 1 | 166 | 2 | 3 | 93 | 0 | 1.9 | 165 | ok |
| synth-cjk | para_insert_char | I0 | 1 | 166 | 2 | 3 | 61 | 0 | 1.8 | 165 | ok |
| synth-cjk | para_insert_char | I0 | 1 | 165 | 1 | 2 | 31 | 0 | 1.4 | 165 | ok |
| synth-cjk | para_insert_char | I0 | 1 | 165 | 1 | 2 | 1 | 0 | 1.1 | 165 | ok |
| synth-cjk | para_split | I2 | 2 | 83.5 | 4 | 5 | 61 | 61 | 2.0 | 83.0 | ok |
| synth-cjk | para_sub_char | I0 | 2 | 37.5 | 2 | 2 | 0 | 0 | 1.9 | 37.0 | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.5 | 2 | 3 | 0 | 0 | 1.1 | 82.0 | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.5 | 2 | 3 | 0 | 0 | 1.1 | 82.0 | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.0 | 1 | 2 | 0 | 0 | 1.0 | 82.0 | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.0 | 1 | 2 | 0 | 0 | 1.0 | 82.0 | ok |
| synth-cjk | quote_add | I3 | 2 | 30.0 | 1 | 3 | 125 | 125 | 2.2 | 30.0 | ok |
| synth-cjk | quote_char | I3 | 1 | 37.0 | 1 | 2 | 126 | 0 | 1.8 | 37.0 | ok |
| synth-cjk | quote_remove | I3 | 2 | 28.0 | 2 | 3 | 125 | 0 | 2.0 | 28.0 | ok |
| synth-cjk | ref_def_add | I5 | 11 | 1.1 | 3 | 4 | 0 | 0 | 0.8 | 1.1 | ok |
| synth-cjk | ref_def_edit | I5 | 1 | 30.0 | 2 | 2 | 130 | 0 | 1.9 | 29.0 | ok |
| synth-cjk | setext_add | I2 | 4 | 42.2 | 2 | 4 | 61 | 0 | 1.7 | 42.0 | ok |
| synth-crlf | blank_delete | I2 | 2 | 0.0 | 0 | 0 | 147 | 147 | 1.6 | 0.0 | ok |
| synth-crlf | blank_delete | I2 | 2 | 151 | 1 | 4 | 103 | 103 | 2.1 | 151 | ok |
| synth-crlf | blank_delete | I2 | 2 | 151 | 1 | 4 | 67 | 67 | 1.8 | 151 | ok |
| synth-crlf | blank_delete | I2 | 2 | 151 | 1 | 4 | 33 | 33 | 1.6 | 151 | ok |
| synth-crlf | blank_insert | I2 | 1 | 3.0 | 1 | 2 | 147 | 0 | 1.6 | 3.0 | ok |
| synth-crlf | blank_insert | I2 | 1 | 154 | 2 | 4 | 104 | 0 | 1.8 | 3.0 | ok |
| synth-crlf | blank_insert | I2 | 1 | 154 | 2 | 4 | 68 | 0 | 1.5 | 3.0 | ok |
| synth-crlf | blank_insert | I2 | 1 | 154 | 2 | 4 | 34 | 0 | 1.3 | 3.0 | ok |
| synth-crlf | bof_delete_char | - | 1 | 23.0 | 1 | 1 | 148 | 0 | 1.8 | 23.0 | ok |
| synth-crlf | bof_insert_char | - | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.8 | 25.0 | ok |
| synth-crlf | bof_insert_newline | - | 1 | 25.0 | 2 | 2 | 148 | 148 | 2.1 | 1.0 | ok |
| synth-crlf | codespan_delim_insert | I1 | 1 | 75.0 | 2 | 2 | 146 | 0 | 2.8 | 73.0 | ok |
| synth-crlf | emphasis_close_completion | I1 | 2 | 31.5 | 2 | 2 | 144 | 0 | 2.5 | 30.5 | ok |
| synth-crlf | emphasis_delim_insert | I1 | 1 | 75.0 | 2 | 2 | 146 | 0 | 2.7 | 73.0 | ok |
| synth-crlf | eof_append_char | - | 1 | 3.0 | 2 | 2 | 0 | 0 | 0.6 | 1.0 | ok |
| synth-crlf | fence_body_char | I0 | 1 | 42.0 | 1 | 3 | 135 | 0 | 1.4 | 42.0 | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok |
| synth-crlf | fence_closer_delete | I4 | 5 | 2023 | 1 | 202 | 0 | 0 | 16.1 | 2023 | ok |
| synth-crlf | fence_info_edit | I1 | 13 | 2.9 | 2 | 4 | 133 | 0 | 1.4 | 2.8 | ok |
| synth-crlf | fence_len_grow | I4 | 1 | 10122 | 3 | 204 | 0 | 0 | 14.6 | 10120 | ok |
| synth-crlf | fence_opener_break | I4 | 2 | 5060 | 3 | 204 | 0 | 0 | 14.7 | 5060 | ok |
| synth-crlf | fence_opener_delete | I4 | 12 | 842 | 3 | 203 | 0 | 0 | 14.6 | 842 | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.8 | 25.0 | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.8 | 25.0 | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.8 | 25.0 | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.9 | 25.0 | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.8 | 25.0 | ok |
| synth-crlf | heading_from_para | I2 | 3 | 51.3 | 2 | 2 | 69 | 69 | 1.7 | 51.3 | ok |
| synth-crlf | large_delete_mid | - | 2142 | 0.1 | 1 | 2 | 55 | 55 | 1.7 | 0.1 | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 132.7 | 1.0 | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 129.5 | 1.0 | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 129.5 | 1.0 | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 129.4 | 1.0 | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 129.1 | 1.0 | ok |
| synth-crlf | list_indent_add | I3 | 2 | 49.5 | 1 | 5 | 140 | 0 | 2.4 | 49.5 | ok |
| synth-crlf | list_marker_add | I3 | 2 | 49.5 | 1 | 5 | 140 | 0 | 2.6 | 49.5 | ok |
| synth-crlf | list_marker_remove | I3 | 2 | 47.5 | 1 | 5 | 140 | 0 | 2.6 | 47.5 | ok |
| synth-crlf | mermaid_body_char | I6 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok |
| synth-crlf | para_delete_char | I0 | 1 | 73.0 | 2 | 2 | 146 | 0 | 2.6 | 71.0 | ok |
| synth-crlf | para_delete_char | I0 | 1 | 150 | 1 | 2 | 103 | 0 | 1.6 | 150 | ok |
| synth-crlf | para_delete_char | I0 | 1 | 150 | 1 | 2 | 69 | 0 | 1.4 | 150 | ok |
| synth-crlf | para_delete_char | I0 | 1 | 152 | 2 | 3 | 33 | 0 | 1.2 | 150 | ok |
| synth-crlf | para_delete_char | I0 | 1 | 150 | 1 | 2 | 1 | 0 | 0.9 | 150 | ok |
| synth-crlf | para_insert_char | I0 | 1 | 75.0 | 2 | 2 | 146 | 0 | 2.8 | 73.0 | ok |
| synth-crlf | para_insert_char | I0 | 1 | 152 | 1 | 2 | 103 | 0 | 1.8 | 152 | ok |
| synth-crlf | para_insert_char | I0 | 1 | 152 | 1 | 2 | 69 | 0 | 1.4 | 152 | ok |
| synth-crlf | para_insert_char | I0 | 1 | 154 | 2 | 3 | 33 | 0 | 1.3 | 152 | ok |
| synth-crlf | para_insert_char | I0 | 1 | 152 | 1 | 2 | 1 | 0 | 0.9 | 152 | ok |
| synth-crlf | para_split | I2 | 3 | 50.7 | 3 | 4 | 69 | 69 | 1.8 | 50.7 | ok |
| synth-crlf | para_sub_char | I0 | 2 | 37.0 | 2 | 2 | 0 | 0 | 1.6 | 36.0 | ok |
| synth-crlf | para_sub_char | I0 | 2 | 75.5 | 1 | 2 | 0 | 0 | 0.9 | 75.5 | ok |
| synth-crlf | para_sub_char | I0 | 2 | 75.5 | 1 | 2 | 0 | 0 | 0.9 | 75.5 | ok |
| synth-crlf | para_sub_char | I0 | 2 | 76.5 | 2 | 3 | 0 | 0 | 1.0 | 75.5 | ok |
| synth-crlf | para_sub_char | I0 | 2 | 75.5 | 1 | 2 | 0 | 0 | 0.9 | 75.5 | ok |
| synth-crlf | quote_add | I3 | 2 | 33.0 | 1 | 3 | 137 | 137 | 2.2 | 33.0 | ok |
| synth-crlf | quote_char | I3 | 1 | 40.0 | 1 | 2 | 138 | 0 | 1.8 | 40.0 | ok |
| synth-crlf | quote_remove | I3 | 2 | 31.0 | 2 | 3 | 137 | 0 | 2.0 | 31.0 | ok |
| synth-crlf | ref_def_add | I5 | 11 | 1.2 | 3 | 4 | 0 | 0 | 0.7 | 1.2 | ok |
| synth-crlf | ref_def_edit | I5 | 1 | 32.0 | 2 | 2 | 142 | 0 | 1.8 | 30.0 | ok |
| synth-crlf | ref_user_edit | I5 | 9 | 8.3 | 2 | 2 | 146 | 0 | 2.7 | 8.1 | ok |
| synth-crlf | setext_add | I2 | 4 | 38.8 | 1 | 3 | 69 | 0 | 1.5 | 38.8 | ok |

## Column notes

- `R` = bytes_scanned / changed_bytes (reparse amplification).
- `D_content` = proj_bytes_invalidated / changed_bytes: how much source a
position-keyed downstream consumer must re-project, ignoring pure offset
shifts (those appear as `surv_shift` / `rec_moved`).
- `conv_Δ` = convergence_line − restart_line (propagation distance, lines).
- `surv_shift` = survivor_blocks_shifted, `rec_moved` = block_records_moved
(the hidden-O(N) gate: metadata work that parsing locality alone hides).
