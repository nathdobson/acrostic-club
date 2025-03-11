#!/bin/sh
curl http://kaiko.getalp.org/static/ontolex/latest/en_dbnary_etymology.ttl.bz2 -o build/en_dbnary_etymology.ttl.bz2
bzip2 -d build/en_dbnary_etymology.ttl.bz2
curl http://kaiko.getalp.org/static/ontolex/latest/en_dbnary_ontolex.ttl.bz2 -o build/en_dbnary_ontolex.ttl.bz2
bzip2 -d build/en_dbnary_ontolex.ttl.bz2
curl http://kaiko.getalp.org/static/ontolex/latest/en_dbnary_morphology.ttl.bz2 -o build/en_dbnary_morphology.ttl.bz2
bzip2 -d build/en_dbnary_morphology.ttl.bz2
cat /Users/nathan/Documents/workspace/acrostic-club/build/en_dbnary_etymology.ttl | grep -v rdfs:label > /Users/nathan/Documents/workspace/acrostic-club/build/en_dbnary_etymology_fixed.ttl

curl https://raw.githubusercontent.com/skywind3000/lemma.en/refs/heads/master/lemma.en.txt -o build/lemma.en.txt

