(ns kotoba.test-runner
  "Aggregate test entry point for `clojure -M:test`."
  (:require [clojure.test :refer [run-tests]]
            [kotoba.actor-host-test]
            [kotoba.backend-qualification-test]
            [kotoba.aiueos-kernel-caps-test]
            [kotoba.cacao-run-test]
            [kotoba.cap-affine-test]
            [kotoba.cap-passing-test]
            [kotoba.cap-table-test]
            [kotoba.cap-typed-test]
            [kotoba.cljs-backend-test]
            [kotoba.did-adapter-test]
            [kotoba.doc-examples-test]
            [kotoba.guest-maturity-l2-test]
            [kotoba.guest-maturity-p0-p1-test]
            [kotoba.git-adapter-test]
            [kotoba.host-providers-test]
            [kotoba.kami-game-test]
            [kotoba.kgraph-test]
            [kotoba.launcher-test]
            [kotoba.language-conformance-test]
            [kotoba.mesh-drama-profile-test]
            [kotoba.mesh-node-test]
            [kotoba.package-admission-test]
            [kotoba.package-key-revoke-e2e-test]
            [kotoba.rad-adapter-test]
            [kotoba.real-host-providers-test]
            [kotoba.runtime-test]
            [kotoba.security-kaizen-test]
            [kotoba.sensing-host-test]
            [kotoba.semantic-code-test]
            [kotoba.type-contract-adapter-test]
            [kotoba.transport-component-test]
            [kotoba.wasm-and-or-when-test]
            [kotoba.wasm-exec-test]
            [kotoba.wasm-map-keyword-test]))

(defn -main [& _]
  (let [{:keys [fail error]} (run-tests 'kotoba.actor-host-test
                                        'kotoba.backend-qualification-test
                                        'kotoba.aiueos-kernel-caps-test
                                        'kotoba.cacao-run-test
                                        'kotoba.cap-affine-test
                                        'kotoba.cap-passing-test
                                        'kotoba.cap-table-test
                                        'kotoba.cap-typed-test
                                        'kotoba.cljs-backend-test
                                        'kotoba.did-adapter-test
                                        'kotoba.doc-examples-test
                                        'kotoba.git-adapter-test
                                        'kotoba.host-providers-test
                                        'kotoba.kami-game-test
                                        'kotoba.kgraph-test
                                        'kotoba.launcher-test
                                        'kotoba.language-conformance-test
                                        'kotoba.mesh-drama-profile-test
                                        'kotoba.mesh-node-test
                                        'kotoba.package-admission-test
                                        'kotoba.package-key-revoke-e2e-test
                                        'kotoba.rad-adapter-test
                                        'kotoba.real-host-providers-test
                                        'kotoba.runtime-test
                                        'kotoba.security-kaizen-test
                                        'kotoba.sensing-host-test
                                        'kotoba.semantic-code-test
                                        'kotoba.type-contract-adapter-test
                                        'kotoba.transport-component-test
                                        'kotoba.wasm-and-or-when-test
                                        'kotoba.wasm-exec-test
                                        'kotoba.wasm-map-keyword-test)]
    (when (pos? (+ (or fail 0) (or error 0)))
      (System/exit 1))))
