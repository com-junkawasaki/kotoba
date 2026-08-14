(ns kotoba.db-adapter-test
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is]]
            [kotoba.cli :as cli]
            [kotoba.db-adapter :as db-adapter]
            [kotoba.launcher :as launcher])
  (:import [java.nio.file Files]
           [java.nio.file.attribute FileAttribute]))

(defn- fake-host
  "In-memory IDbHost. `stores` is an atom of handle-key → store.
   `files` is a map of path → EDN string."
  [stores files]
  (let [handle-key (fn [handle]
                     (if (= :mem (:scheme handle))
                       [:mem (:alias handle)]
                       [:file (:path handle)]))]
    (reify db-adapter/IDbHost
      (-load [_ handle] (get @stores (handle-key handle)))
      (-save [_ handle store]
        (swap! stores assoc (handle-key handle) store)
        store)
      (-read-text [_ path] (get files path)))))

(defn- planned [argv-tail]
  {:kotoba.cli/ok? true
   :kotoba.cli/code :command/planned
   :kotoba.cli/data {:command :db
                     :request (cli/parse-argv argv-tail)
                     :host-action :adapter-required}})

(deftest parse-handle-shapes
  (is (= {:scheme :mem :alias "local"} (db-adapter/parse-handle "mem:local")))
  (is (= {:scheme :file :path "/tmp/db.edn"} (db-adapter/parse-handle "file:/tmp/db.edn")))
  (is (= {:scheme :file :path "data/db.edn"} (db-adapter/parse-handle "data/db.edn")))
  (is (= :db/missing-handle (:error (db-adapter/parse-handle "mem:"))))
  (is (= :db/missing-handle (:error (db-adapter/parse-handle "")))))

(deftest plan-rejects-bad-requests
  (is (= :db/missing-operation (:error (db-adapter/plan {:positionals [] :options {}}))))
  (is (= :db/unknown-operation (:error (db-adapter/plan {:positionals ["index"] :options {:db "mem:x"}}))))
  (is (= :db/missing-handle (:error (db-adapter/plan {:positionals ["status"] :options {}}))))
  (is (= :db/missing-file (:error (db-adapter/plan {:positionals ["query"]
                                                   :options {:db "mem:x"}})))))

(deftest plan-uses-op-option-when-no-subcommand
  (is (= :status (:operation (db-adapter/plan {:positionals []
                                               :options {:op "status" :db "mem:x"}})))))

(deftest apply-tx-uses-canonical-datom-model
  (let [applied (db-adapter/apply-tx db-adapter/empty-store
                                     [{:db/id 1 :person/name "Ada"}
                                      [:db/add 1 :person/team 7]])]
    (is (= 2 (:added applied)))
    (is (= [[1 :person/name "Ada"] [1 :person/team 7]]
           (get-in applied [:store :datoms])))))

(deftest execute-connect-query-transact-pull-status
  (let [stores (atom {})
        host (fake-host stores {"tx.edn" "[{:db/id 1 :person/name \"Ada\" :person/team 7}]"
                                "q.edn" "{:find [?n] :where [[?e :person/name ?n] [?e :person/team 7]]}"})
        connected (db-adapter/execute! host (planned ["connect" "--db" "mem:demo"]))
        transacted (db-adapter/execute! host (planned ["transact" "--db" "mem:demo" "-f" "tx.edn"]))
        queried (db-adapter/execute! host (planned ["query" "--db" "mem:demo" "--file" "q.edn"]))
        pulled (db-adapter/execute! host (planned ["pull" "--db" "mem:demo" "--param" "1"]))
        status (db-adapter/execute! host (planned ["status" "--db" "mem:demo"]))]
    (is (= :db/connected (:kotoba.cli/code connected)))
    (is (= :db/transacted (:kotoba.cli/code transacted)))
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
        _ (db-adapter/execute! host (planned ["transact" "--db" "mem:p" "-f" "tx.edn"]))
        queried (db-adapter/execute!
                 host
                 (planned ["query" "--db" "mem:p" "-f" "q.edn" "--param" "{?e 2}"]))]
    (is (= [["Bob"]] (get-in queried [:kotoba.cli/data :result])))))

(deftest launcher-executes-file-db-end-to-end
  (let [dir (str (Files/createTempDirectory "kotoba-db-adapter" (make-array FileAttribute 0)))
        db-path (str dir "/facts.edn")
        tx-path (str dir "/tx.edn")
        q-path (str dir "/q.edn")
        handle (str "file:" db-path)]
    (spit (io/file tx-path) "[{:db/id \"ada\" :person/name \"Ada\"}]")
    (spit (io/file q-path) "{:find [?n] :where [[?e :person/name ?n]]}")
    (let [connected (launcher/dispatch ["db" "connect" "--db" handle])
          transacted (launcher/dispatch ["db" "transact" "--db" handle "-f" tx-path])
          queried (launcher/dispatch ["db" "query" "--db" handle "-f" q-path])
          pulled (launcher/dispatch ["db" "pull" "--db" handle "--param" "\"ada\""])
          status (launcher/dispatch ["db" "status" "--db" handle])]
      (is (= :db/connected (:kotoba.cli/code connected)))
      (is (= :db/transacted (:kotoba.cli/code transacted)))
      (is (= [["Ada"]] (get-in queried [:kotoba.cli/data :result])))
      (is (= "Ada" (get-in pulled [:kotoba.cli/data :result :person/name])))
      (is (= 1 (get-in status [:kotoba.cli/data :datom-count])))
      (is (.exists (io/file db-path))))))
