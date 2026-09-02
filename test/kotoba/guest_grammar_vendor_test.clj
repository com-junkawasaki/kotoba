(ns kotoba.guest-grammar-vendor-test
  "How far behind kotoba-lang's `lang/guest-grammar.edn` this repository's
  copies are, measured rather than assumed.

  ## What the existing check already does, and what it cannot see

  `kotoba.security-kaizen-test/every-guest-grammar-on-the-classpath-is-the-same-bytes`
  requires every copy of `kotoba/lang/guest-grammar.edn` on this classpath to
  be byte-identical, with no exemption. That is the right check and this file
  does not repeat or weaken it. Measured 2026-09-03 there are FIVE copies --
  `resources/`, `vendor/grammar/resources/`, and one each from the pinned amu,
  kotoba-lang and kotoba-sema -- and all five agree.

  What agreeing with each other cannot tell you is whether they agree with the
  AUTHORITY. All five are one wave behind it, together, and a check that only
  compares them to one another reports green in exactly that state.

  ## The measurement

  | | |
  |---|---|
  | kotoba-lang `lang/guest-grammar.edn`, 2026-09-03 | `67561e57...` |
  | every copy on this classpath | `e20f3e50...` |

  `:admitted-builtins` names THREE kernel heads here -- `kernel-load-u8`,
  `kernel-store-u8`, `kernel-boot-info` -- where kotoba-sema's frontend admits
  114 (53 memory, 8 carried slice, 53 privileged) and the authority now names
  all 114.

  That matters in this repository specifically, and only here.
  `kotoba.grammar/admitted-heads` unions `:admitted-builtins` into the
  known-head set `strict-problems` checks a guest program against, so 111
  heads the compiler admits are reported here as `:unknown-form`. Nothing in
  kotoba-lang, kotoba-sema or amu reads that key at all.

  ## Why the copies are not resynced in this wave

  Resyncing this repository's two copies alone would make them disagree with
  the three that arrive from pinned dependencies, which is precisely what
  `every-guest-grammar-on-the-classpath-is-the-same-bytes` forbids -- and it is
  right to forbid it, because `io/resource` answers with whichever copy comes
  first and admission would be decided by classpath order.

  Bringing the dependency copies forward means advancing the amu pin, which is
  106 commits behind amu main. That is a compiler migration -- it moves the
  backend this repository's whole suite runs against -- not a grammar resync.

  So the gap is RECORDED, with both digests and the closing condition, rather
  than half-closed or left silent. Closing it is one change: advance the amu
  pin (and with it kotoba-lang and kotoba-sema), resync both copies from the
  authority, and update the two literals below in the same commit.

  ## What this file therefore asserts

  A baseline, in the shape the smoke-freshness ratchet uses. Any movement in
  either direction goes red and says what to do:

  - the copies are at the RECORDED digest, so one of them cannot be edited or
    resynced alone;
  - the authority digest is named, so the gap is a number and not a feeling;
  - the loaded catalog names the recorded number of kernel heads, so the
    consequence -- how many admitted heads this repository calls unknown -- is
    measured rather than described."
  (:require [clojure.set :as set]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.grammar :as guest-grammar]))

(def authority-grammar-sha256
  "sha256 of kotoba-lang `lang/guest-grammar.edn` as of 2026-09-03. The same
  literal is pinned in kotoba-lang, kotoba-sema and amu, where it is the
  digest those repositories ARE. Here it is the digest this repository OWES."
  "67561e57ad2b135d848eac75b46ab430d4404a463159f43775e01134e569988f")

(def classpath-grammar-sha256
  "sha256 of every copy on this classpath, 2026-09-03. Behind the authority by
  one wave; see this namespace's docstring for why, and for the single change
  that closes it."
  "e20f3e50727ec8814afb9b664a28b4d2801248c190fb0c5fdc0c347c693b5360")

(def recorded-kernel-head-count
  "Kernel heads `:admitted-builtins` names in the copies on this classpath.
  kotoba-sema's frontend admits 114; the authority now names all 114. The
  difference, 111, is the number of heads `strict-problems` reports here as
  `:unknown-form` although the compiler admits them."
  3)

