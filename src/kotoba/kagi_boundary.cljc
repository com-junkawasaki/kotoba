(ns kotoba.kagi-boundary
  "Reference-only boundary to kagi/kagitaba. Kotoba facts and capabilities must
  never carry secret values. Resolution belongs to an injected kagi runtime adapter."
  (:require [clojure.string :as str]
            [kotoba.security.information-flow :as flow]))

(def allowed-schemes #{"kagi" "keychain" "pkcs11" "passkey"})

(defn secret-ref? [x]
  (and (string? x)
       (when-let [[_ scheme body] (re-matches #"^([a-z][a-z0-9+.-]*)://(.+)$" x)]
         (and (contains? allowed-schemes scheme) (not (str/blank? body))))))

(defn reference-record
  "Canonical Kotoba datom-safe pointer to a kagi item and kagitaba category."
  [{:keys [ref category purpose key-epoch]}]
  (when-not (and (secret-ref? ref) (keyword? category) (keyword? purpose)
                 (nat-int? key-epoch))
    (throw (ex-info "invalid kagi secret reference" {:ref-valid? (boolean (secret-ref? ref))
                                                       :category category :purpose purpose
                                                       :key-epoch key-epoch})))
  #:kotoba.secret{:ref ref :category category :purpose purpose :key-epoch key-epoch})

(def secret-value-keys
  "Keys whose presence means the record carries a secret itself, not a pointer
  to one."
  [:secret :value :plaintext :private-key :password :token])

(defn reference-classification
  "The record's classification on the shared lattice.

  A kagi *reference* is `:confidential` — it names where a secret lives, which
  helps an attacker but is not the secret. A record carrying one of
  `secret-value-keys` is `:restricted`, and `flow/join` makes that the whole
  record's classification: the output of a computation inherits the highest
  input classification. The decision this namespace already made by hand now
  goes through the control that owns it."
  [record]
  (flow/join (cons :confidential
                   (keep #(when (contains? record %) :restricted)
                         secret-value-keys))))

(defn assert-reference-only! [record]
  (let [classification (reference-classification record)]
    (when-not (= :confidential classification)
      (throw (ex-info "secret value crossed the kotoba/kagi boundary"
                      {:keys (vec (keys record))
                       :information-flow/classification classification}))))
  (when-not (secret-ref? (:kotoba.secret/ref record))
    (throw (ex-info "missing kagi secret reference" {})))
  record)
