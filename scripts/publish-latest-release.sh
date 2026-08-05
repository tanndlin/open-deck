#!/bin/sh
set -eu

REPO="tanndlin/open-deck"
API="https://api.github.com/repos/$REPO"
SHORT_SHA=$(printf '%s' "$GIT_COMMIT" | cut -c1-7)

AUTH_HEADER="Authorization: Bearer $GITHUB_TOKEN"
ACCEPT_HEADER="Accept: application/vnd.github+json"

# Remove any existing "latest" release so re-creating the tag doesn't clash.
EXISTING_ID=$(curl -s -H "$AUTH_HEADER" -H "$ACCEPT_HEADER" "$API/releases/tags/latest" | jq -r '.id // empty')
if [ -n "$EXISTING_ID" ]; then
    echo "Deleting existing latest release ($EXISTING_ID)"
    curl -s -X DELETE -H "$AUTH_HEADER" -H "$ACCEPT_HEADER" "$API/releases/$EXISTING_ID" >/dev/null
fi

# Releases don't delete the underlying tag ref, so drop that separately.
curl -s -X DELETE -H "$AUTH_HEADER" -H "$ACCEPT_HEADER" "$API/git/refs/tags/latest" >/dev/null 2>&1 || true

echo "Creating latest release at $GIT_COMMIT"
CREATE_JSON=$(curl -s -X POST \
    -H "$AUTH_HEADER" -H "$ACCEPT_HEADER" \
    "$API/releases" \
    -d "{\"tag_name\":\"latest\",\"target_commitish\":\"$GIT_COMMIT\",\"name\":\"Latest build ($SHORT_SHA)\",\"body\":\"Automated rolling build from commit $GIT_COMMIT.\",\"prerelease\":true}")

UPLOAD_URL=$(printf '%s' "$CREATE_JSON" | jq -r '.upload_url' | sed 's/{?name,label}//')
if [ -z "$UPLOAD_URL" ] || [ "$UPLOAD_URL" = "null" ]; then
    echo "Failed to create release:" >&2
    printf '%s\n' "$CREATE_JSON" >&2
    exit 1
fi

upload_asset() {
    binary="$1"
    asset_name="$2"

    if [ ! -f "$binary" ]; then
        echo "Skipping $asset_name: $binary not found" >&2
        return 1
    fi

    echo "Uploading $binary as $asset_name"
    curl -s -X POST \
        -H "$AUTH_HEADER" \
        -H "Content-Type: application/octet-stream" \
        --data-binary @"$binary" \
        "$UPLOAD_URL?name=$asset_name" >/dev/null

    echo "Published: https://github.com/$REPO/releases/download/latest/$asset_name"
}

upload_asset "target/release/open-deck" "open-deck-linux-x86_64"
upload_asset "target/x86_64-pc-windows-gnu/release/open-deck.exe" "open-deck-windows-x86_64.exe"
