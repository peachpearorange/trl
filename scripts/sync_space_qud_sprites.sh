#!/usr/bin/env bash
# Sync the full Autosync `space qud` sprite pack into the repo.
# Black/white mask PNGs → primary/secondary via palette_sprite_handle (see src/sprites.rs).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
src=${SPACE_QUD_SPRITES:-"$HOME/Bookmarks/mega/Autosync/sprites/space qud"}
dst=$here/../assets/textures/space_qud
mkdir -p "$dst"
# rsync skips files where mtime+size both match (already up to date),
# copies when source is newer. No -u: always restore from source if different.
rsync -a "$src"/ "$dst"/
echo "Synced: $src -> $dst"
# Exact mirror (also drops files removed from source): rsync -a --delete "$src/" "$dst/"
