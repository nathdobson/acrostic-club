#!/bin/sh
curl http://kaiko.getalp.org/static/ontolex/latest/en_dbnary_etymology.ttl.bz2 -o build/en_dbnary_etymology.ttl.bz2
bzip2 -d build/en_dbnary_etymology.ttl.bz2