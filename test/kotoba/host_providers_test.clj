(ns kotoba.host-providers-test
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.host-providers :as host-providers]
            [kotoba.launcher :as launcher]
            [kotoba.runtime :as runtime])
  (:import [java.io File]))

(defn temp-edn-file
  [content]
  (let [file (doto (File/createTempFile "kotoba-host-policy" ".edn")
               (.deleteOnExit))]
    (spit file (pr-str content))
    (.getPath file)))

(deftest policy-derives-grants-and-local-policy-per-host-kind
  (let [policy {:kotoba.policy/capabilities #{:clipboard/text :ledger/append}
                :kotoba.policy/capability-resources {:clipboard/text #{"clipboard:system"}}
                :kotoba.policy/capability-expires {:ledger/append "2027-01-01"}}
        grants (host-providers/policy-grants policy)
        by-kind (into {} (map (juxt :grant/kind identity)) grants)]
    (testing "clipboard/text enables both clipboard kinds, scoped by resources"
      (is (= #{"clipboard:system"}
             (get-in by-kind [:host/clipboard-read :grant/resources])))
      (is (= #{"clipboard:system"}
             (get-in by-kind [:host/clipboard-write :grant/resources])))
      (is (= "policy:clipboard/text"
             (get-in by-kind [:host/clipboard-read :grant/id]))))
    (testing "ledger/append grant carries the policy expiry and :any scope"
      (is (= #{:any} (get-in by-kind [:host/ledger-append :grant/resources])))
      (is (= "2027-01-01" (get-in by-kind [:host/ledger-append :grant/expires]))))
    (testing "no grant for capabilities the policy does not name"
      (is (nil? (:host/http by-kind))))
    (testing "local policy mirrors the grant scopes"
      (is (= {:host/clipboard-read #{"clipboard:system"}
              :host/clipboard-write #{"clipboard:system"}
              :host/ledger-append #{:any}}
             (:policy/allow (host-providers/local-policy policy)))))))

(deftest guarded-run-executes-provider-and-emits-concrete-receipt
  (let [result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--policy" "src/demo_i64_host_policy.edn"
                                   "--json"])
        receipts (get-in result [:kotoba.cli/data :kotoba.host/receipts])
        receipt (first receipts)]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= :run/completed (:kotoba.cli/code result)))
    (is (= 41 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                              :kotoba.runtime/value])))
    (is (= 1 (count receipts)))
    (is (= :ok (:receipt/outcome receipt)))
    (is (= :kotoba.host/host-i64-roundtrip (:receipt/call receipt)))
    (is (= :host/ledger-append (get-in receipt [:receipt/cap :cap/kind])))
    (is (= :any (get-in receipt [:receipt/cap :cap/resource])))
    (is (= ["policy:ledger/append"] (get-in receipt [:receipt/cap :cap/provenance])))
    (is (= "src/demo_i64_host_policy.edn"
           (get-in result [:kotoba.cli/data :kotoba.policy/path])))))

(deftest guarded-run-narrows-receipt-to-policy-resources
  (let [policy-path (temp-edn-file
                     {:kotoba.policy/capabilities #{:ledger/append}
                      :kotoba.policy/capability-resources {:ledger/append #{"ledger:main"}}})
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--policy" policy-path "--json"])
        receipt (first (get-in result [:kotoba.cli/data :kotoba.host/receipts]))]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= 41 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                              :kotoba.runtime/value])))
    (is (= "ledger:main" (get-in receipt [:receipt/cap :cap/resource])))))

(deftest guarded-run-denies-expired-grant-with-denial-receipt
  (let [policy-path (temp-edn-file
                     {:kotoba.policy/capabilities #{:ledger/append}
                      :kotoba.policy/capability-expires {:ledger/append "2020-01-01"}})
        result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--policy" policy-path "--json"])
        runtime-result (get-in result [:kotoba.cli/data :kotoba.runtime/result])
        receipts (get-in result [:kotoba.cli/data :kotoba.host/receipts])
        receipt (first receipts)]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :run/failed (:kotoba.cli/code result)))
    (is (= [{:kotoba.runtime/problem :host-call-denied
             :kotoba.runtime/call 'host-i64-roundtrip
             :kotoba.runtime/denied :expired}]
           (:kotoba.runtime/problems runtime-result)))
    (is (not (contains? runtime-result :kotoba.runtime/value)))
    (is (= 1 (count receipts)))
    (is (= :denied (:receipt/outcome receipt)))
    (is (= :expired (:receipt/denied receipt)))))

