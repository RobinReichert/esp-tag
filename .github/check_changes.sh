#!/bin/bash
set -e

BASE_BRANCH=$1
shift
PATTERNS=("$@")

echo "Base branch: $BASE_BRANCH"

git fetch origin "$BASE_BRANCH"

changed_files=$(git diff --name-only origin/"$BASE_BRANCH"...HEAD)

echo "Changed files:"
echo "$changed_files"

pattern=$(IFS="|"; echo "${PATTERNS[*]}")

echo "Looking for changes matching pattern: $pattern"

if echo "$changed_files" | grep -E "^($pattern)" >/dev/null 2>&1; then
  echo "changed=true" >> $GITHUB_OUTPUT
else
  echo "changed=false" >> $GITHUB_OUTPUT
fi
