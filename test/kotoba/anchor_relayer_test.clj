(ns kotoba.anchor-relayer-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.anchor-relayer :as relayer]))

(def policy (relayer/read-policy))
(def auth {:valid? true :audience (:audience policy)
           :subject "did:key:kotoba-audit-anchor"})
(def request {:anchor-digest "sha256:abc" :payload-cid "bafy-anchor"})

(deftest enqueue-is-authenticated-and-idempotent
  (is (= :relayer/unauthenticated
         (:code (relayer/enqueue policy (relayer/make-state)
                                 {:valid? false} request 1000))))
  (let [first-result (relayer/enqueue policy (relayer/make-state)
                                      auth request 1000)
        second-result (relayer/enqueue policy (:state first-result)
                                       auth request 2000)]
    (is (= :relayer/enqueued (:code first-result)))
    (is (= :relayer/idempotent (:code second-result)))
    (is (= 1 (count (get-in second-result [:state :jobs]))))
    (is (= 1 (count (get-in second-result [:state :events]))))))

(deftest retry-is-bounded-and-deterministically-scheduled
  (let [initial (:state (relayer/enqueue policy (relayer/make-state)
                                         auth request 1000))
        once (relayer/record-failure policy initial "sha256:abc"
                                     :rpc-timeout 1000)]
    (is (= :retry (get-in once [:jobs "sha256:abc" :status])))
    (is (= 2000 (get-in once [:jobs "sha256:abc" :next-at-ms])))
    (is (= ["sha256:abc"]
           (mapv :anchor-digest (relayer/due-jobs once 2000))))
    (let [exhausted
          (reduce (fn [state attempt]
                    (relayer/record-failure policy state "sha256:abc"
                                            :rpc-timeout (+ 2000 attempt)))
                  once (range 1 (:maximum-attempts policy)))]
      (is (= :failed (get-in exhausted [:jobs "sha256:abc" :status])))
      (is (= (:maximum-attempts policy)
             (get-in exhausted [:jobs "sha256:abc" :attempts]))))))

(deftest confirmation-requires-tx-match-and-finality-depth
  (let [queued (:state (relayer/enqueue policy (relayer/make-state)
                                        auth request 1000))
        submitted (relayer/record-submitted queued "sha256:abc"
                                            "0xtx" 100 1100)]
    (is (= :submitted
           (get-in (relayer/confirm policy submitted "sha256:abc"
                                    "wrong" 100 120 1200)
                   [:jobs "sha256:abc" :status])))
    (is (= :submitted
           (get-in (relayer/confirm policy submitted "sha256:abc"
                                    "0xtx" 100 111 1200)
                   [:jobs "sha256:abc" :status])))
    (is (= :confirmed
           (get-in (relayer/confirm policy submitted "sha256:abc"
                                    "0xtx" 100 112 1200)
                   [:jobs "sha256:abc" :status])))))

(deftest reconciliation-and-monitoring-expose-operational-failures
  (let [queued (:state (relayer/enqueue policy (relayer/make-state)
                                        auth request 0))
        submitted (relayer/record-submitted queued "sha256:abc"
                                            "0xtx" 1 1)
        confirmed (relayer/confirm policy submitted "sha256:abc"
                                   "0xtx" 1 13 2)]
    (is (= :reconcile/missing
           (:code (first (relayer/reconcile confirmed [])))))
    (is (= #{:reconcile/duplicate :reconcile/digest-mismatch}
           (set (map :code
                     (relayer/reconcile
                      confirmed
                      [{:anchor-digest "sha256:abc" :payload-cid "wrong"}
                       {:anchor-digest "sha256:abc" :payload-cid "wrong"}])))))
    (is (= 1 (:stale (relayer/monitoring policy queued 900000))))))
