(ns kotoba.origin-assertion-test
  (:require [clojure.test :refer [deftest is]]
            [ed25519.core :as ed]
            [kotoba.origin-assertion :as assertion]))

(def seed (byte-array (range 32)))
(def issuer (ed/did-key-from-seed seed))
(def policy (assertion/read-policy))
(def request {:audience "origin.kotoba.internal"
              :now-ms 1000000
              :method "POST" :path "/xrpc/kotoba.write"
              :body (.getBytes "payload" "UTF-8")})

(defn valid-assertion []
  (assertion/sign
   seed {:version 1 :issuer issuer :subject "did:key:user"
         :audience (:audience request)
         :method (:method request) :path (:path request)
         :body-sha256 (assertion/sha256 (:body request))
         :nonce "edge-unique-1"
         :issued-at-ms 990000 :expires-at-ms 1020000}))

(deftest origin-verifies-signature-and-all-request-bindings
  (let [result (assertion/verify!
                policy {issuer {:status :active}}
                (assertion/new-replay-store) request (valid-assertion))]
    (is (:valid? result))
    (is (= "did:key:user" (:subject result)))))

(deftest assertion-is-single-use-under-concurrent-origin-verification
  (let [store (assertion/new-replay-store)
        token (valid-assertion)
        attempts (doall
                  (map deref
                       (repeatedly
                        16
                        #(future (assertion/verify!
                                  policy {issuer {:status :active}}
                                  store request token)))))]
    (is (= 1 (count (filter :valid? attempts))))
    (is (= 15 (count (filter #(= :origin/replay (:code %)) attempts))))))

(deftest origin-rejects-tampering-expiry-wrong-audience-and-untrusted-edge
  (doseq [[token req trust expected]
          [[(assoc (valid-assertion) :subject "did:key:attacker")
            request {issuer {:status :active}} :origin/invalid-signature]
           [(valid-assertion)
            (assoc request :audience "other-origin")
            {issuer {:status :active}} :origin/audience]
           [(valid-assertion)
            (assoc request :body (.getBytes "changed" "UTF-8"))
            {issuer {:status :active}} :origin/body]
           [(valid-assertion)
            (assoc request :now-ms 2000000)
            {issuer {:status :active}} :origin/expired]
           [(valid-assertion)
            request {issuer {:status :revoked}} :origin/untrusted-issuer]]]
    (is (= expected
           (:code (assertion/verify!
                   policy trust (assertion/new-replay-store) req token))))))

(deftest arbitrary-edge-headers-never-authorize
  (is (= :origin/missing-claim
         (:code (assertion/verify!
                 policy {issuer {:status :active}}
                 (assertion/new-replay-store) request
                 {:subject "admin" :x-forwarded-user "admin"})))))
