### E6-1

P1 total 159.0 samples/op; P0 total 40.5; excess 118.5

| Context/responsibility | P1 samples/op | P0 samples/op | Excess |
|---|---:|---:|---:|
| certificate | 103.79 | 0.00 | +103.79 |
| owner_materialize | 28.49 | 0.00 | +28.49 |
| avl_ownerseq | 6.18 | 0.00 | +6.18 |
| region_loop_other | 5.44 | 0.27 | +5.17 |
| full_build_other | 3.81 | 0.00 | +3.81 |
| block_parse_shared | 9.19 | 7.53 | +1.66 |
| coverage | 0.08 | 0.00 | +0.08 |
| span_coordinate | 0.03 | 0.00 | +0.03 |
| teardown | 1.98 | 3.52 | -1.54 |
| h0_document_materialize | 0.00 | 29.15 | -29.15 |

### E6-5

P1 total 157.1 samples/op; P0 total 41.5; excess 115.6

| Context/responsibility | P1 samples/op | P0 samples/op | Excess |
|---|---:|---:|---:|
| certificate | 103.72 | 0.00 | +103.72 |
| owner_materialize | 26.84 | 0.00 | +26.84 |
| avl_ownerseq | 6.30 | 0.00 | +6.30 |
| region_loop_other | 5.30 | 0.24 | +5.06 |
| full_build_other | 4.03 | 0.00 | +4.03 |
| block_parse_shared | 9.04 | 7.76 | +1.28 |
| coverage | 0.10 | 0.00 | +0.10 |
| span_coordinate | 0.06 | 0.00 | +0.06 |
| teardown | 1.74 | 3.24 | -1.50 |
| h0_document_materialize | 0.00 | 30.31 | -30.31 |

### E6-6

P1 total 9.7 samples/op; P0 total 7.3; excess 2.4

| Context/responsibility | P1 samples/op | P0 samples/op | Excess |
|---|---:|---:|---:|
| owner_materialize | 2.54 | 0.00 | +2.54 |
| certificate | 1.30 | 0.00 | +1.30 |
| avl_ownerseq | 0.55 | 0.00 | +0.55 |
| region_loop_other | 0.42 | 0.03 | +0.39 |
| full_build_other | 0.38 | 0.00 | +0.38 |
| block_parse_shared | 4.42 | 4.19 | +0.24 |
| coverage | 0.01 | 0.00 | +0.01 |
| span_coordinate | 0.01 | 0.00 | +0.01 |
| teardown | 0.08 | 0.21 | -0.13 |
| h0_document_materialize | 0.00 | 2.91 | -2.91 |

## Cross-cell bucket x cell (excess samples/op)

| Bucket | E6-1 | E6-5 | E6-6 |
|---|---:|---:|---:|
| certificate | +103.79 | +103.72 | +1.30 |
| owner_materialize | +28.49 | +26.84 | +2.54 |
| avl_ownerseq | +6.18 | +6.30 | +0.55 |
| region_loop_other | +5.17 | +5.06 | +0.39 |
| full_build_other | +3.81 | +4.03 | +0.38 |
| block_parse_shared | +1.66 | +1.28 | +0.24 |
| coverage | +0.08 | +0.10 | +0.01 |
| span_coordinate | +0.03 | +0.06 | +0.01 |
| teardown | -1.54 | -1.50 | -0.13 |
| h0_document_materialize | -29.15 | -30.31 | -2.91 |
