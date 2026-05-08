#!/usr/bin/env bash
# Sync the full Autosync `space qud` sprite pack into the repo.
# Black/white mask PNGs → primary/secondary via palette_sprite_handle (see src/sprites.rs).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
src=${SPACE_QUD_SPRITES:-"$HOME/Bookmarks/mega/Autosync/sprites/space qud"}
dst=$here/../assets/textures/space_qud
mkdir -p "$dst"
cp -a "$src"/. "$dst"/
echo "Copied: $src -> $dst"
# Exact mirror (also drops files removed from source): rsync -a --delete "$src/" "$dst/"
