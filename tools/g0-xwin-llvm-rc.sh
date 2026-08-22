#!/bin/sh
# G0 Windows cross-build helper (see docs/product/g0-gpui-baseline.md §7).
#
# Problem: the pinned GPUI rev's `resources/windows/gpui.rc` references its
# manifest as "resources/windows/gpui.manifest.xml" — a path relative to the
# crate root. embed-resource invokes llvm-rc with the CWD set to the .rc
# file's directory, so llvm-rc (unlike MSVC rc.exe with VS include paths)
# cannot resolve it.
#
# Fix: re-anchor the CWD two levels up (rc dir -> resources -> crate root)
# before exec'ing llvm-rc. The .rc input and /fo output paths passed by
# embed-resource are absolute, so they are unaffected.
#
# Usage (cross-build from WSL/Linux):
#   export RC_x86_64_pc_windows_msvc="$PWD/tools/g0-xwin-llvm-rc.sh"
#   export PATH="/usr/lib/llvm-22/bin:$PATH"   # llvm tools incl. llvm-rc
#   cargo xwin build --release -p markit --features g0-probe \
#       --target x86_64-pc-windows-msvc
#
# This is a build-environment workaround only: the Markit tree carries no
# GPUI source patches.
cd ../.. 2>/dev/null || true
exec llvm-rc "$@"
