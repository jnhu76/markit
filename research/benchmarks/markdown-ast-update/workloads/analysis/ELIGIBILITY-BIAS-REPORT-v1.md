# ELIGIBILITY-BIAS-REPORT-v1 (CORRECTIVE-B §19-§20)

## Eligibility classes per lane

| lane | all | lane_valid | strict | blocker-bearing | deferred | strict bytes |
|---|---|---|---|---|---|---|
| BENCH-GRAMMAR-v1 | 3970 | 3970 | 530 | 3440 | 0 | 2872622 |
| COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1 | 3970 | 3970 | 1995 | 1975 | 0 | 27654304 |
| MARKIT-EXT-MATH-v1 | 3970 | 0 | 0 | 0 | 3970 | 0 |

## Universe vs G0-strict subset, per dimension

| feature | universe p50 | strict p50 | universe p95 | strict p95 | universe max | strict max |
|---|---|---|---|---|---|---|
| file_bytes | 9160.500 | 3151.500 | 55560.950 | 17239.100 | 841419.000 | 64700.000 |
| line_count | 180.000 | 82.500 | 1178.550 | 394.750 | 23161.000 | 1183.000 |
| block_count | 56.000 | 28.000 | 402.550 | 100.000 | 8607.000 | 542.000 |
| largest_block_bytes | 726.500 | 497.000 | 3768.750 | 1913.950 | 138502.000 | 25021.000 |
| max_container_depth | 2.000 | 2.000 | 6.000 | 4.000 | 12.000 | 6.000 |
| fence_density_per_kib | 0.188 | 0.335 | 1.497 | 2.026 | 13.474 | 13.474 |
| code_occupancy | 0.055 | 0.058 | 0.431 | 0.502 | 0.985 | 0.977 |
| reference_density_per_kib | 0.000 | 0.000 | 1.621 | 2.762 | 13.168 | 5.056 |
| cjk_byte_share | 0.000 | 0.000 | 0.000 | 0.836 | 0.984 | 0.984 |

## Proposal/RFC concentration (§20)

- ethereum-eips, kubernetes-keps, rust-rfcs, swift-evolution account for 2865 files (72.17%) and 46858602 bytes (70.58%) of the candidate universe.
- Selected-set share: 22.22%.
- This is a bias fact about the sampling frame, not an automatic corpus failure.

## Per-project eligibility (top blockers)

| project | domain | candidates | G0 strict | G1 strict | top G0 blockers |
|---|---|---|---|---|---|
| cpp-core-guidelines | LARGE_GUIDE_REFERENCE | 1 | 0 | 1 | raw_html_inline×1916, code_block_indented×1427, strong×119, directive×79, link_autolink×14 |
| crafting-interpreters | OTHER_DECLARED | 50 | 32 | 50 | code_block_indented×330, raw_html_inline×18, strong×3, thematic_break×1 |
| cs231n | ML_SCIENTIFIC_TECHNICAL | 30 | 1 | 2 | strong×937, html_block×673, display_math×212, inline_math×203, raw_html_inline×110 |
| d2l-en | ML_SCIENTIFIC_TECHNICAL | 191 | 20 | 44 | inline_math×6243, display_math×780, directive×711, strong×571, image×232 |
| ethereum-eips | RFC_PROPOSAL_DESIGN | 956 | 0 | 0 | strong×2968, front_matter×956, table×582, inline_math×571, raw_html_inline×118 |
| graphql-spec | STANDARD_SPECIFICATION | 15 | 1 | 7 | directive×576, strong×216, html_block×13, raw_html_inline×10, table×7 |
| kubernetes-keps | RFC_PROPOSAL_DESIGN | 678 | 12 | 147 | strong×9403, html_block×9302, task_list_item×8034, raw_html_inline×1888, code_block_indented×540 |
| myst-parser | API_TECHNICAL_DOC | 27 | 7 | 8 | directive×532, strong×53, link_autolink×31, html_block×27, display_math×12 |
| mystmd | API_TECHNICAL_DOC | 89 | 1 | 2 | directive×806, strong×326, front_matter×84, inline_math×57, image×41 |
| node | API_TECHNICAL_DOC | 70 | 0 | 11 | html_block×5747, directive×3944, raw_html_inline×2787, strong×1747, table×31 |
| oci-image | STANDARD_SPECIFICATION | 18 | 5 | 16 | strong×90, code_block_indented×53, task_list_item×18, link_autolink×11, table×8 |
| oci-runtime | STANDARD_SPECIFICATION | 21 | 1 | 19 | strong×369, raw_html_inline×154, code_block_indented×22, task_list_item×18, link_autolink×7 |
| openapi | STANDARD_SPECIFICATION | 23 | 11 | 11 | raw_html_inline×1986, html_block×941, table×516, strong×252, inline_math×90 |
| openmlsys | ML_SCIENTIFIC_TECHNICAL | 234 | 90 | 167 | inline_math×1506, strong×599, image×420, display_math×161, directive×117 |
| opentelemetry-spec | STANDARD_SPECIFICATION | 99 | 8 | 95 | raw_html_inline×1407, html_block×627, strong×563, table×74, image×28 |
| owasp-cheatsheets | SECURITY_OPERATIONAL_DOC | 122 | 14 | 113 | strong×2009, thematic_break×74, table×65, html_block×64, image×58 |
| progit | OTHER_DECLARED | 3 | 2 | 3 | table×2 |
| rust-book | BOOK_TUTORIAL | 112 | 17 | 111 | html_block×1422, raw_html_inline×302, strong×89, table×13, inline_math×2 |
| rust-rfcs | RFC_PROPOSAL_DESIGN | 651 | 307 | 620 | strong×1848, code_block_indented×691, raw_html_inline×310, html_block×207, table×201 |
| swift-evolution | RFC_PROPOSAL_DESIGN | 580 | 1 | 568 | strong×1942, raw_html_inline×786, code_block_indented×450, html_block×324, table×127 |

Notes:
- G0 eligibility materially changes the corpus: 530/3970 candidates (13.4%) are G0 strict-scope clean; all bias dimensions below compare the universe against this reduced strict subset
- G1 strict evidence is table-scoped only (the single semantically qualified G1 construct); G1 base CommonMark kinds are contract_declared_not_qualified and are never counted as strict coverage
- G2 lane_valid is false for every candidate: math semantics are deferred, so every math-looking candidate is ambiguous/unknown evidence, never occupancy
