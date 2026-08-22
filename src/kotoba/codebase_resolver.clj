(ns kotoba.codebase-resolver
  "Fail-closed actor → IPLD → runnable codebase resolver."
  (:require [kotoba.codebase-actor-store :as actor-store]
            [kotoba.codebase-ipld :as codebase-ipld]))

(defn sync!
  "Resolve an actor-controlled namespace from untrusted peers.

  The latest actor record must first pass durable signature/sequence/CAS
  admission. Its signed IPLD head CID then selects the exact codebase manifest;
  the manifest namespace must equal the actor namespace before the local
  semantic head can advance."
  [{:keys [actor-store-root actor-peer-url actor-id namespace
           codebase-root block-root block-peer-url expected-head]}]
  (let [accepted (actor-store/fetch-latest! actor-store-root actor-peer-url
                                             actor-id namespace)
        statement (:statement accepted)
        head-cid (:head-cid statement)]
    (codebase-ipld/hydrate-namespace!
     codebase-root block-root block-peer-url head-cid expected-head
     (fn [{requested-namespace :namespace}]
       (= namespace requested-namespace))
     namespace)))

(defn sync-and-run!
  "Complete the verified path and run TARGET only after `sync!` succeeds.
  EXTRA-ARGV is forwarded to the codebase runner."
  [{:keys [codebase-root namespace target extra-argv] :as request}]
  (let [synced (sync! request)
        dispatch (requiring-resolve 'kotoba.launcher/dispatch)
        argv (into ["codebase" "run" target "--store" (str codebase-root)
                    "--namespace" namespace]
                   extra-argv)
        result (dispatch argv)]
    {:sync synced :run result}))
