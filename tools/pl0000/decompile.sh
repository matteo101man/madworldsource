#!/bin/sh
# Decompile DOL functions to C text with headless Ghidra (read-only on a scratch copy of a project).
# usage: sh tools/pl0000/decompile.sh <out.c> 802C2434+802C2528+...   (targets must be rows of tools/ghidra-functions.tsv)
# Output is derived from the owned main.dol: keep it outside the repo (scratch), cite addresses in research notes.
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WORK=${MW_GHIDRA_WORK:-${TMPDIR:-/tmp}/mw-ghidra}
[ -d "$WORK/MotionNative.rep" ] || { mkdir -p "$WORK"; cp -r "$ROOT/tools/ghidra-projects/MotionNative.gpr" "$ROOT/tools/ghidra-projects/MotionNative.rep" "$WORK/"; }
GHIDRA=${GHIDRA_HOME:-C:/Tools/ghidra_12.0.2_PUBLIC}
"$GHIDRA/support/analyzeHeadless.bat" "$(cygpath -w "$WORK")" MotionNative -process -noanalysis -readOnly \
  -scriptPath "$(cygpath -w "$ROOT/tools/ghidra-scripts")" -postScript MadWorldDecompileText.java \
  "$(cygpath -w "$ROOT")" "$2" "$(cygpath -w "$1")" > "$1.log" 2>&1
grep -c "^// =====" "$1"
