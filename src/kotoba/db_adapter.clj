(ns kotoba.db-adapter
  "Host adapter for the :db CLI command.

  The command shape is owned by kotoba-lang/kotoba-lang (`lang/cli.edn` +
  `kotoba.cli/dispatch`). This namespace consumes the `:command/planned`
  result for `db` and runs Datomic-shaped operations (connect / query /
  transact / pull / status) over the language's in-memory EAVT store
  (`kotoba.kgraph` + `datom.core`).

  Persistent kotobase is a separate database product and must not become a
  kotoba dependency (ADR-2607032500: kotobase → kotoba only). `--db` is a
  handle, not a vendor URI:

  - `mem:<alias>`  host-held atom
  - `file:<path>`  EDN datom log on disk
  - `<path>`       treated as `file:`

  Planning is pure; load/save and file reads happen only through an injected
  host port."
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            [clojure.walk :as walk]
            [datom.core :as dc]
            [kotoba.kgraph :as kgraph]))

(defprotocol IDbHost
  "Host-supplied db capabilities.

  `-load` / `-save` take a handle map `{:scheme :mem|:file ...}`.
  `-read-text` reads a filesystem path as a string, or nil if absent."
  (-load [this handle])
  (-save [this handle store])
  (-read-text [this path]))

(def operations #{:connect :query :transact :pull :status})

(def empty-store
  {:datoms [] :tx-count 0 :basis 0})

(defn request-operation
  "Resolve the db operation from a parsed CLI request: the first positional
  subcommand wins, then the --op option."
  [request]
  (let [raw (or (first (:positionals request))
                (get-in request [:options :op]))]
    (when raw
      (keyword (name (cond-> raw (string? raw) str/trim))))))

(defn request-file [request]
  (or (get-in request [:options :file])
      (get-in request [:options :f])))

(defn parse-handle
  "Parse `--db` into {:scheme :mem|:file :alias string :path string}."
  [s]
  (cond
    (or (nil? s) (and (string? s) (str/blank? s)))
    {:error :db/missing-handle}

    (and (string? s) (str/starts-with? s "mem:"))
    (let [alias (subs s 4)]
      (if (str/blank? alias)
        {:error :db/missing-handle :scheme :mem}
        {:scheme :mem :alias alias}))

    (and (string? s) (str/starts-with? s "file:"))
    {:scheme :file :path (subs s 5)}

    (string? s)
    {:scheme :file :path s}

    :else
    {:error :db/missing-handle :value s}))

(defn request-handle [request]
  (parse-handle (get-in request [:options :db])))

(defn parse-edn-value
  "Read one CLI/file EDN string. nil when blank."
  [s]
  (when (and (string? s) (not (str/blank? s)))
    (edn/read-string s)))

(defn request-params
  "EDN values from every `--param`. A single value is wrapped as a vector."
  [request]
  (let [raw (get-in request [:options :param])]
    (->> (cond (nil? raw) []
               (vector? raw) raw
               :else [raw])
         (keep parse-edn-value)
         vec)))

(defn plan
  "Pure plan: validate a parsed :db request. Returns {:operation :handle ...}
  or {:error ...}."
  [request]
  (let [op (request-operation request)
        handle (request-handle request)
        file (request-file request)]
    (cond
      (nil? op)
      {:error :db/missing-operation
       :expected (sort operations)}

      (not (operations op))
      {:error :db/unknown-operation
       :operation op
       :expected (sort operations)}

      (:error handle)
      (assoc handle :operation op)

      (and (#{:query :transact} op) (str/blank? (str file)))
      {:error :db/missing-file
       :operation op
       :expected "-f/--file"}

      :else
      {:operation op
       :handle handle
       :file file
       :params (request-params request)})))

(defn- item-datoms [item]
  (cond
    (map? item)
    (if (some #(= "id" (name %)) (keys item))
      (dc/eavt item)
      :db/missing-entity-id)

    (and (vector? item) (= :db/add (first item)) (= 4 (count item)))
    [(subvec item 1)]

    (and (vector? item) (= 3 (count item)))
    [item]

    :else
    :db/bad-tx-item))

(defn normalize-tx
  "Accept a tx vector, a single entity map, or `{:tx-data [...]}`."
  [input]
  (cond
    (and (map? input) (contains? input :tx-data)) (:tx-data input)
    (vector? input) input
    (map? input) [input]
    :else nil))

(defn apply-tx
  "Append tx-data onto a store. Returns {:store ... :added n} or {:error ...}."
  [store tx-data]
  (let [items (normalize-tx tx-data)]
    (if (nil? items)
      {:error :db/bad-tx}
      (let [chunks (map item-datoms items)
            bad (first (filter keyword? chunks))]
        (if bad
          {:error bad}
          (let [added (vec (mapcat identity chunks))
                datoms (into (vec (:datoms store)) added)]
            {:store (assoc store
                           :datoms datoms
                           :tx-count (inc (or (:tx-count store) 0))
                           :basis (inc (or (:basis store) 0))
                           :last-tx items)
             :added (count added)}))))))

(defn substitute-params
  "Replace logic vars in a query with `--param` map bindings."
  [query params]
  (let [mapping (into {} (filter map? params))]
    (if (empty? mapping)
      query
      (walk/postwalk (fn [form]
                       (if (contains? mapping form)
                         (get mapping form)
                         form))
                     query))))

(defn pull-entity
  "Reconstruct a Datomic-style entity map from `[e a v]` datoms."
  [datoms e]
  (let [found (kgraph/get-objects datoms e)]
    (when (seq found)
      (into {:db/id e}
            (map (fn [[_ a v]] [a v]) found)))))

(defn store-status [store handle]
  {:handle handle
   :datom-count (count (:datoms store))
   :tx-count (or (:tx-count store) 0)
   :basis (or (:basis store) 0)})

(defn- load-or-empty [host handle]
  (or (-load host handle) empty-store))

(defn- fail [code message data]
  {:kotoba.cli/ok? false
   :kotoba.cli/code code
   :kotoba.cli/message message
   :kotoba.cli/data data})

(defn- ok [code data]
  {:kotoba.cli/ok? true
   :kotoba.cli/code code
   :kotoba.cli/data data})

(defn- read-edn-file [host path]
  (let [text (-read-text host path)]
    (if (nil? text)
      {:error :db/missing-input :path path}
      (try
        {:value (edn/read-string text)}
        (catch Exception e
          {:error :db/bad-edn
           :path path
           :message (ex-message e)})))))

(defn execute!
  "Execute a `:command/planned` result for :db through the injected host
  port. Returns a kotoba.cli-shaped result map."
  [host planned-result]
  (let [request (get-in planned-result [:kotoba.cli/data :request])
        planned (plan request)]
    (if (:error planned)
      (fail (:error planned)
            "db adapter could not plan the request"
            (assoc (dissoc planned :error) :request request))
      (let [{:keys [operation handle file params]} planned
            store (load-or-empty host handle)]
        (case operation
          :connect
          (do (-save host handle store)
              (ok :db/connected (store-status store handle)))

          :status
          (ok :db/status (store-status store handle))

          :transact
          (let [parsed (read-edn-file host file)]
            (if (:error parsed)
              (fail (:error parsed) "db transact could not read tx data" parsed)
              (let [applied (apply-tx store (:value parsed))]
                (if (:error applied)
                  (fail (:error applied) "db transact rejected tx data"
                        (assoc planned :tx (:value parsed)))
                  (do (-save host handle (:store applied))
                      (ok :db/transacted
                          (merge (store-status (:store applied) handle)
                                 {:added (:added applied)})))))))

          :query
          (let [parsed (read-edn-file host file)]
            (if (:error parsed)
              (fail (:error parsed) "db query could not read query data" parsed)
              (let [query (substitute-params (:value parsed) params)
                    rows (kgraph/query (:datoms store) query)]
                (ok :db/queried
                    (merge (store-status store handle)
                           {:query query :result rows})))))

          :pull
          (let [entity-arg (fn [x]
                             (cond
                               (map? x) (or (:db/id x) (:entity x))
                               :else x))
                entity-id (or (entity-arg (first params))
                              (let [parsed (when file (read-edn-file host file))]
                                (cond
                                  (nil? file) nil
                                  (:error parsed) parsed
                                  :else (entity-arg (:value parsed)))))]
            (if (and (map? entity-id) (:error entity-id))
              (fail (:error entity-id) "db pull could not read entity" entity-id)
              (if (nil? entity-id)
                (fail :db/missing-entity
                      "db pull needs --param <entity> or -f with an entity id"
                      planned)
                (ok :db/pulled
                    (merge (store-status store handle)
                           {:entity entity-id
                            :result (pull-entity (:datoms store) entity-id)}))))))))))
