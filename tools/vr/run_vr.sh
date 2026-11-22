#!/usr/bin/env bash
# Run mw-play once under the shared one-instance lock (.bevy-instance.lock), with the environment given as
# KEY=VALUE arguments, e.g.:  tools/vr/run_vr.sh MW_VR=sim MW_STAGE=street MW_PLAY_SHOTS="60:a.png"
# The lock is taken atomically (mkdir) and released when the game exits.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOCK="$ROOT/.bevy-instance.lock"
EXE="${MW_EXE:-D:/GDecomp/build-cache/bevy-0.20/debug/mw-play.exe}"
# (MW_EXE may also come as one of the KEY=VALUE arguments)
for a in "$@"; do case "$a" in MW_EXE=*) EXE="${a#MW_EXE=}";; esac; done
for i in $(seq 1 120); do
  if mkdir "$LOCK" 2>/dev/null; then break; fi
  if [ "$i" = 120 ]; then echo "lock busy: $(cat "$LOCK/owner.txt" 2>/dev/null)"; exit 3; fi
  sleep 5
done
trap 'rm -rf "$LOCK"' EXIT
echo "session=vr-port pid=$$ start=$(date -Iseconds) exe=$EXE" > "$LOCK/owner.txt"
env "$@" "$EXE"
