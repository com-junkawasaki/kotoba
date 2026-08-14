(ns kotoba.graph-adapter-test
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is]]
            [kotoba.cli :as cli]
            [kotoba.graph-adapter :as graph-adapter]
            [kotoba.launcher :as launcher])
  (:import [java.nio.file Files]
           [java.nio.file.attribute FileAttribute]))

(defn- fake-host
  "In-memory IGraphHost. `stores` is an atom of handle-key → store.
   `files` is a map of path → EDN string."
  [stores files]
  (let [handle-key (fn [handle]
                     (if (= :mem (:scheme handle))
                       [:mem (:alias handle)]
                       [:file (:path handle)]))]
    (reify graph-adapter/IGraphHost
      (-load [_ handle] (get @stores (handle-key handle)))
      (-save [_ handle store]
        (swap! stores assoc (handle-key handle) store)
        store)
      (-read-text [_ path] (get files path)))))

(defn- planned [argv-tail]
  {:kotoba.cli/ok? true
   :kotoba.cli/code :command/planned
   :kotoba.cli/data {:command :graph
                     :request (cli/parse-argv argv-tail)
                     :host-action :adapter-required}})

(deftest parse-handle-shapes
  (is (= {:scheme :mem :alias "local"} (graph-adapter/parse-handle "mem:local")))
  (is (= {:scheme :file :path "/tmp/db.edn"} (graph-adapter/parse-handle "file:/tmp/db.edn")))
  (is (= {:scheme :file :path "data/db.edn"} (graph-adapter/parse-handle "data/db.edn")))
  (is (= :graph/missing-handle (:error (graph-adapter/parse-handle "mem:"))))
  (is (= :graph/missing-handle (:error (graph-adapter/parse-handle "")))))

(deftest plan-rejects-bad-requests
  (is (= :graph/missing-operation (:error (graph-adapter/plan {:positionals [] :options {}}))))
  (is (= :graph/unknown-operation (:error (graph-adapter/plan {:positionals ["index"] :options {:graph "mem:x"}}))))
  (is (= :graph/missing-handle (:error (graph-adapter/plan {:positionals ["status"] :options {}}))))
  (is (= :graph/missing-file (:error (graph-adapter/plan {:positionals ["query"]
                                                         :options {:graph "mem:x"}})))))

(deftest plan-uses-op-option-when-no-subcommand
  (is (= :status (:operation (graph-adapter/plan {:positionals []
                                                 :options {:op "status" :graph "mem:x"}})))))

(deftest plan-accepts-db-handle-alias
  (is (= :status (:operation (graph-adapter/plan {:positionals ["status"]
                                                 :options {:db "mem:x"}})))))

(deftest apply-tx-uses-canonical-datom-model
  (let [applied (graph-adapter/apply-tx graph-adapter/empty-store
                                        [{:db/id 1 :person/name "Ada"}
                                         [:db/add 1 :person/team 7]])]
    (is (= 2 (:added applied)))
    (is (= [[1 :person/name "Ada"] [1 :person/team 7]]
           (get-in applied [:store :datoms])))))

(deftest execute-connect-query-transact-pull-status
  (let [stores (atom {})
        host (fake-host stores {"tx.edn" "[{:db/id 1 :person/name \"Ada\" :person/team 7}]"
                                "q.edn" "{:find [?n] :where [[?e :person/name ?n] [?e :person/team 7]]}"})
        connected (graph-adapter/execute! host (planned ["connect" "--graph" "mem:demo"]))
        transacted (graph-adapter/execute! host (planned ["transact" "--graph" "mem:demo" "-f" "tx.edn"]))
        queried (graph-adapter/execute! host (planned ["query" "--graph" "mem:demo" "--file" "q.edn"]))
        pulled (graph-adapter/execute! host (planned ["pull" "--graph" "mem:demo" "--param" "1"]))
        status (graph-adapter/execute! host (planned ["status" "--graph" "mem:demo"]))]
    (is (= :graph/connected (:kotoba.cli/code connected)))
    (is (= :graph/transacted (:kotoba.cli/code transacted)))
    (is (= 2 (get-in transacted [:kotoba.cli/data :added])))
    (is (= [["Ada"]] (get-in queried [:kotoba.cli/data :result])))
    (is (= {:db/id 1 :person/name "Ada" :person/team 7}
           (get-in pulled [:kotoba.cli/data :result])))
    (is (= 2 (get-in status [:kotoba.cli/data :datom-count])))
    (is (= 1 (get-in status [:kotoba.cli/data :tx-count])))))

(deftest execute-query-substitutes-param-maps
  (let [stores (atom {})
        host (fake-host stores {"tx.edn" "[{:db/id 1 :person/name \"Ada\"} {:db/id 2 :person/name \"Bob\"}]"
                                "q.edn" "{:find [?n] :where [[?e :person/name ?n]]}"})
        _ (graph-adapter/execute! host (planned ["transact" "--graph" "mem:p" "-f" "tx.edn"]))
        queried (graph-adapter/execute!
                 host
                 (planned ["query" "--graph" "mem:p" "-f" "q.edn" "--param" "{?e 2}"]))]
    (is (= [["Bob"]] (get-in queried [:kotoba.cli/data :result])))))

(deftest launcher-executes-file-graph-end-to-end
  (let [dir (str (Files/createTempDirectory "kotoba-graph-adapter" (make-array FileAttribute 0)))
        db-path (str dir "/facts.edn")
        tx-path (str dir "/tx.edn")
        q-path (str dir "/q.edn")
        handle (str "file:" db-path)]
    (spit (io/file tx-path) "[{:db/id \"ada\" :person/name \"Ada\"}]")
    (spit (io/file q-path) "{:find [?n] :where [[?e :person/name ?n]]}")
    (let [connected (launcher/dispatch ["graph" "connect" "--graph" handle])
          transacted (launcher/dispatch ["graph" "transact" "--graph" handle "-f" tx-path])
          queried (launcher/dispatch ["graph" "query" "--graph" handle "-f" q-path])
          pulled (launcher/dispatch ["graph" "pull" "--graph" handle "--param" "\"ada\""])
          status (launcher/dispatch ["graph" "status" "--db" handle])]
      (is (= :graph/connected (:kotoba.cli/code connected)))
      (is (= :graph/transacted (:kotoba.cli/code transacted)))
      (is (= [["Ada"]] (get-in queried [:kotoba.cli/data :result])))
      (is (= "Ada" (get-in pulled [:kotoba.cli/data :result :person/name])))
      (is (= 1 (get-in status [:kotoba.cli/data :datom-count])))
      (is (.exists (io/file db-path))))))