(def ^:private resource-path "kotoba/lang/guest-grammar.edn")

(defn- sha256-hex [^bytes bs]
  (let [d (.digest (java.security.MessageDigest/getInstance "SHA-256") bs)]
    (apply str (map #(format "%02x" %) d))))

(defn- classpath-copies []
  (->> (enumeration-seq (.getResources (clojure.lang.RT/baseLoader) resource-path))
       (mapv (fn [url]
               {:url (str url)
                :sha256 (sha256-hex
                         (with-open [in (.openStream url)] (.readAllBytes in)))}))))

(deftest the-classpath-grammar-is-one-wave-behind-the-authority
  (let [copies (classpath-copies)
        digests (into #{} (map :sha256) copies)]
    (println (format "COMPARED\t%d\tclasspath copies of %s\tAUTHORITY-GAP\t%s -> %s"
                     (count copies) resource-path
                     (subs classpath-grammar-sha256 0 12)
                     (subs authority-grammar-sha256 0 12)))
    (is (>= (count copies) 2)
        (str "found " (count copies) " copies; a run that opened fewer than two"
             " has measured nothing about a repository that ships two"))
    (is (= #{classpath-grammar-sha256} digests)
        (str "a copy moved off the recorded baseline: " (pr-str digests) "\n"
             "  recorded  " classpath-grammar-sha256 "\n"
             "  authority " authority-grammar-sha256 "\n"
             "If this is the resync: advance the amu pin (and with it"
             " kotoba-lang and kotoba-sema), resync BOTH copies this repository"
             " ships, and update `classpath-grammar-sha256` to the authority"
             " digest in the same commit. Resyncing one copy alone is what"
             " `every-guest-grammar-on-the-classpath-is-the-same-bytes` exists"
             " to refuse."))
    (is (not= classpath-grammar-sha256 authority-grammar-sha256)
        "the recorded baseline and the authority digest are equal, so this gap
         is closed -- delete this test and let
         `every-guest-grammar-on-the-classpath-is-the-same-bytes` and the other
         three repositories' digest pins carry it")))

(deftest the-gap-is-a-number-of-heads-not-a-feeling
  ;; `kotoba.grammar/admitted-heads` is the one reader of `:admitted-builtins`
  ;; anywhere, so this is where the staleness is paid for. Asserting the count
  ;; rather than describing it means the day the copies move, the size of the
  ;; change is visible in the diff of this file.
  (let [admitted (guest-grammar/admitted-heads)
        kernel (into #{} (filter #(or (str/starts-with? (name %) "kernel-")
                                      (str/starts-with? (name %) "slice-")))
                     admitted)]
    (println (format "SCANNED\t%d\tadmitted heads through kotoba.grammar (%d kernel, authority names 114)"
                     (count admitted) (count kernel)))
    (is (pos? (count admitted))
        "the grammar catalog did not load; `kotoba.grammar` falls back to a
         `:status :missing` map with every set empty, and an empty admitted set
         would make this test pass by measuring nothing")
    (is (= recorded-kernel-head-count (count kernel))
        (str "the loaded grammar names " (count kernel) " kernel heads, not "
             recorded-kernel-head-count ". If the copies were resynced, update"
             " both this number and `classpath-grammar-sha256`."))
    (testing "and these are the three, so a different three would not pass"
      (is (= #{"kernel-load-u8" "kernel-store-u8" "kernel-boot-info"}
             (into #{} (map name) kernel))))
    (testing "the heads the compiler admits and this repository still calls
              unknown -- four samples across the three families"
      (doseq [head '[kernel-load-u32 kernel-dot-f32 slice-sub kernel-xsetbv]]
        (is (not (contains? admitted head))
            (str head " is now admitted here; the resync has happened and the"
                 " baselines in this file are stale"))))
    (testing "local-state slice 1 has not reached these copies either:
              atom/swap!/reset! are still forbidden heads here, while the
              authority admits them by elaboration"
      (is (= '#{atom swap! reset!}
             (set/intersection (guest-grammar/forbidden-heads)
                               '#{atom swap! reset!}))))))
