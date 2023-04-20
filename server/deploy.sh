#!/bin/sh

set -e
set -u

cargo build \
  -p acrostic-server \
  --profile smol \
  --target x86_64-unknown-linux-musl \
  --target-dir target/x86_64-unknown-linux-musl

gcloud compute scp \
  target/x86_64-unknown-linux-musl/smol/acrostic-server \
  micro:/home/nathan/workspace/acrostic-club/target/x86_64-unknown-linux-musl/smol/acrostic-server \
  --zone us-west1-a \
  --project "acrostic-club"

gcloud compute ssh --zone "us-west1-a" "micro" --project "acrostic-club" -- \
  "cd /home/nathan/workspace/acrostic-club/ && \
  RUST_LOG=debug sudo ./target/x86_64-unknown-linux-musl/release/acrostic-server \
  --bind 0.0.0.0:443 \
  --cert /etc/letsencrypt/live/ws.acrostic.club/fullchain.pem \
  --key /etc/letsencrypt/live/ws.acrostic.club/privkey.pem \
  "
