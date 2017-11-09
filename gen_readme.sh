#!/bin/bash

# https://stackoverflow.com/a/29613573/5069285
quoteSubst() {
  IFS= read -d '' -r < <(sed -e ':a' -e '$!{N;ba' -e '}' -e 's/[&/\]/\\&/g; s/\n/\\&/g' <<<"$1")
  printf %s "${REPLY%$'\n'}"
}

cp README_template.md README.md

sed -i "s/\[image\]/$(quoteSubst "$(termplay image --help)")/g;s/\[video\]/$(quoteSubst "$(termplay video --help)")/g;s/\[screen\]/$(quoteSubst "$(termplay screen --help)")/g" README.md
