#!/bin/sh
set -eu

command -v clojure >/dev/null 2>&1 || { echo "build-native: clojure is required at build time" >&2; exit 1; }
command -v native-image >/dev/null 2>&1 || { echo "build-native: GraalVM native-image is required" >&2; exit 1; }

clojure -T:build uber
jar tf target/kotoba-standalone.jar | grep -q '^kotoba/launcher.class$' || {
  echo "build-native: AOT main class missing from uberjar" >&2
  exit 1
}
mkdir -p target/native
native-image \
  --no-fallback \
  -H:+ReportExceptionStackTraces \
  --features=clj_easy.graal_build_time.InitClojureClasses \
  --initialize-at-build-time=com.dylibso.chicory \
  '-H:IncludeResources=.*\.(clj|cljc|edn|properties)$' \
  -jar target/kotoba-standalone.jar \
  target/native/kotoba

target/native/kotoba selfhost check --json >/dev/null
target/native/kotoba compile src/demo.kotoba --target web \
  -o target/native/native-web-smoke.mjs --json >/dev/null
target/native/kotoba compile test/fixtures/source/web-library.kotoba --target web \
  -o target/native/native-web-library.mjs --json >/dev/null
node scripts/test-native-web-library.mjs target/native/native-web-library.mjs
target/native/kotoba compile test/fixtures/source/web-string-library.kotoba --target web \
  -o target/native/native-web-string.mjs --json >/dev/null
node scripts/test-native-web-string.mjs target/native/native-web-string.mjs
target/native/kotoba compile --project test/fixtures/project/kotoba-project.edn --target web \
  -o target/native/native-web-project.mjs --json
grep -q 'kotoba.artifact/module-graph-digest' target/native/native-web-project.mjs.manifest.edn
node scripts/test-native-web-project.mjs target/native/native-web-project.mjs
target/native/kotoba compile test/fixtures/typed-project/fixture/app.kotoba \
  --source-path test/fixtures/typed-project --unpinned --target web \
  -o target/native/native-web-typed-project.mjs --json
grep -q 'kotoba.artifact/module-graph-digest' \
  target/native/native-web-typed-project.mjs.manifest.edn
node scripts/test-native-typed-project.mjs target/native/native-web-typed-project.mjs
target/native/kotoba compile test/fixtures/typed-project/fixture/app.kotoba \
  --source-path test/fixtures/typed-project --unpinned --target wasm \
  -o target/native/native-wasm-typed-project.wasm --json
test -s target/native/native-wasm-typed-project.wasm
target/native/kotoba check --project test/fixtures/project-signed/kotoba-project.edn \
  --target web --json
target/native/kotoba compile --project test/fixtures/project-signed/kotoba-project.edn \
  --target web -o target/native/native-web-signed-project.mjs --json
grep -q 'kotoba.artifact/package-receipt-digest' \
  target/native/native-web-signed-project.mjs.manifest.edn
node scripts/test-native-web-project.mjs target/native/native-web-signed-project.mjs
