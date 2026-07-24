(ns kotoba.capability-cross-layer-test
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is]]
            [kotoba.cap-table :as cap-table]
            [kotoba.lang.capability-values :as capability]
            [kotoba.lang.package-contract :as package-contract]
            [kotoba.runtime :as runtime]))

(defn intersection-input [requested delegated local]
  {:requested (capability/make-cap :host/http requested)
   :cacao-grants [{:grant/kind :host/http
                   :grant/resources delegated
                   :grant/id "cross-layer"}]
   :local-policy {:policy/allow {:host/http local}
                  :policy/forbid-wildcard true}
   :now "2026-07-23"})

(deftest acquisition-and-runtime-use-preserve-the-normative-intersection
  (doseq [requested ["https://a.example/x" "https://b.example/y" :any]
          delegated [#{"https://a.example/x"} #{"https://b.example/y"}
                     #{"https://a.example/x" "https://b.example/y"} #{:any}]
          local [#{"https://a.example/x"} #{"https://b.example/y"}
                 #{"https://a.example/x" "https://b.example/y"} #{:any}]]
    (let [input (intersection-input requested delegated local)
          normative (capability/intersect-grants input)
          table (cap-table/make-table)
          acquired (cap-table/acquire!
                    table {:kind :host/http :resource requested
                           :grants (:cacao-grants input)
                           :policy (:local-policy input)
                           :now (:now input)})]
      (if (capability/capability? normative)
        (let [handle (:kotoba.host/result acquired)
              stored (cap-table/resolve-cap table handle)
              used (cap-table/resolve-use table handle :host/http "2026-07-23")]
          (is (:kotoba.host/ok? acquired))
          (is (= normative stored))
          (is (= normative (:cap used))))
        (do
          (is (false? (:kotoba.host/ok? acquired)))
          (is (= (:denied normative) (:kotoba.host/denied acquired)))
          (is (empty? (:caps @table))))))))

(deftest compiler-effect-row-and-package-declaration-deny-undeclared-authority
  (let [body (list 'cap-acquire :host/http "https://a.example/x")
        http-fn (list 'defn (with-meta 'fetch {:effects #{:host/http}})
                      [] body)
        missing-row (list 'defn (with-meta 'fetch {:effects #{}})
                          [] body)]
    (is (empty? (runtime/cap-effect-problems [http-fn])))
    (is (= #{:host/http}
           (:kotoba.runtime/missing
            (first (runtime/cap-effect-problems [missing-row]))))))
  (let [lock (edn/read-string
              (slurp "test/fixtures/package/positive-lock.edn"))]
    (is (nil? (package-contract/lockfile-error
               lock {:declared-capabilities [:graph-read]})))
    (is (= "capability grant exceeds package declaration"
           (:message
            (package-contract/lockfile-error
             lock {:declared-capabilities []}))))))

(deftest production-wildcard-never-becomes-effective-authority
  (let [input (intersection-input :any #{:any} #{:any})
        result (capability/intersect-grants input)
        table (cap-table/make-table)
        acquired (cap-table/acquire!
                  table {:kind :host/http :resource :any
                         :grants (:cacao-grants input)
                         :policy (:local-policy input)
                         :now (:now input)})]
    (is (= :wildcard-forbidden (:denied result)))
    (is (= :wildcard-forbidden (:kotoba.host/denied acquired)))
    (is (empty? (:caps @table)))))
