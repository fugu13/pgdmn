#!/bin/sh
# Creates website/articles/draft.md: a placeholder draft post, dated today,
# ready to fill in. Run via `make website-draft`.
set -eu

cd "$(dirname "$0")/.."

target="website/articles/draft.md"

if [ -e "$target" ]; then
  echo "new-draft: $target already exists -- rename or remove it before creating a new draft." >&2
  exit 1
fi

date_stamp="$(date +%Y-%m-%d)"

cat > "$target" <<EOF
---
title: TODO
date: $date_stamp
summary: TODO
draft: true
---

TODO
EOF

echo "new-draft: created $target"
