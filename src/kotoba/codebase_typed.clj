(ns kotoba.codebase-typed
  "Source in, KIR-derived definitions out.

  This is the join the workspace was missing. `kotoba-lang/codebase` owns
  content-addressed identity and refuses to parse source; `kotoba-lang/compiler`
  owns checking and lowering and knows nothing about a codebase. Only here are
  both on the classpath, so only here can a `.kotoba` file become definitions
  whose identity IS the checked KIR the backends consume.

  Nothing about the compiler is reimplemented: the source goes through the same
  `check-source` admission and the same `lower` that a `kotoba compile` runs, and
  the resulting KIR is handed to the codebase whole."
  (:require [kotoba.codebase.authoring :as authoring]
            [kotoba.codebase.typed-code :as typed]
            [kotoba.codebase.typed-eval :as typed-eval]
            [kotoba.compiler.core :as compiler]
            [kotoba.kir :as kir]))

(defn source->kir
  "Check and lower SOURCE, returning the KIR a definition identity is taken from.

  No backend is invoked: lowering is where checking ends and target selection
  begins, and a definition's identity must not depend on which target someone
  happened to ask for."
  ([source] (source->kir source {}))
  ([source policy]
   (let [{:keys [hir]} (compiler/check-source source policy)]
     (kir/lower hir))))

(defn plan
  "Plan a namespace update whose definitions are hashed from checked KIR."
  ([root namespace source] (plan root namespace source {}))
  ([root namespace source policy]
   (let [module (source->kir source policy)]
     (authoring/plan-with root namespace
                          #(typed/compile-module module {:definitions %})))))

(defn update-namespace!
  "Plan and commit in one step."
  ([root namespace source] (update-namespace! root namespace source {}))
  ([root namespace source policy]
   (authoring/commit! root (plan root namespace source policy))))

(defn typed-block?
  "Whether a stored block is a KIR-derived definition."
  [block]
  (contains? #{typed/schema typed/member-schema} (get block "schema")))

(defn invoke
  "Execute a KIR-derived definition through the language oracle."
  ([root cid args] (typed-eval/invoke root cid args {}))
  ([root cid args opts] (typed-eval/invoke root cid args opts)))
