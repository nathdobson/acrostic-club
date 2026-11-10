#!/bin/sh
# Gives a worktree the main checkout's `build/` files as copy-on-write clones,
# so the generator can run there without rebuilding its outputs or storing a
# second copy of their many gigabytes. A clone shares its disk blocks with the
# original until either copy changes, so cloning costs almost no disk. `du`
# counts each clone at full size; `df` shows the disk actually free.
#
# Clones serve better than links here because the generator overwrites its
# outputs in place. Through a link, rerunning a stage in one worktree would
# overwrite the main checkout's copy, which every other worktree reads; with a
# clone, the rerun changes only that worktree's copy.
#
# The clones are a snapshot of the main checkout's `build/`, whose outputs came
# from whatever code last ran there. When a branch changes how an output is
# built, rerun that output's stage in the worktree. The snapshot includes the
# downloads too. The generator only reads those, so a clone of one serves as
# well as the link `download.sh` makes.
#
# Run the script from anywhere inside the worktree. The script skips each name
# the worktree's `build/` already holds, so to take the main checkout's copy of
# an output, delete the worktree's own first.

# Opens each message. `##*/` strips everything through the last slash of the
# script's path, leaving its name.
script_name=${0##*/}

# Work on the checkout holding the current folder, so one copy of the script
# serves every worktree. `-P` resolves symbolic links, so `checkout_root` and
# `main_root` compare reliably.
toplevel=$(git rev-parse --show-toplevel) || exit 1
cd "$toplevel" || exit 1
checkout_root=$(pwd -P)
# Git's common folder is the main checkout's `.git`, even in a worktree. Git may
# print it as a relative path, which the `cd` resolves.
main_git_folder=$(git rev-parse --git-common-dir) || exit 1
main_root=$(cd "$main_git_folder/.." && pwd -P) || exit 1

if [ "$checkout_root" = "$main_root" ]; then
  echo "$script_name: this is the main checkout, whose build/ the worktrees" \
    "clone from, so there is nothing to clone"
  exit 0
fi

shared_build="$main_root/build"
checkout_build="$checkout_root/build"
if [ ! -d "$shared_build" ]; then
  echo "$script_name: the main checkout has no build/ folder to clone from" >&2
  exit 1
fi
mkdir -p "$checkout_build" || exit 1

# Cloning works only within one filesystem. Across two, macOS's `cp -c` quietly
# falls back to a full copy, so check first. `df -P` prints a header line and
# then the filesystem's line, whose first field names the filesystem.
shared_filesystem=$(df -P "$shared_build" | awk 'NR == 2 { print $1 }')
checkout_filesystem=$(df -P "$checkout_build" | awk 'NR == 2 { print $1 }')
if [ "$shared_filesystem" != "$checkout_filesystem" ]; then
  echo "$script_name: $shared_build and $checkout_build are on different" \
    "filesystems, where cloning would copy every file in full" >&2
  exit 1
fi

# Clones `$1` into the folder `$2`, recursing into subfolders.
clone() {
  if [ "$(uname)" = Darwin ]; then
    # `-c` clones each file. Calling `/bin/cp` by its full path rules out a GNU
    # cp earlier on the `PATH`, which reads `-c` differently.
    /bin/cp -c -R "$1" "$2"
  else
    # `--reflink=always` clones each file, and fails rather than copying in
    # full where the filesystem can't clone.
    cp --reflink=always -R "$1" "$2"
  fi
}

# Clones take shape in a scratch folder and move into place once complete, so
# an interrupted run never leaves a partial output that looks finished. The
# `trap` deletes the scratch folder when the script exits, including after an
# error.
scratch=$(mktemp -d "$checkout_build/clone.XXXXXX") || exit 1
trap 'rm -rf "$scratch"' EXIT

for source in "$shared_build"/*; do
  # With no match, the pattern stays as written; `-e` skips it.
  [ -e "$source" ] || continue
  name=${source##*/}
  target="$checkout_build/$name"
  # `-L` also catches a broken link, which `-e` misses.
  if [ -e "$target" ] || [ -L "$target" ]; then
    echo "$script_name: kept build/$name, which this worktree already holds"
    continue
  fi
  clone "$source" "$scratch/" || exit 1
  mv "$scratch/$name" "$target" || exit 1
  echo "$script_name: cloned build/$name"
done
