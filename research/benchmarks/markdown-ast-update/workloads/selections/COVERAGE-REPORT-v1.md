# COVERAGE-REPORT-v1 (CORRECTIVE-B §37-§38)

Candidate 3970 files / 66391919 bytes → G0 strict 530 files / 2872622 bytes → selected core (representative) 18 files / 115086 bytes; realism/syntax/full adds 18 files / 1938307 bytes.

## Feature-cell retention (candidate / eligible / selected)

| cell | candidates | eligible | selected |
|---|---|---|---|
| file_bytes=LOW | 647 | 177 | 6 |
| file_bytes=MEDIUM | 672 | 176 | 5 |
| file_bytes=HIGH | 2651 | 177 | 7 |
| block_count=LOW | 667 | 179 | 6 |
| block_count=MEDIUM | 800 | 176 | 6 |
| block_count=HIGH | 2503 | 175 | 6 |
| largest_block_bytes=LOW | 760 | 177 | 8 |
| largest_block_bytes=MEDIUM | 951 | 176 | 4 |
| largest_block_bytes=HIGH | 2259 | 177 | 6 |
| max_container_depth=ZERO | 959 | 137 | 6 |
| max_container_depth=LOW | 1771 | 356 | 9 |
| max_container_depth=HIGH | 1240 | 37 | 3 |
| fence_density_per_kib=ZERO | 1409 | 208 | 6 |
| fence_density_per_kib=LOW | 1382 | 108 | 4 |
| fence_density_per_kib=MEDIUM | 714 | 107 | 3 |
| fence_density_per_kib=HIGH | 465 | 107 | 5 |
| code_occupancy=ZERO | 1410 | 208 | 6 |
| code_occupancy=LOW | 1097 | 108 | 4 |
| code_occupancy=MEDIUM | 686 | 107 | 4 |
| code_occupancy=HIGH | 777 | 107 | 4 |
| reference_density_per_kib=ZERO | 2739 | 324 | 9 |
| reference_density_per_kib=LOW | 856 | 69 | 2 |
| reference_density_per_kib=MEDIUM | 208 | 68 | 3 |
| reference_density_per_kib=HIGH | 167 | 69 | 4 |
| cjk_byte_share=ZERO | 3826 | 479 | 14 |
| cjk_byte_share=LOW | 53 | 17 | 2 |
| cjk_byte_share=MEDIUM | 46 | 17 | 1 |
| cjk_byte_share=HIGH | 45 | 17 | 1 |

## Syntax/context cell coverage

| cell | required | candidate evidence | selected | best grade | gap |
|---|---|---|---|---|---|
| core:paragraph | required | 3969 | 35 | strict | - |
| core:heading_atx | required | 3534 | 32 | strict | - |
| core:list | required | 2978 | 26 | strict | - |
| core:block_quote | required | 564 | 3 | strict | - |
| core:code_block_fenced | required | 2561 | 23 | strict | - |
| core:emphasis | required | 2568 | 15 | strict | - |
| core:code_span | required | 3278 | 28 | strict | - |
| core:link_inline | required | 3241 | 22 | strict | - |
| core:link_reference | required | 686 | 14 | strict | - |
| core:reference_definition | required | 1231 | 16 | strict | - |
| syntax:table | required | 618 | 7 | strict | - |
| syntax:image | required | 408 | 7 | declared | - |
| syntax:link_autolink | required | 161 | 1 | declared | - |
| syntax:thematic_break | required | 1118 | 4 | declared | - |
| syntax:code_block_indented | required | 113 | 1 | declared | - |
| syntax:heading_setext | required | 1081 | 2 | declared | - |
| syntax:html_block | required | 1059 | 7 | declared | - |
| syntax:raw_html_inline | required | 431 | 4 | declared | - |
| syntax:math_inline_candidate | required | 307 | 2 | ambiguous_or_unknown | - |
| syntax:math_display_candidate | required | 157 | 1 | ambiguous_or_unknown | - |
| table:ordinary | required | 618 | 7 | strict | - |
| table:wide | required | 21 | 1 | strict | - |
| table:long | required | 50 | 3 | strict | - |
| table:alignment_marker | required | 111 | 2 | strict | - |
| table:escaped_pipe_candidate | required | 3 | 1 | candidate | - |
| table:inline_inside_table | required | 475 | 4 | strict | - |
| extra:strong | observed_extra | 2029 | 9 | declared | - |
| extra:hard_break | observed_extra | 97 | 2 | declared | - |
| extra:soft_break | observed_extra | 3441 | 24 | declared | - |
| extra:strikethrough | observed_extra | 29 | 2 | candidate | - |
| extra:task_list_item | observed_extra | 543 | 1 | candidate | - |
| extra:front_matter | observed_extra | 1069 | 2 | candidate | - |
| extra:directive | observed_extra | 321 | 4 | candidate | - |

## Extreme-role coverage