(deftest denied-host-call-never-invokes-provider-handler
  (let [calls (atom 0)
        policy {:kotoba.policy/capabilities #{:ledger/append}
                :kotoba.policy/capability-expires {:ledger/append "2020-01-01"}}
        recorded (atom [])
        host-call (host-providers/host-call
                   policy
                   {:record! (fn [receipt] (swap! recorded conj receipt))
                    :handlers {'host-i64-roundtrip
                               (fn [_cap args] (swap! calls inc) (first args))}})
        thrown (try
                 (host-call 'host-i64-roundtrip [41])
                 nil
                 (catch clojure.lang.ExceptionInfo e (ex-data e)))]
    (is (= :expired (:kotoba.host/denied thrown)))
    (is (= 'host-i64-roundtrip (:kotoba.host/call thrown)))
    (is (zero? @calls))
    (is (= 1 (count @recorded)))
    (is (= :denied (:receipt/outcome (first @recorded))))))

(deftest granted-host-call-passes-concrete-cap-to-injected-handler
  (let [seen (atom nil)
        policy {:kotoba.policy/capabilities #{:ledger/append}
                :kotoba.policy/capability-resources {:ledger/append #{"ledger:main"}}}
        host-call (host-providers/host-call
                   policy
                   {:handlers {'host-i64-roundtrip
                               (fn [cap args] (reset! seen cap) (first args))}})]
    (is (= 41 (host-call 'host-i64-roundtrip [41])))
    (is (= :host/ledger-append (:cap/kind @seen)))
    (is (= "ledger:main" (:cap/resource @seen)))))

(deftest host-effect-denies-classification-downgrade-before-provider
  (let [calls (atom 0)
        receipts (atom [])
        policy {:kotoba.policy/capabilities #{:ledger/append}
                :kotoba.policy/information-flow
                {:subject :runtime/service :purpose :audit-export
                 :input-classifications [:confidential]
                 :output-classification :public}}
        host-call (host-providers/host-call
                   policy
                   {:now "2026-07-19T12:00:00Z"
                    :record! #(swap! receipts conj %)
                    :handlers {'host-i64-roundtrip
                               (fn [_ _] (swap! calls inc) 0)}})
        denied (try (host-call 'host-i64-roundtrip [41]) nil
                    (catch clojure.lang.ExceptionInfo e (ex-data e)))]
    (is (= :information-flow (:kotoba.host/denied denied)))
    (is (zero? @calls))
    (is (= :information-flow (:receipt/denied (first @receipts))))
    (is (false? (get-in @receipts [0 :receipt/information-flow
                                   :information-flow/allowed?])))))

(deftest kgraph-handlers-real-eavt-round-trip
  (testing "kgraph-handlers (kotoba.kgraph-backed, not a 0-returning stub) really stores and queries"
    (let [store (atom [])
          policy {:kotoba.policy/capabilities #{:graph/kotoba}}
          host-call (host-providers/host-call policy {:handlers (host-providers/kgraph-handlers store)})]
      (is (zero? (host-call 'kgraph-assert! ["[1 :name :aoi]"])))
      (is (zero? (host-call 'kgraph-assert! ["[1 :age 7]"])))
      (is (= 2 (count @store)))
      (is (= [[:aoi]]
             (edn/read-string
              (host-call 'kgraph-query ["{:find [?v] :where [[1 :name ?v]]}"]))))
      (is (= [[1 :name :aoi] [1 :age 7]]
             (edn/read-string (host-call 'kgraph-get-objects ["1"]))))
      (is (zero? (host-call 'kgraph-retract! ["[1 :age 7]"])))
      (is (= 1 (count @store))))))

(deftest guarded-run-binds-has-capability-query
  (let [result (launcher/dispatch ["run" "src/demo_cap.kotoba"
                                   "--policy" "src/demo_policy.edn"
                                   "--json"])]
    (is (true? (:kotoba.cli/ok? result)))
    (is (= 7 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                             :kotoba.runtime/value])))
    (testing "capability queries are not host effects and leave no receipts"
      (is (= [] (get-in result [:kotoba.cli/data :kotoba.host/receipts]))))))

(deftest legacy-no-policy-run-is-unchanged
  (testing "pure source runs exactly as before, with no receipts key"
    (let [result (launcher/dispatch ["run" "src/demo.kotoba" "--json"])]
      (is (true? (:kotoba.cli/ok? result)))
      (is (= 42 (get-in result [:kotoba.cli/data :kotoba.runtime/result
                                :kotoba.runtime/value])))
      (is (not (contains? (:kotoba.cli/data result) :kotoba.host/receipts)))))
  (testing "host-import source without policy keeps the ambient static denial"
    (let [result (launcher/dispatch ["run" "src/demo_i64_host.kotoba" "--json"])]
      (is (false? (:kotoba.cli/ok? result)))
      (is (= :run/failed (:kotoba.cli/code result)))
      (is (= :capability-not-granted
             (get-in result [:kotoba.cli/data :kotoba.runtime/result
                             :kotoba.runtime/problems 0 :kotoba.runtime/problem])))
      (is (not (contains? (:kotoba.cli/data result) :kotoba.host/receipts))))))

(deftest run-reports-unreadable-policy
  (let [result (launcher/dispatch ["run" "src/demo_i64_host.kotoba"
                                   "--policy" "missing-policy.edn" "--json"])]
    (is (false? (:kotoba.cli/ok? result)))
    (is (= :run/policy-not-readable (:kotoba.cli/code result)))
    (is (= "missing-policy.edn"
           (get-in result [:kotoba.cli/data :kotoba.policy/path])))))

(deftest runtime-run-legacy-arity-is-unchanged
  (let [forms (runtime/read-file "src/demo.kotoba" :kotoba)
        plan (launcher/source-plan "src/demo.kotoba")
        ran (runtime/run (launcher/safe-analyzer-fact-classification) plan forms)]
    (is (true? (:kotoba.runtime/ok? ran)))
    (is (= 42 (:kotoba.runtime/value ran)))))
