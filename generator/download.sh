#!/bin/sh
# Downloads the data the generator reads, keeping one shared copy in the main
# checkout's `build/` folder. Run from a worktree, the script also links that
# copy into the worktree's own `build/`. It skips files the shared copy already
# holds, so to refresh one, delete it from the main checkout's `build/` and
# rerun.

# `$0` is this script's path, so its folder's parent is the checkout root. `-P`
# resolves symbolic links, so `checkout_root` and `main_root` compare reliably.
checkout_root=$(cd "$(dirname "$0")/.." && pwd -P) || exit 1
cd "$checkout_root" || exit 1
# Git's common folder is the main checkout's `.git`, even in a worktree. Git may
# print it as a relative path, which the `cd` resolves.
main_git_folder=$(git rev-parse --git-common-dir) || exit 1
main_root=$(cd "$main_git_folder/.." && pwd -P) || exit 1
shared_build="$main_root/build"
mkdir -p "$shared_build" || exit 1

# Files take shape in a scratch folder and move into place once complete, so an
# interrupted run never leaves a file that looks finished. The `trap` deletes
# the scratch folder when the script exits, including after an error.
scratch=$(mktemp -d "$shared_build/download.XXXXXX") || exit 1
trap 'rm -rf "$scratch"' EXIT

# Downloads the URL `$1` to `$scratch/$2`, ending the script if the download
# fails.
fetch() {
  # `--fail` exits nonzero on an HTTP error instead of saving the error page.
  curl --fail --location --output "$scratch/$2" "$1" || exit 1
}

# Moves the finished `$scratch/$1` into the shared `build/`.
publish() {
  mv "$scratch/$1" "$shared_build/$1" || exit 1
}

dbnary=http://kaiko.getalp.org/static/ontolex/latest
for part in ontolex morphology etymology; do
  dictionary="en_dbnary_$part.ttl"
  if [ ! -e "$shared_build/$dictionary" ]; then
    fetch "$dbnary/$dictionary.bz2" "$dictionary.bz2"
    bzip2 --decompress "$scratch/$dictionary.bz2" || exit 1
    if [ "$part" = etymology ]; then
      # Turtle allows one language tag per label, but DBnary gives some labels
      # several, as in `"pincil"@xno,fro`, so the parser rejects the file. The
      # generator never reads labels, so drop them all.
      grep -v rdfs:label "$scratch/$dictionary" > "$scratch/filtered" || exit 1
      mv "$scratch/filtered" "$scratch/$dictionary" || exit 1
    fi
    publish "$dictionary"
  fi
done

if [ ! -e "$shared_build/lemma.en.txt" ]; then
  lemma_repository=https://raw.githubusercontent.com/skywind3000/lemma.en
  fetch "$lemma_repository/refs/heads/master/lemma.en.txt" lemma.en.txt
  publish lemma.en.txt
fi

if [ ! -e "$shared_build/xd/clues.tsv" ]; then
  fetch https://xd.saul.pw/xd-clues.zip xd-clues.zip
  # The archive holds `xd/clues.tsv`; `-d` sets the folder `unzip` extracts
  # into.
  unzip "$scratch/xd-clues.zip" -d "$scratch" || exit 1
  mkdir -p "$shared_build/xd" || exit 1
  publish xd/clues.tsv
fi

# In a worktree, link each shared file into the worktree's own `build/`.
if [ "$checkout_root" != "$main_root" ]; then
  mkdir -p "$checkout_root/build/xd" || exit 1
  for name in en_dbnary_ontolex.ttl en_dbnary_morphology.ttl \
    en_dbnary_etymology.ttl lemma.en.txt xd/clues.tsv; do
    # `-f` replaces whatever the worktree held under that name.
    ln -sf "$shared_build/$name" "$checkout_root/build/$name" || exit 1
  done
fi
