#!/bin/sh
set -eu

VERSION=${1:?usage: package-native-release.sh VERSION PLATFORM POLICY_PATH}
PLATFORM=${2:?usage: package-native-release.sh VERSION PLATFORM POLICY_PATH}
POLICY_PATH=${3:?usage: package-native-release.sh VERSION PLATFORM POLICY_PATH}

test -z "$(git status --porcelain --untracked-files=no)" || {
  echo "release packaging requires a clean tracked worktree" >&2
  exit 1
}

TEST_LOG=$(mktemp "${TMPDIR:-/tmp}/kotoba-release-tests.XXXXXX")
trap 'rm -f "$TEST_LOG"' EXIT
clojure -M:test > "$TEST_LOG"
cat "$TEST_LOG"
scripts/build-native.sh
mkdir -p target/release-evidence target/release-package
cp "$TEST_LOG" target/release-evidence/tests.txt

COMMIT=$(git rev-parse HEAD)
TREE=$(git rev-parse 'HEAD^{tree}')
node scripts/verify-release-binary.mjs target/native/kotoba \
  --release-version "$VERSION" \
  --language-profile 6 \
  --package-contract 1 \
  --commit "$COMMIT" \
  --tree "$TREE" \
  --platform "$PLATFORM"

cp target/native/kotoba LICENSE README.md target/release-package/
cp target/native/release-evidence.json \
  "target/release-evidence/kotoba-$PLATFORM.json"
tar -C target/release-package -czf "target/kotoba-$PLATFORM.tar.gz" kotoba LICENSE README.md
(cd target && shasum -a 256 "kotoba-$PLATFORM.tar.gz" > "kotoba-$PLATFORM.tar.gz.sha256")

clojure -M -m kotoba.release-build \
  "$VERSION" "$PLATFORM" "target/kotoba-$PLATFORM.tar.gz" \
  "target/release-evidence/kotoba-$PLATFORM.json" \
  target/release-evidence/tests.txt "$POLICY_PATH" \
  target/release-evidence/unsigned-envelope.edn

echo "unsigned envelope: target/release-evidence/unsigned-envelope.edn"
