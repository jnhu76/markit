# MD4C vendored sources — provenance

- upstream: https://github.com/mity/md4c
- pinned version: **release-0.5.3**
- files: `md4c.c`, `md4c.h`, `entity.c`, `entity.h` (from `src/`), `LICENSE.md`
- tarball: https://github.com/mity/md4c/archive/refs/tags/release-0.5.3.tar.gz
- sha256 (individual files, recorded at vendor time 2026-09-16):
  - `md4c.c`  `f12907817a17ae7d0f6c8d18770df839f187cad5649dd36a475dba0675c5c1f8`
  - `md4c.h`  `4efd19bf7ec270691d5b4189f496886e421768a814b5e817eb945aa85e859f18`
- license: ISC (see LICENSE.md)
- purpose: M0-B full-parse baseline (issue #19 plan §9). Research-only;
  never linked from markit-core or apps.

Parser configuration used by the baseline: `flags = 0` (no MD_FLAG
extensions → pure CommonMark-style dialect; no tables/latex/tasks).
