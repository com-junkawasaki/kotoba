(ns kotoba.codebase-ipld
  "IPLD publication bridge for the local semantic codebase.

  A published root is immutable: it links to the selected namespace commit and
  to CID-bound executable-source witnesses. Peers need only this root CID to
  hydrate the complete runnable closure through `kotoba.ipld-block-store`."
  (:require [cbor.core :as cbor]
            [kotoba.ipld-block-store :as blocks]
            [kotoba.semantic-code :as semantic]
            [kotoba.semantic-codebase :as codebase]
            [multiformats.core :as mf]))

(def head-schema "kotoba.codebase-ipld-head.v2")
(def block-schema "kotoba.codebase-semantic-block.v1")
(def source-schema "kotoba.codebase-executable-source.v1")

(defn publish-namespace!
  "Publish one selected namespace as an immutable IPLD root and return its CID.
  The caller distributes/authorizes this root CID separately (for example in a
  signed head record); block availability is verified by the receiving peer."
  [codebase-root block-root namespace]
  (let [head (codebase/head codebase-root namespace)]
    (when-not head
      (throw (ex-info "namespace has no selected head" {:problem :codebase/head-not-found
                                                          :namespace namespace})))
    (let [closure (codebase/export-closure codebase-root [head])]
      (when (seq (:missing closure))
        (throw (ex-info "cannot publish incomplete codebase closure"
                        {:problem :codebase/ipld-missing-blocks :missing (:missing closure)})))
      (let [semantic-blocks
            (into (sorted-map)
                  (map (fn [{:keys [cid bytes]}]
                         [cid (semantic/cid-link
                               (blocks/put-node! block-root
                                                 {"schema" block-schema
                                                  "cid" cid
                                                  "bytes" bytes}))]))
                  (:blocks closure))
            witnesses (into (sorted-map)
                            (map (fn [{:keys [cid source]}]
                                   [cid (semantic/cid-link
                                         (blocks/put-node! block-root
                                                           {"schema" source-schema
                                                            "definition" cid
                                                            "source" source}))]))
                            (:executable-sources closure))]
        (blocks/put-node! block-root {"schema" head-schema
                                      "namespace" namespace
                                      "commit" head
                                      "blocks" semantic-blocks
                                      "witnesses" witnesses})))))

(defn- link-cid [link]
  (let [raw (or (:value link) (:v link)) xs (seq raw)]
    (when-not (and (= 42 (:n link)) (= 0 (bit-and 0xff (int (first xs)))))
      (throw (ex-info "invalid IPLD link" {:problem :codebase/ipld-invalid-link})))
    (str "b" (mf/base32 (rest xs)))))

(defn hydrate-namespace!
  "Fetch HEAD-CID from PEER, verify every IPLD block, import its semantic
  closure/source witnesses, then advance the local namespace only via CAS and
  AUTHORIZE!."
  ([codebase-root block-root peer-url head-cid expected-head authorize!]
   (hydrate-namespace! codebase-root block-root peer-url head-cid expected-head authorize! nil))
  ([codebase-root block-root peer-url head-cid expected-head authorize! expected-namespace]
  (let [{:keys [blocks]} (blocks/fetch-closure! block-root peer-url head-cid)
        decoded (into {} (map (fn [cid] [cid (cbor/decode (blocks/get-block block-root cid))]) blocks))
        head (get decoded head-cid)]
    (when-not (= head-schema (get head "schema"))
      (throw (ex-info "IPLD root is not a codebase head" {:problem :codebase/ipld-invalid-head})))
    (let [namespace (get head "namespace")
          commit (get head "commit")
          semantic-blocks
          (for [[semantic-cid wrapper-link] (get head "blocks")
                :let [wrapper (get decoded (link-cid wrapper-link))
                      bytes (get wrapper "bytes")]]
            (do
              (when-not (and (= block-schema (get wrapper "schema"))
                             (= semantic-cid (get wrapper "cid"))
                             (= semantic-cid (mf/cidv1-dag-cbor bytes)))
                (throw (ex-info "invalid semantic block wrapper"
                                {:problem :codebase/ipld-invalid-block
                                 :cid semantic-cid})))
              {:cid semantic-cid :bytes bytes}))
          witnesses
          (mapv (fn [[definition-cid witness-link]]
                  (let [witness (get decoded (link-cid witness-link))]
                    (when-not (and (= source-schema (get witness "schema"))
                                   (= definition-cid (get witness "definition"))
                                   (string? (get witness "source")))
                      (throw (ex-info "invalid executable source witness"
                                      {:problem :codebase/ipld-invalid-witness
                                       :cid definition-cid})))
                    {:cid definition-cid :source (get witness "source")}))
                (get head "witnesses"))]
      (when (and expected-namespace (not= expected-namespace namespace))
        (throw (ex-info "actor namespace does not match IPLD manifest"
                        {:problem :codebase/ipld-namespace-mismatch
                         :expected expected-namespace :actual namespace})))
      (codebase/import-closure! codebase-root {:blocks semantic-blocks
                                                :executable-sources (vec witnesses)})
      {:namespace namespace :head commit
       :publication (codebase/publish-head! codebase-root namespace commit expected-head authorize!)}))))
