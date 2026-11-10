#!/bin/sh
# Downloads the data the generator reads into `build/` at the root of the
# checkout that holds this script, so the script fills the right folder from any
# working directory, in the main checkout or a worktree.

# `$0` is this script's path, so its folder's parent is the checkout root.
checkout_root=$(cd "$(dirname "$0")/.." && pwd) || exit 1
build_folder="$checkout_root/build"
mkdir -p "$build_folder" || exit 1

# Downloads the URL `$1` to `build/$2`, ending the script if the download fails.
fetch() {
  # `--fail` exits nonzero on an HTTP error instead of saving the error page.
  curl --fail --location --output "$build_folder/$2" "$1" || exit 1
}

dbnary=http://kaiko.getalp.org/static/ontolex/latest
for part in ontolex morphology etymology; do
  fetch "$dbnary/en_dbnary_$part.ttl.bz2" "en_dbnary_$part.ttl.bz2"
  # `--force` replaces the file an earlier run decompressed.
  bzip2 --decompress --force "$build_folder/en_dbnary_$part.ttl.bz2" || exit 1
done

# Turtle allows one language tag per label, but DBnary gives some labels
# several, as in `"pincil"@xno,fro`, so the parser rejects the file. The
# generator never reads labels, so drop them all.
etymology="$build_folder/en_dbnary_etymology.ttl"
grep -v rdfs:label "$etymology" > "$etymology.filtered" || exit 1
mv "$etymology.filtered" "$etymology" || exit 1

lemma_repository=https://raw.githubusercontent.com/skywind3000/lemma.en
fetch "$lemma_repository/refs/heads/master/lemma.en.txt" lemma.en.txt

fetch https://xd.saul.pw/xd-clues.zip xd-clues.zip
# The archive holds `xd/clues.tsv`; `-d` puts it under `build/`, and `-o`
# replaces files an earlier run extracted, without asking.
unzip -o "$build_folder/xd-clues.zip" -d "$build_folder" || exit 1
