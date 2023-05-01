#!/bin/sh
RUST_LOG=trace cargo run \
  -p acrostic-server \
  -- \
  --bind-https 0.0.0.0:443 \
  --bind-http 0.0.0.0:80 \
  --cert certs/certificate.pem \
  --key certs/key.pem