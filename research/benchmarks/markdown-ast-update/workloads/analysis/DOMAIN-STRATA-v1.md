# DOMAIN-STRATA-v1 — frozen source→domain mapping (CORRECTIVE-B §17)

Status: **committed INPUT, frozen before any selection runs**. Machine
form: `domain-strata-v1.json` (same directory). Strata are assigned from
project purpose only — never from parser behavior, eligibility, or any
performance fact.

## Population statement (§16)

> The frozen sampling frame is a curated population of retrievable
> open-source technical Markdown/documentation from the 20 pinned sources
> in PR #34. It is not a random sample of all Markdown usage.

It is not "all real Markdown", not the "average Markdown user workload",
and not the "general internet Markdown distribution".

## Closed vocabulary

```text
BOOK_TUTORIAL  API_TECHNICAL_DOC  STANDARD_SPECIFICATION
RFC_PROPOSAL_DESIGN  LARGE_GUIDE_REFERENCE  SECURITY_OPERATIONAL_DOC
ML_SCIENTIFIC_TECHNICAL  CJK_TECHNICAL  OTHER_DECLARED
```

## Mapping (20 sources)

| source | stratum | counts as role coverage |
|---|---|---|
| cpp-core-guidelines | LARGE_GUIDE_REFERENCE | yes |
| crafting-interpreters | OTHER_DECLARED | **no** — book text is HTML |
| cs231n | ML_SCIENTIFIC_TECHNICAL | yes |
| d2l-en | ML_SCIENTIFIC_TECHNICAL | yes |
| ethereum-eips | RFC_PROPOSAL_DESIGN | yes |
| graphql-spec | STANDARD_SPECIFICATION | yes |
| kubernetes-keps | RFC_PROPOSAL_DESIGN | yes |
| myst-parser | API_TECHNICAL_DOC | yes |
| mystmd | API_TECHNICAL_DOC | yes |
| node | API_TECHNICAL_DOC | yes |
| oci-image | STANDARD_SPECIFICATION | yes |
| oci-runtime | STANDARD_SPECIFICATION | yes |
| openapi | STANDARD_SPECIFICATION | yes |
| openmlsys | ML_SCIENTIFIC_TECHNICAL | yes |
| opentelemetry-spec | STANDARD_SPECIFICATION | yes |
| owasp-cheatsheets | SECURITY_OPERATIONAL_DOC | yes |
| progit | OTHER_DECLARED | **no** — book text is AsciiDoc |
| rust-book | BOOK_TUTORIAL | yes |
| rust-rfcs | RFC_PROPOSAL_DESIGN | yes |
| swift-evolution | RFC_PROPOSAL_DESIGN | yes |

## Known honest gaps recorded at freeze time

- `CJK_TECHNICAL` has **no member**: no acquisition source exists whose
  *project purpose* is CJK-language technical documentation. openmlsys
  contains zh translations (recorded per file by the parser-independent
  `cjk_byte_share` SOURCE_FACT), but its stratum is the ML-systems
  purpose, so mapping it to CJK_TECHNICAL would mislabel the project.
  The empty stratum is reported as a domain missing from the sampling
  frame (UNCOVERED-WORKLOAD-SPACE), not papered over.
- `crafting-interpreters` and `progit` carry the acquisition
  `role_caveat` (authoritative book text is HTML / AsciiDoc); they are
  classified `OTHER_DECLARED` with `counts_as_role_coverage = false`, so
  selection can never count them as book-workload coverage.
