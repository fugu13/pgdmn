#!/bin/sh
# Vendored dsntk management mechanics, driven by the vendor-* make targets.
# See vendor/README.md for the layering model this implements.
set -eu

CRATES="dsntk-common dsntk-examples dsntk-feel dsntk-feel-evaluator dsntk-feel-grammar dsntk-feel-number dsntk-feel-parser dsntk-feel-regex dsntk-feel-temporal dsntk-macros dsntk-model dsntk-model-evaluator dsntk-recognizer"

status() {
    pristine="$1"
    version=$(grep -m1 '^version' vendor/dsntk-feel/Cargo.toml | cut -d'"' -f2)
    echo "vendored dsntk version: $version"
    echo "pristine base commit:   ${pristine:-<none found>}"
    if [ -n "$pristine" ]; then
        echo "patch summaries: vendor/PATCHES.md"
        echo "patch layer (commits since pristine base touching vendor/):"
        git log --oneline --no-merges "$pristine"..HEAD -- vendor/ | sed 's/^/  /'
        echo "patch layer size (excluding vendor README/rustfmt config):"
        git diff --shortstat "$pristine" -- vendor/ ':(exclude)vendor/README.md' ':(exclude)vendor/PATCHES.md' ':(exclude)vendor/rustfmt.toml' ':(exclude)vendor/LICENSE-*' ':(exclude)vendor/NOTICE' | sed 's/^/  /'
    fi
}

check() {
    # All dsntk crates must resolve through the vendor patch, never the registry.
    bad=$(awk '/^name = "dsntk-/{n=$3} /^source = "registry/{if (n != "") print n; n=""} /^$/{n=""}' Cargo.lock)
    if [ -n "$bad" ]; then
        echo "vendor-check FAILED: dsntk crates resolving from the registry (patch unused):" >&2
        echo "$bad" >&2
        echo "Likely version skew between root Cargo.toml requirements and vendor/ - cargo only WARNS on this." >&2
        exit 1
    fi
    vendored=$(grep -m1 '^version' vendor/dsntk-feel/Cargo.toml | cut -d'"' -f2)
    required=$(grep -m1 '^dsntk-feel = ' Cargo.toml | cut -d'"' -f2)
    case "$vendored" in
        "$required"*) ;;
        *) echo "vendor-check FAILED: vendor holds dsntk $vendored but root Cargo.toml requires $required" >&2; exit 1 ;;
    esac
    echo "vendor-check OK: all dsntk crates resolve from vendor/ ($vendored)"
}

upgrade() {
    version="$1"
    if [ "$(git symbolic-ref --short HEAD 2>/dev/null)" = "main" ]; then
        echo "refusing to stage a pristine swap on main; create a branch first" >&2
        echo "(flow: branch -> make vendor-upgrade -> make vendor-inspect -> PR)" >&2
        exit 1
    fi
    stage=$(mktemp -d)
    trap 'rm -rf "$stage"' EXIT
    echo "downloading dsntk $version crates to $stage"
    for c in $CRATES; do
        url="https://static.crates.io/crates/$c/$c-$version.crate"
        curl -fsSL "$url" -o "$stage/$c.crate" || { echo "FAILED to download $c $version"; exit 1; }
        sha=$(shasum -a 256 "$stage/$c.crate" | cut -d' ' -f1)
        echo "  $c $version sha256=$sha"
        tar xzf "$stage/$c.crate" -C "$stage"
    done
    echo "swapping vendor/ to pristine $version (README/rustfmt.toml preserved)"
    for c in $CRATES; do
        rm -rf "vendor/$c"
        cp -R "$stage/$c-$version" "vendor/$c"
    done
    git add vendor
    git commit -m "Vendor pristine dsntk $version

Pristine upstream sources from crates.io (sha256 of each tarball in the
command output above); the PGDMN patch layer is re-applied on top by
follow-up commits — run 'make vendor-inspect' to drive that.

THIS COMMIT INTENTIONALLY DROPS the previous patch layer from the working
tree; the extension will not build until the layer is re-applied."
    echo
    echo "Pristine $version committed. Next: make vendor-inspect"
}

case "${1:-}" in
    status)  shift; status "${1:-}" ;;
    upgrade) shift; upgrade "$1" ;;
    check)   check ;;
    *) echo "usage: vendor.sh {status <pristine-sha>|upgrade <version>|check}"; exit 2 ;;
esac
