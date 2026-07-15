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
        echo "patch layer (commits since pristine base touching vendor/):"
        git log --oneline "$pristine"..HEAD -- vendor/ | sed 's/^/  /'
        echo "patch layer size (excluding vendor README/rustfmt config):"
        git diff --shortstat "$pristine" -- vendor/ ':(exclude)vendor/README.md' ':(exclude)vendor/rustfmt.toml' | sed 's/^/  /'
    fi
}

upgrade() {
    version="$1"
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
    *) echo "usage: vendor.sh {status <pristine-sha>|upgrade <version>}"; exit 2 ;;
esac
