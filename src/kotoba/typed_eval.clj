(ns kotoba.typed-eval
  "The host provider behind Kotoba's `(eval request)` surface.

  `eval` never receives source text. The request is a bounded `:document`
  naming a checked definition CID and carrying argument documents. Admission
  rechecks the target's exact interface, complete effect row, fuel, and nested
  eval depth before KIR execution."
  (:require [kotoba.codebase.typed-eval :as codebase-eval]
            [kotoba.compiler.value-codec :as value-codec]
            [kotoba.kir.value :as value]))

(def capability-id 30)
(def default-fuel codebase-eval/default-fuel)
(def default-max-depth codebase-eval/default-eval-depth)

(defn- fail! [problem data]
  (throw (ex-info (name problem) (assoc data :problem problem))))

(defn- document-key [[tag payload :as node]]
  (case tag
    "keyword" payload
    "string" payload
    (fail! :typed-eval/request-key-invalid {:node node})))

(defn- document-map [document]
  (let [[tag entries :as document] (value/bounded-document! document)]
    (when-not (= "map" tag)
      (fail! :typed-eval/request-invalid {:expected :document-map
                                          :document document}))
    (into {} (map (fn [[key item]] [(document-key key) item])) entries)))

(defn- field [request key]
  (or (get request key)
      (get request (name key))))

(defn- required-field [request key]
  (or (field request key)
      (fail! :typed-eval/request-field-required {:field key})))

(defn- string-node [[tag payload :as node] field-name]
  (when-not (= "string" tag)
    (fail! :typed-eval/request-field-invalid {:field field-name :node node}))
  payload)

(defn- vector-node [[tag payload :as node] field-name]
  (when-not (= "vector" tag)
    (fail! :typed-eval/request-field-invalid {:field field-name :node node}))
  payload)

(defn- positive-i64-node [[tag payload :as node] field-name maximum]
  (when-not (and (= "i64" tag) (integer? payload)
                 (pos? payload) (<= payload maximum))
    (fail! :typed-eval/request-bound-invalid
           {:field field-name :maximum maximum :node node}))
  (long payload))

(defn- decode-request [document {:keys [fuel max-depth]}]
  (let [request (document-map document)
        requested-fuel (if-let [node (field request :fuel)]
                         (positive-i64-node node :fuel fuel)
                         fuel)
        requested-depth (if-let [node (field request :max-depth)]
                          (positive-i64-node node :max-depth max-depth)
                          max-depth)]
    {:cid (string-node (required-field request :definition-cid) :definition-cid)
     :argument-documents (vector-node (required-field request :arguments) :arguments)
     :fuel requested-fuel
     :max-depth requested-depth}))

(declare invoke-request)

(defn- nested-dispatch
  [root {:keys [allowed-effects typed-cap-call receipt-sink] :as options} depth fuel]
  (fn [id request-type result-type request]
    (if (= capability-id id)
      (do
        (when (<= depth 1)
          (fail! :typed-eval/depth-exhausted {:max-depth depth}))
        (when-not (= :document request-type)
          (fail! :typed-eval/request-type-mismatch {:actual request-type}))
        (invoke-request root request result-type
                        (assoc options :max-depth (dec depth) :fuel fuel)))
      (if typed-cap-call
        (typed-cap-call id request-type result-type request)
        (fail! :typed-eval/capability-provider-not-installed
               {:capability id :allowed-effects allowed-effects})))))

(defn invoke-request
  "Evaluate one checked definition selected by a bounded document request.

  OPTIONS is host authority, not guest input: `:allowed-effects` can only be
  narrowed, and nested eval receives a strictly smaller depth. `receipt-sink`
  observes DefCID/AdmissionCID/ValueCID evidence without granting anything."
  ([root request result-type] (invoke-request root request result-type {}))
  ([root request result-type
    {:keys [allowed-effects fuel max-depth receipt-sink] :as options
     :or {allowed-effects #{}
          fuel default-fuel
          max-depth default-max-depth}}]
   (let [{:keys [cid argument-documents fuel max-depth]}
         (decode-request request {:fuel fuel :max-depth max-depth})
         admission (codebase-eval/admit
                    root cid {:allowed-effects allowed-effects
                              :expected-result result-type
                              :fuel fuel
                              :max-depth max-depth})
         {:keys [param-types schemas]} (:interface admission)
         _ (when-not (= (count param-types) (count argument-documents))
             (fail! :typed-eval/arity-mismatch
                    {:expected (count param-types)
                     :actual (count argument-documents)}))
         args (mapv (fn [descriptor document]
                      (value-codec/wire-value->runtime
                       descriptor schemas
                       (value-codec/runtime->wire-value :document document)))
                    param-types argument-documents)
         result (codebase-eval/invoke-admitted
                 root admission args
                 {:typed-cap-call (nested-dispatch root options max-depth fuel)
                  :receipt-sink receipt-sink})]
     (:value result))))

(defn provider
  "Build the exact provider map for one contextual `(eval request)` result type."
  ([root result-type] (provider root result-type {}))
  ([root result-type options]
   {:request-type :document
    :result-type result-type
    :invoke #(invoke-request root % result-type options)}))
