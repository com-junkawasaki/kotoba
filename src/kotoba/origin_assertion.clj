(ns kotoba.origin-assertion
  "Origin-side verification for short-lived edge identity assertions."
  (:require [clojure.edn :as edn]
            [ed25519.core :as ed])
  (:import [java.security MessageDigest]
           [java.util Base64]))

(def policy-path "qualification/origin-edge-assertion.edn")

(defn read-policy []
  (edn/read-string (slurp policy-path)))

(defn sha256 [^bytes bytes]
  (format "%064x" (BigInteger. 1 (.digest (MessageDigest/getInstance "SHA-256")
                                          bytes))))

(defn canonical-body [assertion]
  (.getBytes (pr-str (into (sorted-map) (dissoc assertion :signature)))
             "UTF-8"))

(defn sign [seed claims]
  (assoc claims :signature
         (.encodeToString (Base64/getEncoder)
                          (ed/sign seed (canonical-body claims)))))

(defn new-replay-store []
  (atom #{}))

(defn- reserve-once! [store replay-key]
  (loop []
    (let [seen @store]
      (cond
        (contains? seen replay-key) false
        (compare-and-set! store seen (conj seen replay-key)) true
        :else (recur)))))

(defn verify!
  [policy trust replay-store
   {:keys [audience now-ms method path body] :as request}
   assertion]
  (let [required (:required-claims policy)
        missing (remove #(contains? assertion %) required)
        issuer (:issuer assertion)
        issued (:issued-at-ms assertion)
        expires (:expires-at-ms assertion)
        lifetime (when (and (integer? issued) (integer? expires))
                   (- expires issued))
        skew (:clock-skew-ms policy)
        signature (try
                    (.decode (Base64/getDecoder) ^String (:signature assertion))
                    (catch Exception _ nil))
        signature-valid?
        (and signature
             (try (ed/verify-did issuer (canonical-body assertion) signature)
                  (catch Exception _ false)))
        code
        (cond
          (seq missing) :origin/missing-claim
          (not= 1 (:version assertion)) :origin/version
          (not= :active (get-in trust [issuer :status])) :origin/untrusted-issuer
          (not signature-valid?) :origin/invalid-signature
          (not= audience (:audience assertion)) :origin/audience
          (not= method (:method assertion)) :origin/method
          (not= path (:path assertion)) :origin/path
          (not= (sha256 body) (:body-sha256 assertion)) :origin/body
          (or (nil? lifetime) (not (pos? lifetime))
              (> lifetime (:maximum-lifetime-ms policy))) :origin/lifetime
          (< now-ms (- issued skew)) :origin/not-yet-valid
          (>= now-ms (+ expires skew)) :origin/expired
          :else nil)
        replay-key [issuer (:nonce assertion)]]
    (if code
      {:valid? false :code code}
      (if (reserve-once! replay-store replay-key)
        {:valid? true :code nil :subject (:subject assertion)
         :issuer issuer :audience audience}
        {:valid? false :code :origin/replay}))))
