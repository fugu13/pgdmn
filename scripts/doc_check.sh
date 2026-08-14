#!/bin/sh
# Verifies every SQL-facing (#[pg_extern]) function in src/functions/*.rs has
# a worked example in README.md and is covered by the website's Docs page
# (website/src/pages/docs.rs). See CI-004 in TODO.md and the "README.md"
# entry in CLAUDE.md's documentation table ("an example for every SQL
# function").
#
# Ground truth is extracted from source at run time, not hardcoded, so this
# check can never drift from the actual function list. Extraction relies on
# this repo's rustfmt convention: a `#[pg_extern...]` attribute sits on the
# line immediately above `pub fn NAME(...)`, with nothing else between them.
# If that assumption ever breaks (attribute count != extracted name count),
# the script fails loudly rather than silently under-checking.
set -eu

cd "$(dirname "$0")/.."

README="README.md"
DOCS_PAGE="website/src/pages/docs.rs"

functions=$(grep -A1 -h '^#\[pg_extern' src/functions/*.rs \
  | grep -oE '^pub fn [a-zA-Z_][a-zA-Z0-9_]*' \
  | awk '{print $3}' \
  | sort -u)

fn_count=$(printf '%s\n' "$functions" | grep -c . || true)
attr_count=$(grep -h -o '^#\[pg_extern' src/functions/*.rs | wc -l | tr -d ' ')

if [ "$fn_count" -ne "$attr_count" ] || [ "$fn_count" -eq 0 ]; then
  echo "doc_check: found $attr_count #[pg_extern] attribute(s) but extracted $fn_count function name(s) from src/functions/*.rs." >&2
  echo "doc_check: the 'attribute directly above pub fn NAME' extraction assumption may no longer hold -- fix this script before trusting its output." >&2
  exit 1
fi

missing_readme=""
missing_docs=""

# README convention: a function "has an example" when its name appears
# inside a fenced ```sql code block.
readme_sql_blocks=$(awk '/^```sql$/{f=1;next} /^```$/{f=0} f' "$README")

for fn in $functions; do
  if ! printf '%s\n' "$readme_sql_blocks" | grep -q -- "$fn"; then
    missing_readme="$missing_readme  - $fn
"
  fi
  # Website Docs page: a literal function-name-string search is enough --
  # this page is prose/`view!` markup, not a code reference to parse.
  if ! grep -q -- "$fn" "$DOCS_PAGE"; then
    missing_docs="$missing_docs  - $fn
"
  fi
done

status=0

if [ -n "$missing_readme" ]; then
  status=1
  printf 'doc_check: README.md has no ```sql example for:\n%s' "$missing_readme" >&2
fi

if [ -n "$missing_docs" ]; then
  status=1
  printf 'doc_check: website/src/pages/docs.rs does not mention:\n%s' "$missing_docs" >&2
fi

if [ "$status" -eq 0 ]; then
  echo "doc_check: all $fn_count SQL-facing functions are documented in README.md and the website Docs page."
fi

exit "$status"
