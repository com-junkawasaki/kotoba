(ns kotoba.reproducible-emit-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.reproducible-emit :as repro]))

(deftest checked-in-emit-digests-reproduce-here
  (let [result (repro/verify)]
    (is (:valid? result) (pr-str (:errors result)))
    (is (= 84 (:sources result)))
    (is (= 73 (:reproducible result)))))

(deftest every-source-is-recorded
  (let [recorded (set (keys (:emit-digests/wasm (repro/read-manifest))))]
    (is (= recorded (set (repro/sources)))
        "an unlisted source reports coverage the gate does not have")))

(deftest a-source-that-is-not-reproducible-says-why
  ;; `:wasm/check-failed` names a stage, not a reason, so it has to carry
  ;; `:problem-kinds`. Every other code already names the stage that could not
  ;; proceed -- `:wasm/binary-unsupported` is the whole answer for a source
  ;; whose checker passes and whose host ABI the binary emitter cannot lower.
  (is (every? (fn [{:keys [digest unsupported problem-kinds]}]
                (or digest
                    (and unsupported
                         (or (not= :wasm/check-failed unsupported)
                             (seq problem-kinds)))))
              (vals (:emit-digests/wasm (repro/read-manifest))))))

(deftest an-unknown-form-is-named-not-just-counted
  (let [detail (repro/failure-detail
                {:kotoba.cli/data
                 {:kotoba.runtime/result
                  {:kotoba.runtime/problems
                   [{:kotoba.runtime/problem :unknown-form
                     :kotoba.runtime/form "future-primitive"}]}}})]
    (is (= [:unknown-form] (:problem-kinds detail)))
    (is (= ["future-primitive"] (:unknown-forms detail))
        "a record that says a primitive is missing without saying which one
         cannot tell anyone what to implement")))

;; --- fail-closed cases ----------------------------------------------------
;;
;; Each mutates the recorded manifest rather than the emitter, so the emitted
;; bytes stay real and only the expectation moves.

(def one-source ["src/demo_bitops.kotoba"])

(defn- recorded-digest []
  (get-in (repro/read-manifest) [:emit-digests/wasm "src/demo_bitops.kotoba" :digest]))

(deftest digest-drift-is-fail-closed
  (let [manifest (assoc-in (repro/read-manifest)
                           [:emit-digests/wasm "src/demo_bitops.kotoba" :digest]
                           (str "sha256:" (apply str (repeat 64 "0"))))
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (= :digest-drift (-> result :errors first :kind)))))

(deftest a-source-outside-the-manifest-is-fail-closed
  (let [manifest (update (repro/read-manifest) :emit-digests/wasm
                         dissoc "src/demo_bitops.kotoba")
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (= :unlisted-source (-> result :errors first :kind)))))

(deftest a-source-recorded-but-deleted-is-fail-closed
  (let [manifest (assoc-in (repro/read-manifest)
                           [:emit-digests/wasm "src/does_not_exist.kotoba"]
                           {:digest (recorded-digest)})
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (some #(= :missing-source (:kind %)) (:errors result)))))

(deftest a-source-that-starts-emitting-must-be-re-recorded
  (let [manifest (assoc-in (repro/read-manifest)
                           [:emit-digests/wasm "src/demo_bitops.kotoba"]
                           {:unsupported :wasm/check-failed
                            :problem-kinds [:unknown-form]})
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (= :emit-now-supported (-> result :errors first :kind)))))

(deftest a-recorded-policy-that-does-not-exist-is-fail-closed
  (let [manifest (assoc-in (repro/read-manifest)
                           [:emit-digests/wasm "src/demo_bitops.kotoba" :policy]
                           "src/no_such_policy.edn")
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (= :missing-policy (-> result :errors first :kind)))))

(deftest a-recorded-policy-that-shadows-a-sibling-is-fail-closed
  ;; The bug this gate was added for: demo_string_host_sugar.kotoba had its own
  ;; sibling policy granting hash/sha256, and the manifest pinned an unrelated
  ;; shared policy instead. The source then recorded as capability-blocked, so
  ;; a bookkeeping mistake read as a missing capability.
  (let [source "src/demo_string_host_sugar.kotoba"
        manifest (assoc-in (repro/read-manifest)
                           [:emit-digests/wasm source :policy]
                           "src/demo_provider_policy.edn")
        result (repro/verify manifest [source])]
    (is (false? (:valid? result)))
    (is (= :policy-shadows-sibling (-> result :errors first :kind)))
    (is (= "src/demo_string_host_sugar_policy.edn"
           (-> result :errors first :sibling)))))

(deftest an-unsupported-manifest-version-is-fail-closed
  (let [manifest (assoc (repro/read-manifest) :emit-digests/version 99)
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (= :unsupported-version (-> result :errors first :kind)))))
