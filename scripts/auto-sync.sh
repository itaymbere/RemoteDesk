#!/usr/bin/env bash
set -euo pipefail

INTERVAL="${SYNC_INTERVAL:-15}"
BRANCH="$(git rev-parse --abbrev-ref HEAD)"

notify() {
  local msg="$1"
  echo "[$(date '+%H:%M:%S')] $msg"
  if command -v notify-send >/dev/null 2>&1; then
    notify-send "RemoteDeskIT" "$msg"
  elif command -v osascript >/dev/null 2>&1; then
    osascript -e "display notification \"$msg\" with title \"RemoteDeskIT\""
  fi
}

notify "Auto-sync ativo (acompanhando o branch ativo a cada ${INTERVAL}s)"

while true; do
  BRANCH="$(git rev-parse --abbrev-ref HEAD)"
  git fetch origin "$BRANCH" --quiet 2>/dev/null || true
  LOCAL="$(git rev-parse HEAD)"
  REMOTE="$(git rev-parse "origin/$BRANCH" 2>/dev/null || echo "$LOCAL")"

  if [ "$LOCAL" != "$REMOTE" ]; then
    if git pull --ff-only origin "$BRANCH" --quiet; then
      notify "Repositório atualizado ($BRANCH -> $(git rev-parse --short HEAD))"
    else
      notify "Novo commit em origin/$BRANCH (há alterações locais, faça pull manual)"
    fi
  fi

  sleep "$INTERVAL"
done
