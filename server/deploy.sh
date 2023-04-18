#!/bin/sh

set -e
set -u

cargo build -p acrostic-server --release --target x86_64-unknown-linux-musl

gcloud compute scp \
  target/x86_64-unknown-linux-musl/release/acrostic-server \
  micro:/home/nathan/workspace/acrostic-club/target/x86_64-unknown-linux-musl/release/acrostic-server \
  --zone us-west1-a \
  --project "acrostic-club"

gcloud compute ssh --zone "us-west1-a" "micro" --project "acrostic-club" -- \
  "cd /home/nathan/workspace/acrostic-club/ && ./target/x86_64-unknown-linux-musl/release/acrostic-server"
