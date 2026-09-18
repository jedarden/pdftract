#!/bin/sh
# Build and smoke-test the locally consumable Docker variants.
#
# This is intentionally POSIX shell so it can run in the docker:dind image used
# by the Argo CI workflow as well as on a developer workstation.

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture="${repo_root}/tests/fixtures/test-minimal.pdf"
variants=${PDFTRACT_DOCKER_VARIANTS:-"default full"}
report=${PDFTRACT_DOCKER_REPORT:-}
image_prefix=${PDFTRACT_DOCKER_IMAGE_PREFIX:-pdftract-verify}

if ! command -v docker >/dev/null 2>&1; then
    echo "verify-docker.sh: docker is required" >&2
    exit 2
fi
if [ ! -s "$fixture" ]; then
    echo "verify-docker.sh: smoke fixture is missing: $fixture" >&2
    exit 2
fi

if [ -n "$report" ]; then
    mkdir -p "$(dirname -- "$report")"
    : >"$report"
    printf '[\n' >"$report"
fi

first=true
for variant in $variants; do
    case "$variant" in
        default|full) ;;
        *)
            echo "verify-docker.sh: unsupported variant: $variant" >&2
            exit 2
            ;;
    esac

    image="${image_prefix}:${variant}"
    echo "=== building $image ==="
    docker build --pull --build-arg "FEATURES=${variant}" --tag "$image" "$repo_root"

    echo "=== checking $image --version ==="
    version=$(docker run --rm "$image" --version)
    test -n "$version"
    echo "$version"

    echo "=== checking $image compiled features ==="
    features=$(docker run --rm "$image" doctor --features)
    if [ "$variant" = default ]; then
        test -z "$features"
        feature_record='core-defaults:serde,decrypt,quick-xml,cache'
    else
        expected='cache grep inspect markdown mcp receipts serve'
        actual=$(printf '%s\n' "$features" | sort | tr '\n' ' ' | sed 's/[[:space:]]*$//')
        test "$actual" = "$expected"
        feature_record='cli:serve,mcp,inspect,grep,cache,receipts,markdown;core-defaults:serde,decrypt,quick-xml,cache'
    fi
    echo "features: $feature_record"

    echo "=== extracting W3C smoke fixture with $image ==="
    output=$(mktemp)
    trap 'rm -f "$output"' EXIT HUP INT TERM
    docker run --rm \
        --mount "type=bind,src=${fixture},dst=/tmp/test-minimal.pdf,readonly" \
        "$image" extract /tmp/test-minimal.pdf --json - >"$output"
    grep -F 'Dummy PDF file' "$output" >/dev/null
    rm -f "$output"
    trap - EXIT HUP INT TERM

    size=$(docker image inspect "$image" --format '{{.Size}}')
    test "$size" -gt 0
    echo "size_bytes: $size"

    if [ -n "$report" ]; then
        if [ "$first" = false ]; then
            printf ',\n' >>"$report"
        fi
        first=false
        escaped_features=$(printf '%s' "$feature_record" | sed 's/\\/\\\\/g; s/"/\\"/g')
        escaped_version=$(printf '%s' "$version" | sed 's/\\/\\\\/g; s/"/\\"/g')
        printf '  {"variant":"%s","image":"%s","version":"%s","features":"%s","size_bytes":%s}' \
            "$variant" "$image" "$escaped_version" "$escaped_features" "$size" >>"$report"
    fi
done

if [ -n "$report" ]; then
    printf '\n]\n' >>"$report"
    echo "Docker verification report: $report"
fi

echo 'Docker build and smoke verification passed.'
