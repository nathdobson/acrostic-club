#!/bin/sh

set -e
set -u

cargo build \
  -p acrostic-server \
  --profile smol \
  --target x86_64-unknown-linux-musl \
  --target-dir target/x86_64-unknown-linux-musl

gcloud compute ssh --zone "us-west1-a" "micro" --project "acrostic-club" -- \
 "screen -XS server quit;" || true

gcloud compute scp \
  target/x86_64-unknown-linux-musl/x86_64-unknown-linux-musl/smol/acrostic-server \
  micro:/home/nathan/workspace/acrostic-club/target/x86_64-unknown-linux-musl/smol/acrostic-server \
  --zone us-west1-a \
  --project "acrostic-club"

gcloud compute ssh --zone "us-west1-a" "micro" --project "acrostic-club" -- \
  "
    screen -S server sh -c '
    cd /home/nathan/workspace/acrostic-club
    sudo RUST_LOG=trace ./target/x86_64-unknown-linux-musl/smol/acrostic-server \
      --bind-https 0.0.0.0:443 \
      --bind-http 0.0.0.0:8080 \
      --cert /etc/letsencrypt/live/ws.acrostic.club/fullchain.pem \
      --key /etc/letsencrypt/live/ws.acrostic.club/privkey.pem
    sleep 60'
  "
