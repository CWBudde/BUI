#!/usr/bin/env sh

set -xe

# Run from the directory the command is meant for, inside the repository.
REL="$(git rev-parse --show-prefix)"

# Create a temporary directory and delete it once the script is done
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Create a copy of this directory's *tracked* files in the temporary directory, at the
# same path below it as in the repository
git ls-files -z | xargs -0 git checkout-index --prefix "$TMP/" --
cd "$TMP"

# Initialize as git repo to enable diff
git init . 2>/dev/null
git add . 2>/dev/null
cd "$TMP/$REL"

# Run the given command, that may make changes to the source.
"$@"

# Print all changes and return with a non zero error if anything changed.
git diff --exit-code --ignore-space-at-eol
