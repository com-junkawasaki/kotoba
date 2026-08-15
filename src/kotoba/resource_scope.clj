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
