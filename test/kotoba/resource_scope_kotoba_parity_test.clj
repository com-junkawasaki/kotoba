(ns kotoba.resource-scope-kotoba-parity-test
  "Q9 wave 1 parity: kotoba.resource-scope (retained JVM oracle) against
  cores/resource_scope_core.cljk (the decision, compiled by the Kotoba
  compiler and executed as KIR).

  The oracle is untouched -- it still parses with java.net.URI and decides
  in Clojure. The core decides from `resource-scope/parts`. Any place where
  the record encoding loses something the oracle can see shows up here as a
  corpus failure rather than as a silent behavioural change.

  Corpus-bounded by construction: this proves agreement on the pairs below,
  not over all strings. The boundary cases that motivated the port -- a
  grant covering a longer, different name -- are pinned individually."
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.compiler.core :as compiler]
            [kotoba.kir :as ir]
            [kotoba.resource-scope :as scope]))

(def ^:private core-source (slurp "cores/resource_scope_core.cljk"))

(def ^:private url-ty
  (str "[:record :scope/url [[:scheme :string] [:host :string] [:port :i64] "
       "[:path :string] [:userinfo :bool] [:query :bool] [:fragment :bool]]]"))

(defn- kotoba-literal [s]
  (str \" (-> s (str/replace "\\" "\\\\") (str/replace "\"" "\\\"")) \"))

(defn- bool-literal [x] (if x "true" "false"))

(defn- url-form [s]
  (let [p (scope/parts s)]
    (str "(record-new " url-ty " "
         (kotoba-literal (:scheme p)) " "
         (kotoba-literal (:host p)) " "
         (long (:port p)) " "
         (kotoba-literal (:path p)) " "
         (bool-literal (:userinfo? p)) " "
         (bool-literal (:query? p)) " "
         (bool-literal (:fragment? p)) ")")))

(defn- compile-i64-cases
  "Append one nullary :i64 entry per case, compile the core once, and read
  every entry back out of the same KIR module."
  [cases]
  (let [defs (for [[name body] cases]
               (str "(defn " name " [] :i64 " body ")"))
        src (-> core-source
                (str/replace-first
                 #"\(:export \[[^\]]+\]\)"
                 (str "(:export [covers? covers-http? path-covers? exact? "
                      (str/join " " (map first cases)) "])"))
                (str "\n" (str/join "\n" defs)))
        kir (:kir (compiler/compile-source src :wasm32-kotoba-v1 {}))]
    (into {} (map (fn [[name _]] [name (ir/execute kir (symbol name) [])]) cases))))

;; ── the corpus ───────────────────────────────────────────────────────────

(def ^:private corpus
  [;; exact match, any scheme
   ["kotoba://graph/alice" "kotoba://graph/alice"]
   ["file:///etc/passwd" "file:///etc/passwd"]
   ["" ""]
   ;; the hazard this port exists to pin: a prefix that is not a boundary
   ["http://h/alice" "http://h/alice-evil"]
   ["http://h/alice" "http://h/alice/photos"]
   ["http://h/alice" "http://h/alice"]
   ["http://h/alice/" "http://h/alice/photos"]
   ["http://h/alice/" "http://h/alice"]
   ["kotoba://graph/alice" "kotoba://graph/alice-evil"]
   ;; root grants
   ["http://h/" "http://h/anything/at/all"]
   ["http://h" "http://h/anything"]
   ;; scheme
   ["https://h/a" "http://h/a/b"]
   ["http://h/a" "https://h/a/b"]
   ["ftp://h/a" "ftp://h/a/b"]
   ["HTTP://h/a" "http://h/a/b"]
   ;; host
   ["http://H.EXAMPLE/a" "http://h.example/a/b"]
   ["http://h/a" "http://other/a/b"]
   ["http:///a" "http:///a/b"]
   ;; port
   ["http://h/a" "http://h:80/a/b"]
   ["http://h:80/a" "http://h/a/b"]
   ["https://h/a" "https://h:443/a/b"]
   ["https://h:80/a" "https://h/a/b"]
   ["http://h:8080/a" "http://h:8080/a/b"]
   ["http://h:8080/a" "http://h:9090/a/b"]
   ;; presentation
   ["http://u@h/a" "http://h/a/b"]
   ["http://h/a" "http://u@h/a/b"]
   ["http://h/a#frag" "http://h/a/b"]
   ["http://h/a" "http://h/a/b#frag"]
   ["http://h/a?q=1" "http://h/a/b"]
   ["http://h/a" "http://h/a/b?q=1"]
   ;; dot segments, normalized by the host before the core sees them
   ["http://h/a/b" "http://h/a/./b/c"]
   ["http://h/a/b" "http://h/a/x/../b/c"]
   ["http://h/a/x/.." "http://h/a/b"]
   ;; unparseable input
   ["http://h/ a" "http://h/ a/b"]
   ["http://h/a" "http://h/{bad}"]
   ["not a uri" "not a uri/b"]
   ;; empty and degenerate
   ["" "http://h/a"]
   ["http://h/a" ""]])

