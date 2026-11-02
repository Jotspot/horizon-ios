#!/bin/sh
# Publishes one iOS build as a GitHub prerelease, `ios-build-<number>`, with Horizon.ipa and a
# changelog: the commits since the previous `ios-build-*` release (docs/ios.md). Run from a full
# clone (tags included) with GH_TOKEN and GH_REPO set; an existing release is left alone.
#
#   tools/ios_release.sh NUMBER COMMIT BRANCH IPA RUN_URL
set -eu
number=$1
commit=$2
branch=$3
ipa=$4
run_url=$5
tag="ios-build-$number"

if gh release view "$tag" >/dev/null 2>&1; then
    echo "$tag already released"
    exit 0
fi

# Releases made since this clone (a backfill makes several in a row).
git fetch -q --tags origin 2>/dev/null || true

# The newest earlier build's commit: the changelog starts after it.
previous=""
for t in $(git tag -l 'ios-build-*' | sed 's/^ios-build-//' | sort -n -r); do
    if [ "$t" -lt "$number" ]; then
        previous="ios-build-$t"
        break
    fi
done

notes=$(mktemp)
{
    echo "Unsigned test build $number of the iOS port, from \`$branch\` at $commit."
    echo
    echo "Install it with SideStore or AltStore, then copy your prepared"
    echo "\`.local/horizon-client/26.3\` into Files → On My iPhone → Horizon → horizon-client"
    echo "([iOS guide](https://github.com/$GH_REPO/blob/android/docs/ios.md))."
    echo
    if [ -n "$previous" ]; then
        echo "## Changes since $previous"
        range="$previous..$commit"
    else
        echo "## Changes"
        range="$commit~15..$commit"
    fi
    echo
    git log --no-merges --format='- %s' "$range" -- . || true
    echo
    echo "[Build log]($run_url)"
} >"$notes"

gh release create "$tag" "$ipa" \
    --prerelease \
    --target "$commit" \
    --title "iOS build $number ($branch)" \
    --notes-file "$notes"
rm -f "$notes"
