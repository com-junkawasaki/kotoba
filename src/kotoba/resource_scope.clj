(ns kotoba.resource-scope
  "Structural resource-scope matching shared by static and runtime gates."
  (:require [clojure.string :as str])
  (:import (java.net URI)))

(defn- effective-port [^URI uri]
  (let [port (.getPort uri)]
    (if (neg? port)
      (case (some-> (.getScheme uri) str/lower-case)
        "http" 80
        "https" 443
        -1)
      port)))

(defn- normalized-path [^URI uri]
  (let [path (or (.getPath (.normalize uri)) "/")]
    (if (empty? path) "/" path)))

(defn http-scope-covers?
  "True when GRANT and RESOURCE are HTTP(S) URLs with the same canonical
  origin and RESOURCE is at GRANT's path or below it. Userinfo and fragments
  are rejected so authority cannot be hidden in presentation syntax."
  [grant resource]
  (try
    (let [^URI g (URI/create grant)
          ^URI r (URI/create resource)
          gs (some-> (.getScheme g) str/lower-case)
          rs (some-> (.getScheme r) str/lower-case)
          gp (normalized-path g)
          rp (normalized-path r)]
      (boolean
       (and (contains? #{"http" "https"} gs)
            (= gs rs)
            (nil? (.getUserInfo g)) (nil? (.getUserInfo r))
            (nil? (.getFragment g)) (nil? (.getFragment r))
            (nil? (.getQuery g))
            (some? (.getHost g)) (some? (.getHost r))
            (= (str/lower-case (.getHost g))
               (str/lower-case (.getHost r)))
            (= (effective-port g) (effective-port r))
            (or (= gp rp)
                (if (str/ends-with? gp "/")
                  (str/starts-with? rp gp)
                  (str/starts-with? rp (str gp "/")))))))
    (catch Exception _ false)))

(defn covers?
  "Exact match for ordinary resources; structural origin/path containment for
  HTTP(S). Other URI schemes do not gain prefix semantics."
  [grant resource]
  (and (string? grant)
       (string? resource)
       (or (= grant resource)
           (http-scope-covers? grant resource))))

(defn parts
  "Structural view of S for a decision this namespace no longer has to make.

  Parsing is mechanism; what a grant covers is the decision, and that moved
  to `cores/resource_scope_core.cljk` (Q9 wave 1). The core takes these
  fields and nothing else, so it never has to reimplement java.net.URI.

  `:scheme \"\"` means S did not parse at all, and `:host \"\"` that the
  authority was not server-based. Both fail closed in the core through the
  ordinary path rather than through a sentinel every caller has to test for.
  `:port -1` is URI.getPort's own encoding of an absent port."
  [s]
  (if-not (string? s)
    {:scheme "" :host "" :port -1 :path "/"
     :userinfo? false :query? false :fragment? false}
    (try
      (let [^URI u (URI/create s)]
        {:scheme (or (.getScheme u) "")
         :host (or (.getHost u) "")
         :port (.getPort u)
         :path (normalized-path u)
         :userinfo? (some? (.getUserInfo u))
         :query? (some? (.getQuery u))
         :fragment? (some? (.getFragment u))})
      (catch Exception _
        {:scheme "" :host "" :port -1 :path "/"
         :userinfo? false :query? false :fragment? false}))))