(deftest covers-matches-the-jvm-oracle
  (let [cases (into {} (map-indexed
                        (fn [i [grant resource]]
                          [(str "cv_" i)
                           (str "(if (covers? "
                                (kotoba-literal grant) " "
                                (kotoba-literal resource) " "
                                (url-form grant) " "
                                (url-form resource) ") 1 0)")])
                        corpus))
        actual (compile-i64-cases cases)]
    (is (= (count corpus) (count actual))
        "every corpus pair produced a value")
    (doseq [[i [grant resource]] (map-indexed vector corpus)]
      (testing (pr-str [grant resource])
        (is (= (if (scope/covers? grant resource) 1 0)
               (get actual (str "cv_" i))))))))

(deftest path-containment-refuses-a-longer-different-name
  ;; The clause the port exists for, exercised without any URI around it.
  (let [cases {"p_same"     "(if (path-covers? \"/alice\" \"/alice\") 1 0)"
               "p_below"    "(if (path-covers? \"/alice\" \"/alice/x\") 1 0)"
               "p_longer"   "(if (path-covers? \"/alice\" \"/alice-evil\") 1 0)"
               "p_slash"    "(if (path-covers? \"/alice/\" \"/alice/x\") 1 0)"
               "p_slash_up" "(if (path-covers? \"/alice/\" \"/alice\") 1 0)"
               "p_root"     "(if (path-covers? \"/\" \"/anything\") 1 0)"
               "p_empty"    "(if (path-covers? \"\" \"/a\") 1 0)"}
        actual (compile-i64-cases cases)]
    (is (= 1 (get actual "p_same")))
    (is (= 1 (get actual "p_below")))
    (is (= 0 (get actual "p_longer"))
        "/alice must not cover /alice-evil")
    (is (= 1 (get actual "p_slash")))
    (is (= 0 (get actual "p_slash_up")))
    (is (= 1 (get actual "p_root")))
    (is (= 0 (get actual "p_empty"))
        "an empty grant path reaches nothing, not everything")))

(deftest the-host-never-hands-the-core-an-empty-path
  ;; `path-covers?` refuses an empty grant path outright. That is only a
  ;; narrowing rather than a divergence because the oracle cannot produce
  ;; one -- assert that here instead of trusting the reading.
  (doseq [s (distinct (mapcat identity corpus))]
    (testing (pr-str s)
      (is (seq (:path (scope/parts s)))
          "parts must normalize an absent path to \"/\", never \"\""))))

(deftest parts-encoding-is-lossless-for-the-corpus
  ;; The record uses "" for both `nil` host and empty host. Assert the oracle
  ;; never actually sees an empty-but-present host on this corpus, so the
  ;; encoding is not quietly conflating two different answers.
  (doseq [s (distinct (mapcat identity corpus))]
    (testing (pr-str s)
      (let [u (try (java.net.URI/create s) (catch Exception _ nil))]
        (is (not= "" (some-> u .getHost))
            "a present-but-empty host would collide with the absent encoding")))))
