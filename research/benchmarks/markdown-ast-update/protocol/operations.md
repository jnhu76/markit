# operations.md — AUTHORITY POINTER (implementation deferred)

The first-round operation set (`FULL_PARSE`, `INSERT`, `DELETE`,
`REPLACE_EQ`, `REPLACE_GROW`, `REPLACE_SHRINK`, `STRUCTURAL_EDIT`, `QUERY`)
and the structural minimum (six propagation families) are FROZEN in
`R0-METHODOLOGY.md` §8 and are not restated or modified here.

R1 materializes only the `OperationKind` type and the canonical
UTF-8 edit descriptor (`common::edit`). Operation/case GENERATION belongs
to R3+.
