(ns kotoba.codebase-typed
  "Source in, KIR-derived definitions out.

  This is the join the workspace was missing. `kotoba-lang/codebase` owns
  content-addressed identity and refuses to parse source; `kotoba-lang/amu`
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
            [kotoba.compiler.module-lock :as module-lock]
            [kotoba.compiler.project :as project]
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

(defn plan-locked
  "Plan an update from a CID-PINNED module graph rather than a source file.

  This is where the two content-addressed halves meet. `module-lock` pins the
  compilation INPUTS by source CID -- which bytes were compiled -- and
  `typed-code` hashes the resulting definitions -- what they mean. Neither
  implies the other: the same pinned bytes under a different compiler contract
  produce different definitions, and two different sources can produce the same
  definition. Linking them here means a namespace update can name the exact
  input set it came from, and the returned `:lock-cid` is what a receipt binds
  so that claim is checkable later.

  The linked project is compiled as one module, so a `:require` across the
  locked graph becomes an ordinary intra-module call and then a dependency CID
  like any other."
  ([root namespace lock-path blocks-path] (plan-locked root namespace lock-path blocks-path {}))
  ([root namespace lock-path blocks-path policy]
   (let [{:keys [sources root-namespace lock-cid modules]}
         (let [loaded (module-lock/load-locked-graph lock-path blocks-path)]
           (assoc loaded :root-namespace (:root loaded)))
         linked (project/link-source sources root-namespace)
         module (source->kir (:source linked) policy)]
     (assoc (authoring/plan-with root namespace
                                 #(typed/compile-module module {:definitions %}))
            :lock-cid lock-cid
            :modules modules))))

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
