(ns kotoba.guest-grammar-vendor-test
  "Every copy of kotoba-lang's `lang/guest-grammar.edn` on this classpath is
  the authority's bytes, and the authority is named by digest.

  ## What this adds to the check next door

  `kotoba.security-kaizen-test/every-guest-grammar-on-the-classpath-is-the-same-bytes`
  requires every copy of `kotoba/lang/guest-grammar.edn` here to be
  byte-identical, with no exemption. That is the right check and this file does
  not repeat or weaken it. What agreeing with each OTHER cannot tell you is
  whether they agree with the AUTHORITY: measured 2026-09-03 all five copies --
  `resources/`, `vendor/grammar/resources/`, and one each from the pinned amu,
  kotoba-lang and kotoba-sema -- agreed with each other while being one wave
  behind kotoba-lang, and this file was written to record that gap as a number.

  ## The 112-head gap is closed; the one-head gap behind it is closed too

  Closing it was the one change the earlier version of this namespace said it
  would be: advance the amu pin (and with it kotoba-lang and kotoba-sema),
  resync BOTH copies this repository ships, in one commit. Done 2026-09-03 at
  authority `67561e57`. **While that change was open, the authority moved
  again** (kotoba-lang added `kernel-uefi-alloc-region`, taking the authority
  to `6e1202fd` / 115 kernel heads), which reopened a one-head gap recorded
  here as two literals.

  Closed for good 2026-09-04: both copies, the amu / kotoba-lang / kotoba-sema
  pins and the `ci.yml` authority checkout moved together to kotoba-lang
  dd8bcb62 (`871f3873...`), so the classpath IS the authority and there is one
  kernel-head count, not two. `authority-grammar-sha256` and
  `authority-kernel-head-count` are gone, as the test itself prescribed for
  exactly this state. What keeps the gap closed from here:
  `every-guest-grammar-on-the-classpath-is-the-same-bytes` (byte-identity of
  every copy), this file's digest pin, and the digest pins the other three
  repositories carry.

  ## What the gap costs, for the record

  `kotoba.grammar/admitted-heads` unions `:admitted-builtins` into the
  known-head set `strict-problems` checks a guest program against, and nothing
  in kotoba-lang, kotoba-sema or amu reads that key at all. While the copies
  were at `e20f3e50`, `:admitted-builtins` named THREE kernel heads where the
  frontend admits 114, so 111 heads the compiler admits were reported here as
  `:unknown-form`. It names 115 now."
  (:require [clojure.set :as set]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [kotoba.grammar :as guest-grammar]))

(def classpath-grammar-sha256
  "sha256 of every copy on this classpath: the two this repository ships and
  the ones the pinned amu, kotoba-lang and kotoba-sema carry. Resynced to the
  authority 2026-09-05 at kbb slice 2 (env-read + fs-browse string heads, 9d701ea...; prior #550 811e3d5e); NOTE: sema bf01d4a8 (kbb slice 3) vendors the same 9d701ea copy, so the classpath now carries ONE digest (kotoba-lang PR #553 ships this repo's two copies; the sema gap is tracked separately), in one commit
  with the three pins and the ci.yml authority checkout. Change it only as part
  of a resync wave, in all four repositories, and resync both copies this
  repository ships in the same commit."
  "9d701ea9a803a4b3d7dc4245274a9a901ab4ac506ebd401282b2acdf7747dd9c")

(def recorded-kernel-head-count
  "Kernel heads `:admitted-builtins` names in the copies on this classpath,
  equal to what kotoba-lang main names -- the gap is closed, so there is one
  count, not two."
  115)

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

(deftest every-classpath-copy-is-the-pinned-grammar
  (let [copies (classpath-copies)
        digests (into #{} (map :sha256) copies)]
    (println (format "COMPARED\t%d\tclasspath copies of %s\tAT\t%s\t(%d kernel heads, gap closed 2026-09-04)"
                     (count copies) resource-path
                     (subs classpath-grammar-sha256 0 12)
                     recorded-kernel-head-count))
    (is (>= (count copies) 2)
        (str "found " (count copies) " copies; a run that opened fewer than two"
             " has measured nothing about a repository that ships two"))
    (is (= #{classpath-grammar-sha256} digests)
        (str "a copy moved off the recorded baseline: " (pr-str digests) "\n"
             "  recorded  " classpath-grammar-sha256 "\n"
             "Resyncing one copy alone is what"
             " `every-guest-grammar-on-the-classpath-is-the-same-bytes` exists"
             " to refuse: the two this repository ships move together with the"
             " amu, kotoba-lang and kotoba-sema pins, in one commit."))))

(deftest the-authority-reaches-the-one-place-that-reads-it
  ;; `kotoba.grammar/admitted-heads` is the one reader of `:admitted-builtins`
  ;; anywhere, so this is where a stale copy is paid for. Asserting the count
  ;; rather than describing it means any movement is visible in this file.
  (let [admitted (guest-grammar/admitted-heads)
        kernel (into #{} (filter #(or (str/starts-with? (name %) "kernel-")
                                      (str/starts-with? (name %) "slice-")))
                     admitted)]
    (println (format "SCANNED\t%d\tadmitted heads through kotoba.grammar (%d kernel)"
                     (count admitted) (count kernel)))
    (is (pos? (count admitted))
        "the grammar catalog did not load; `kotoba.grammar` falls back to a
         `:status :missing` map with every set empty, and an empty admitted set
         would make this test pass by measuring nothing")
    (is (= recorded-kernel-head-count (count kernel))
        (str "the loaded grammar names " (count kernel) " kernel heads, not "
             recorded-kernel-head-count "."))
    (testing "four samples across the three kernel families, none of which the
              three-head copy carried"
      (doseq [head '[kernel-load-u32 kernel-dot-f32 slice-sub kernel-xsetbv]]
        (is (contains? admitted head)
            (str head " is not admitted here, so these copies are behind the"
                 " frontend again"))))
    (testing "local-state slice 1 arrived with the same bytes: atom, swap! and
              reset! are no longer forbidden heads, because the authority now
              admits a function-local atom by elaboration"
      (is (empty? (set/intersection (guest-grammar/forbidden-heads)
                                    '#{atom swap! reset!}))))))
