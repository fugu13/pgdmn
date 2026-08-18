#!/bin/sh
# Commits every change under website/articles/ and website/public/ to a new
# branch and pushes it, with a commit message auto-generated from which
# articles were added/updated and whether they're drafts. Run via
# `make website-blog`.
set -eu

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

paths="website/articles website/public"

if git diff --quiet -- $paths \
  && git diff --cached --quiet -- $paths \
  && [ -z "$(git ls-files --others --exclude-standard -- $paths)" ]; then
  echo "blog-commit: no changes under website/articles/ or website/public/ to commit." >&2
  exit 1
fi

# Required so the new branch only ever carries blog content, never commits
# ridden along from whatever else main's checkout happened to be branched off.
current_branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$current_branch" != "main" ]; then
  echo "blog-commit: make website-blog must be run from main (currently on '$current_branch')." >&2
  exit 1
fi
git fetch origin main
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "blog-commit: main is behind origin/main -- pull before running make website-blog." >&2
  exit 1
fi

branch="blog-$(date +%Y%m%d-%H%M%S)"
git checkout -b "$branch"
git add -- $paths

# Prints "<title><TAB><draft>" for a git content spec readable by `git show`
# (":path" for staged, "HEAD:path" for the prior committed version), reading
# the file's front matter in one pass.
front_matter_fields() {
  git show "$1" 2>/dev/null | awk '
    NR == 1 && $0 == "---" { in_fm = 1; next }
    in_fm && $0 == "---" { exit }
    in_fm && $0 ~ /^title:/ {
      v = $0; sub(/^title:[ \t]*/, "", v); title = v
    }
    in_fm && $0 ~ /^draft:/ {
      v = $0; sub(/^draft:[ \t]*/, "", v); draft = v
    }
    END { printf "%s\t%s\n", title, draft }
  '
}

tmpfile="$(mktemp)"
trap 'rm -f "$tmpfile"' EXIT
git diff --cached --name-status --no-renames -- $paths > "$tmpfile"

entries=""
asset_count=0
while IFS="$(printf '\t')" read -r status path; do
  asset_count=$((asset_count + 1))
  case "$path" in
  website/articles/*.md) ;;
  *) continue ;;
  esac
  slug="$(basename "$path" .md)"
  spec=":$path"
  [ "$status" = "D" ] && spec="HEAD:$path"
  IFS="$(printf '\t')" read -r title draft <<EOF
$(front_matter_fields "$spec")
EOF
  title="${title:-$slug}"
  case "$status" in
  A)
    if [ "$draft" = "true" ]; then
      entry="added draft of \"$title\""
    else
      entry="added \"$title\""
    fi
    ;;
  M)
    if [ "$draft" = "true" ]; then
      entry="updated draft of \"$title\""
    else
      entry="published update of \"$title\""
    fi
    ;;
  D)
    entry="removed \"$title\""
    ;;
  *)
    continue
    ;;
  esac
  entries="${entries:+$entries, }$entry"
done < "$tmpfile"

if [ -z "$entries" ]; then
  subject="Update blog assets ($asset_count file(s) under website/articles or website/public)"
else
  first="$(printf '%s' "$entries" | cut -c1 | tr '[:lower:]' '[:upper:]')"
  rest="$(printf '%s' "$entries" | cut -c2-)"
  subject="$first$rest"
fi

git commit -m "$subject"
git push -u origin "$branch"

echo "blog-commit: pushed $branch: $subject"