| dimension | observed max | max file | selected max | tail replicate |
|---|---|---|---|---|
| file_bytes | 841419.000 | cpp-core-guidelines/files/CppCoreGuidelines.md | 841419.000 | node/files/doc/api/fs.md |
| block_count | 8607.000 | cpp-core-guidelines/files/CppCoreGuidelines.md | 8607.000 | node/files/doc/api/fs.md |
| largest_block_bytes | 138502.000 | ethereum-eips/files/EIPS/eip-7643.md | 138502.000 | owasp-cheatsheets/files/cheatsheets/Kubernetes_Security_Cheat_Sheet.md |
| max_container_depth | 12.000 | kubernetes-keps/files/keps/sig-cluster-lifecycle/wgs/783-component-base/README.md | 12.000 | swift-evolution/files/proposals/0509-swift-sboms-via-swiftpm.md |
| fence_density_per_kib | 13.474 | myst-parser/files/docs/develop/_changelog.md | 13.474 | d2l-en/files/chapter_generative-adversarial-networks/index.md |
| code_occupancy | 0.985 | ethereum-eips/files/EIPS/eip-7643.md | 0.985 | d2l-en/files/chapter_appendix-tools-for-deep-learning/utils.md |
| reference_density_per_kib | 13.168 | oci-runtime/files/implementations.md | 13.168 | rust-rfcs/files/text/3672-Project-Goals-2024h2.md |
| cjk_byte_share | 0.984 | openmlsys/files/v1/zh_chapters/chapter_rl_sys/summary.md | 0.984 | NONE_AVAILABLE |

## Syntax inventory (§14)

| target | lane | grade | files | recognized | candidate | ambiguous | unknown | non-host | projects | domains |
|---|---|---|---|---|---|---|---|---|---|---|
| paragraph | G0 | strict_lane_coverage | 3969 | 326527 | 0 | 0 | 0 | 0 | 20 | 8 |
| heading_atx | G0 | strict_lane_coverage | 3534 | 80213 | 0 | 0 | 0 | 0 | 20 | 8 |
| list | G0 | strict_lane_coverage | 2978 | 52928 | 0 | 0 | 0 | 0 | 20 | 8 |
| block_quote | G0 | strict_lane_coverage | 564 | 1667 | 0 | 0 | 0 | 0 | 16 | 7 |
| code_block_fenced | G0 | strict_lane_coverage | 2561 | 27229 | 36 | 0 | 0 | 6 | 18 | 7 |
| emphasis | G0 | strict_lane_coverage | 2568 | 57063 | 0 | 0 | 0 | 0 | 20 | 8 |
| code_span | G0 | strict_lane_coverage | 3278 | 251621 | 0 | 0 | 0 | 0 | 20 | 8 |
| link_inline | G0 | strict_lane_coverage | 3241 | 66275 | 0 | 0 | 0 | 0 | 19 | 8 |
| link_reference | G0 | strict_lane_coverage | 686 | 10057 | 0 | 0 | 0 | 0 | 12 | 5 |
| reference_definition | G0 | strict_lane_coverage | 1231 | 13549 | 0 | 0 | 0 | 0 | 13 | 6 |
| table | G1 | strict_lane_coverage | 618 | 2022 | 4 | 0 | 0 | 7 | 18 | 7 |
| image | G1 | contract_declared_not_qualified | 408 | 1195 | 39 | 0 | 0 | 187 | 15 | 6 |
| link_autolink | G1 | contract_declared_not_qualified | 161 | 434 | 32 | 0 | 0 | 223 | 13 | 6 |
| thematic_break | G1 | contract_declared_not_qualified | 1118 | 1313 | 0 | 0 | 0 | 84 | 9 | 5 |
| code_block_indented | G1 | contract_declared_not_qualified | 113 | 1446 | 1021 | 0 | 0 | 7676 | 11 | 7 |
| heading_setext | G1 | contract_declared_not_qualified | 1081 | 1091 | 47 | 0 | 0 | 268 | 8 | 3 |
| html_block | G1 | contract_declared_not_qualified | 1059 | 15358 | 1974 | 0 | 0 | 18675 | 16 | 7 |
| raw_html_inline | G1 | contract_declared_not_qualified | 431 | 10440 | 28 | 0 | 0 | 24458 | 16 | 7 |
| strong | G1 | contract_declared_not_qualified | 2029 | 21719 | 300 | 0 | 0 | 3899 | 19 | 8 |
| hard_break | G1 | contract_declared_not_qualified | 97 | 398 | 0 | 0 | 0 | 0 | 9 | 4 |
| soft_break | G1 | contract_declared_not_qualified | 3441 | 228151 | 0 | 0 | 0 | 0 | 20 | 8 |
| strikethrough | G0 | out_of_lane_candidate | 0 | 0 | 47 | 0 | 0 | 4 | 0 | 0 |
| task_list_item | G0 | out_of_lane_candidate | 0 | 0 | 8124 | 0 | 0 | 36 | 0 | 0 |
| front_matter | G0 | out_of_lane_candidate | 0 | 0 | 1069 | 0 | 0 | 0 | 0 | 0 |
| directive | G0 | out_of_lane_candidate | 0 | 0 | 7082 | 0 | 0 | 4820 | 0 | 0 |
| inline_math | G1 | lane_deferred | 0 | 0 | 0 | 8870 | 0 | 384 | 0 | 0 |

## Spot checks (§45)

all_pass = true (13 checks; details in coverage-v1.json)
