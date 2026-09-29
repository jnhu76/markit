### E6-1

P1 total 161.0 samples/op; P0 total 40.4; excess 120.6

| Context/responsibility | P1 samples/op | P0 samples/op | Excess |
|---|---:|---:|---:|
| certificate | 105.76 | 0.00 | +105.76 |
| owner_materialize | 27.51 | 0.00 | +27.51 |
| avl_ownerseq | 6.32 | 0.00 | +6.32 |
| region_loop_other | 6.00 | 0.34 | +5.66 |
| full_build_other | 3.88 | 0.00 | +3.88 |
| block_parse_shared | 9.26 | 7.71 | +1.55 |
| coverage | 0.07 | 0.00 | +0.07 |
| span_coordinate | 0.05 | 0.00 | +0.05 |
| teardown | 2.19 | 3.40 | -1.21 |
| h0_document_materialize | 0.00 | 28.96 | -28.96 |

### E6-5

P1 total 159.0 samples/op; P0 total 39.8; excess 119.2

| Context/responsibility | P1 samples/op | P0 samples/op | Excess |
|---|---:|---:|---:|
| certificate | 104.77 | 0.00 | +104.77 |
| owner_materialize | 26.96 | 0.00 | +26.96 |
| avl_ownerseq | 6.32 | 0.00 | +6.32 |
| region_loop_other | 5.11 | 0.16 | +4.95 |
| full_build_other | 4.19 | 0.00 | +4.19 |
| block_parse_shared | 9.16 | 7.61 | +1.55 |
| coverage | 0.13 | 0.00 | +0.13 |
| span_coordinate | 0.07 | 0.00 | +0.07 |
| teardown | 2.28 | 3.29 | -1.01 |
| h0_document_materialize | 0.00 | 28.73 | -28.73 |

### E6-6

P1 total 9.7 samples/op; P0 total 7.9; excess 1.8

| Context/responsibility | P1 samples/op | P0 samples/op | Excess |
|---|---:|---:|---:|
| owner_materialize | 2.55 | 0.00 | +2.55 |
| certificate | 1.30 | 0.00 | +1.30 |
| avl_ownerseq | 0.60 | 0.00 | +0.60 |
| region_loop_other | 0.44 | 0.04 | +0.40 |
| full_build_other | 0.34 | 0.00 | +0.34 |
| coverage | 0.01 | 0.00 | +0.01 |
| span_coordinate | 0.00 | 0.00 | +0.00 |
| teardown | 0.07 | 0.23 | -0.16 |
| block_parse_shared | 4.38 | 4.60 | -0.22 |
| h0_document_materialize | 0.00 | 3.04 | -3.04 |

## Cross-cell bucket x cell (excess samples/op)

| Bucket | E6-1 | E6-5 | E6-6 |
|---|---:|---:|---:|
| certificate | +105.76 | +104.77 | +1.30 |
| owner_materialize | +27.51 | +26.96 | +2.55 |
| avl_ownerseq | +6.32 | +6.32 | +0.60 |
| region_loop_other | +5.66 | +4.95 | +0.40 |
| full_build_other | +3.88 | +4.19 | +0.34 |
| block_parse_shared | +1.55 | +1.55 | -0.22 |
| coverage | +0.07 | +0.13 | +0.01 |
| span_coordinate | +0.05 | +0.07 | +0.00 |
| teardown | -1.21 | -1.01 | -0.16 |
| h0_document_materialize | -28.96 | -28.73 | -3.04 |
