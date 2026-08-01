(ns kotoba.reproducible-emit-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.reproducible-emit :as repro]))

(deftest checked-in-emit-digests-reproduce-here
  (let [result (repro/verify)]
    (is (:valid? result) (pr-str (:errors result)))
    (is (= 74 (:sources result)))
    (is (= 71 (:reproducible result)))))

(deftest every-source-is-recorded
  (let [recorded (set (keys (:emit-digests/wasm (repro/read-manifest))))]
    (is (= recorded (set (repro/sources)))
        "an unlisted source reports coverage the gate does not have")))

(deftest a-source-that-is-not-reproducible-says-why
  (is (every? (fn [{:keys [digest unsupported problem-kinds]}]
                (or digest (and unsupported (seq problem-kinds))))
              (vals (:emit-digests/wasm (repro/read-manifest))))))

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

(deftest an-unsupported-manifest-version-is-fail-closed
  (let [manifest (assoc (repro/read-manifest) :emit-digests/version 99)
        result (repro/verify manifest one-source)]
    (is (false? (:valid? result)))
    (is (= :unsupported-version (-> result :errors first :kind)))))
