(ns kotoba.deploy-adapter
  "Concrete local deployment adapter for admitted semantic artifacts.

  Releases are immutable directories named by semantic receipt CID. The only
  mutable state is a target's current event ref, advanced under a file lock and
  expected-release CAS. Deployment and rollback events are content addressed
  and signed by the deploy-store controller."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [ed25519.core :as ed]
            [kotoba.semantic-code :as semantic-code]
            [kotoba.semantic-supply-chain :as semantic-supply-chain])
  (:import [java.nio ByteBuffer]
           [java.nio.channels FileChannel]
           [java.nio.charset StandardCharsets]
           [java.nio.file Files Path StandardCopyOption StandardOpenOption]
           [java.time Instant]
           [java.util Base64 Comparator UUID]))

(def store-schema "kotoba.local-deploy-store.v1")
(def event-schema "kotoba.deployment-receipt.v1")
(def max-artifact-bytes (* 512 1024 1024))
(def target-pattern #"[A-Za-z0-9][A-Za-z0-9._-]{0,127}")
(def release-pattern #"b[a-z2-7]{20,200}")

(defn- file [root & parts] (apply io/file root parts))
(defn- target-directory [root target] (file root "targets" target))
(defn- release-directory [root target release]
  (file (target-directory root target) "releases" release))
(defn- event-file [root target cid]
  (file (target-directory root target) "events" (str cid ".edn")))
(defn- current-file [root target]
  (file (target-directory root target) "current.head"))

(defn- require-target! [target]
  (when-not (and (string? target) (re-matches target-pattern target))
    (throw (ex-info "deployment target must be a bounded safe identifier"
                    {:problem :deploy/invalid-target :target target}))))

(defn- require-release! [release]
  (when-not (and (string? release)
                 (re-matches release-pattern release))
    (throw (ex-info "deployment release must be a canonical CID"
                    {:problem :deploy/invalid-release
                     :release release}))))

(defn- force-directory! [^Path directory]
  (with-open [channel (FileChannel/open
                       directory
                       (into-array StandardOpenOption
                                   [StandardOpenOption/READ]))]
    (.force channel true)))

(defn- atomic-write-bytes! [target ^bytes bytes]
  (let [path (.toPath ^java.io.File target)
        parent (.getParent path)
        _ (Files/createDirectories parent
                                   (make-array java.nio.file.attribute.FileAttribute
                                               0))
        tmp (Files/createTempFile
             parent "deploy-" ".tmp"
             (make-array java.nio.file.attribute.FileAttribute 0))]
    (try
      (with-open [channel
                  (FileChannel/open
                   tmp
                   (into-array StandardOpenOption
                               [StandardOpenOption/WRITE]))]
        (loop [buffer (ByteBuffer/wrap bytes)]
          (when (.hasRemaining buffer)
            (.write channel buffer)
            (recur buffer)))
        (.force channel true))
      (Files/move tmp path
                  (into-array StandardCopyOption
                              [StandardCopyOption/ATOMIC_MOVE
                               StandardCopyOption/REPLACE_EXISTING]))
      (force-directory! parent)
      (finally
        (Files/deleteIfExists tmp)))))

(defn- atomic-write-edn! [target value]
  (atomic-write-bytes!
   target (.getBytes (pr-str value) StandardCharsets/UTF_8)))

(defn- read-edn [target]
  (edn/read-string
   {:readers {}
    :default (fn [tag _]
               (throw (ex-info "tagged deployment-store value rejected"
                               {:problem :deploy/store-corrupt :tag tag})))}
   (slurp target)))

(defn- marker-file [root] (file root "DEPLOY_STORE.edn"))

(defn- initialize! [root controller]
  (Files/createDirectories
   (.toPath (io/file root))
   (make-array java.nio.file.attribute.FileAttribute 0))
  (let [lock-path (.toPath (file root "STORE_LOCK"))]
    (with-open [channel
                (FileChannel/open
                 lock-path
                 (into-array StandardOpenOption
                             [StandardOpenOption/CREATE
                              StandardOpenOption/WRITE]))
                _lock (.lock channel)]
      (let [marker (marker-file root)]
        (if (.exists marker)
          (let [stored (read-edn marker)]
            (when-not (and (= store-schema (:schema stored))
                           (= controller (:controller stored)))
              (throw (ex-info "deployment store controller mismatch"
                              {:problem :deploy/controller-mismatch
                               :expected (:controller stored)
                               :actual controller}))))
          (atomic-write-edn! marker
                             {:schema store-schema
                              :controller controller})))))
  true)

(defn- require-store! [root]
  (let [marker (marker-file root)
        value (when (.isFile marker) (read-edn marker))]
    (when-not (= store-schema (:schema value))
      (throw (ex-info "deployment store is not initialized"
                      {:problem :deploy/store-not-initialized})))
    value))

(defn- statement-bytes [cid]
  (.getBytes
   (str "kotoba.deployment-receipt/v1\nreceipt-cid:" cid "\n")
   StandardCharsets/UTF_8))

(defn- sign-event [seed body]
  (let [controller (ed/did-key-from-seed seed)
        cid (semantic-supply-chain/canonical-cid body)]
    {:schema event-schema
     :receipt-cid cid
     :controller controller
     :body body
     :signature
     (.encodeToString
      (Base64/getEncoder)
      (ed/sign seed (statement-bytes cid)))}))

(defn- verify-event! [controller event]
  (let [body (:body event)
        declared (:receipt-cid event)
        computed (semantic-supply-chain/canonical-cid body)
        valid?
        (try
          (ed/verify-did
           controller (statement-bytes declared)
           (.decode (Base64/getDecoder) ^String (:signature event)))
          (catch Exception _ false))]
    (when-not (and (= event-schema (:schema event))
                   (= controller (:controller event))
                   (= declared computed)
                   valid?)
      (throw (ex-info "deployment event is corrupt or unauthorized"
                      {:problem :deploy/invalid-deployment-receipt
                       :declared declared :computed computed})))
    event))

(defn- current-event [root target controller]
  (let [ref (current-file root target)]
    (when (.isFile ref)
      (let [event-cid (read-edn ref)
            stored (event-file root target event-cid)]
        (when-not (.isFile stored)
          (throw (ex-info "deployment current ref has no event"
                          {:problem :deploy/corrupt-current-ref
                           :event-cid event-cid})))
        (verify-event! controller (read-edn stored))))))

(defn- write-event! [root target event]
  (let [cid (:receipt-cid event)
        target-file (event-file root target cid)]
    (if (.exists target-file)
      (when-not (= event (read-edn target-file))
        (throw (ex-info "immutable deployment event conflict"
                        {:problem :deploy/immutable-event-conflict
                         :event-cid cid})))
      (atomic-write-edn! target-file event))
    cid))

(defn- delete-tree! [^Path root]
  (when (Files/exists root (make-array java.nio.file.LinkOption 0))
    (with-open [paths (Files/walk root (make-array java.nio.file.FileVisitOption 0))]
      (.forEach
       (.sorted paths (Comparator/reverseOrder))
       (reify java.util.function.Consumer
         (accept [_ path] (Files/deleteIfExists ^Path path)))))))

(defn- write-release!
  [root target receipt manifest artifact-bytes receipt-trust]
  (let [release (:kotoba.semantic-build/receipt-cid receipt)
        _ (require-release! release)
        releases (.toPath (file (target-directory root target) "releases"))
        _ (Files/createDirectories
           releases (make-array java.nio.file.attribute.FileAttribute 0))
        destination (.toPath (release-directory root target release))]
    (if (Files/exists destination (make-array java.nio.file.LinkOption 0))
      release
      (let [stage (Files/createDirectory
                   (.resolve releases
                             (str ".stage-" (UUID/randomUUID)))
                   (make-array java.nio.file.attribute.FileAttribute 0))]
        (try
          (atomic-write-bytes! (.toFile (.resolve stage "artifact.bin"))
                               artifact-bytes)
          (atomic-write-edn! (.toFile (.resolve stage "artifact.manifest.edn"))
                             manifest)
          (atomic-write-edn! (.toFile (.resolve stage "semantic.receipt.edn"))
                             receipt)
          (atomic-write-edn! (.toFile (.resolve stage "receipt-trust.edn"))
                             receipt-trust)
          (atomic-write-bytes!
           (.toFile (.resolve stage "sbom.spdx.json"))
           (.getBytes
            (semantic-supply-chain/spdx-json
             (:kotoba.semantic-build/spdx receipt))
            StandardCharsets/UTF_8))
          (force-directory! stage)
          (Files/move stage destination
                      (into-array StandardCopyOption
                                  [StandardCopyOption/ATOMIC_MOVE]))
          (force-directory! releases)
          release
          (finally
            (delete-tree! stage)))))))

(defn- release-files [root target release]
  (let [directory (release-directory root target release)]
    {:directory directory
     :artifact (file directory "artifact.bin")
     :manifest (file directory "artifact.manifest.edn")
     :receipt (file directory "semantic.receipt.edn")
     :trust (file directory "receipt-trust.edn")
     :spdx (file directory "sbom.spdx.json")}))

(defn verify-release!
  "Reverify one stored release from bytes through signed semantic receipt."
  [root target release]
  (require-release! release)
  (let [{:keys [directory artifact manifest receipt trust spdx]}
        (release-files root target release)]
    (when-not
     (and
      (.isDirectory directory)
      (every?
       #(and (.isFile ^java.io.File %)
             (not (Files/isSymbolicLink
                   (.toPath ^java.io.File %))))
       [artifact manifest receipt trust spdx])
      (<= (.length ^java.io.File artifact) max-artifact-bytes)
      (every? #(<= (.length ^java.io.File %) (* 16 1024 1024))
              [manifest receipt trust spdx]))
      (throw (ex-info "deployment release is incomplete"
                      {:problem :deploy/release-incomplete
                       :release release})))
    (let [receipt-value (read-edn receipt)
          manifest-value (read-edn manifest)
          trust-value (read-edn trust)
          artifact-bytes (Files/readAllBytes (.toPath artifact))
          spdx-bytes (Files/readAllBytes (.toPath spdx))
          admission
          (semantic-supply-chain/verify-receipt
           receipt-value manifest-value artifact-bytes release trust-value)]
      (when-not (= (:kotoba.semantic-build/spdx-cid receipt-value)
                   (semantic-code/source-cid spdx-bytes))
        (throw (ex-info "stored SPDX bytes do not match receipt"
                        {:problem :deploy/spdx-mismatch
                         :release release})))
      {:release release
       :artifact-cid (:kotoba.deploy/artifact-cid admission)
       :semantic-root-cid (:kotoba.deploy/semantic-root-cid admission)
       :derivation-cid (:kotoba.deploy/derivation-cid admission)
       :spdx-cid (:kotoba.deploy/spdx-cid admission)
       :receipt-signer (:kotoba.deploy/signer admission)})))

(defn status!
  [root target]
  (require-target! target)
  (let [{:keys [controller]} (require-store! root)
        event (current-event root target controller)]
    (if-not event
      {:target target :status :empty}
      (let [release (get-in event [:body :release-cid])
            verified (verify-release! root target release)]
        (when-not
         (= (select-keys (:body event)
                         [:artifact-cid :semantic-root-cid :derivation-cid])
            (select-keys verified
                         [:artifact-cid :semantic-root-cid :derivation-cid]))
          (throw (ex-info "deployment event does not match stored release"
                          {:problem :deploy/deployment-release-mismatch
                           :release release})))
        {:target target
         :status :deployed
         :current-release release
         :deployment-receipt-cid (:receipt-cid event)
         :deployment-controller controller
         :operation (get-in event [:body :operation])
         :deployed-at (get-in event [:body :deployed-at])
         :verified-release verified}))))

(defn- cas! [current expected next-release]
  (let [actual (get-in current [:body :release-cid])]
    (when (and actual (nil? expected) (not= actual next-release))
      (throw (ex-info "deployment update requires --expected-deployment-head"
                      {:problem :deploy/expected-head-required
                       :actual actual})))
    (when (and expected (not= expected actual))
      (throw (ex-info "deployment head compare-and-set failed"
                      {:problem :deploy/head-cas-mismatch
                       :expected expected :actual actual})))
    actual))

(defn apply!
  [{:keys [root target expected-head seed receipt manifest artifact-bytes
           receipt-trust]}]
  (require-target! target)
  (when-not (and (bytes? seed) (= 32 (count seed)))
    (throw (ex-info "deployment signing key must be 32 bytes"
                    {:problem :deploy/signing-key-invalid})))
  (let [controller (ed/did-key-from-seed seed)
        _ (initialize! root controller)
        target-dir (target-directory root target)
        _ (Files/createDirectories
           (.toPath target-dir)
           (make-array java.nio.file.attribute.FileAttribute 0))
        lock-path (.toPath (file target-dir "LOCK"))]
    (with-open [channel
                (FileChannel/open
                 lock-path
                 (into-array StandardOpenOption
                             [StandardOpenOption/CREATE
                              StandardOpenOption/WRITE]))
                _lock (.lock channel)]
      (let [current (current-event root target controller)
            release (:kotoba.semantic-build/receipt-cid receipt)
            previous (cas! current expected-head release)
            _ (semantic-supply-chain/verify-receipt
               receipt manifest artifact-bytes release receipt-trust)
            _ (write-release! root target receipt manifest artifact-bytes
                              receipt-trust)
            event
            (sign-event
             seed
             {:schema "kotoba.deployment-event.v1"
              :operation :apply
              :target target
              :release-cid release
              :previous-release-cid previous
              :artifact-cid (:kotoba.semantic-build/artifact-cid receipt)
              :semantic-root-cid
              (:kotoba.semantic-build/semantic-root-cid receipt)
              :derivation-cid
              (:kotoba.semantic-build/derivation-cid receipt)
              :deployed-at (str (Instant/now))})
            event-cid (write-event! root target event)
            verified (verify-release! root target release)]
        (atomic-write-edn! (current-file root target) event-cid)
        {:target target
         :operation :apply
         :current-release release
         :previous-release previous
         :deployment-receipt event
         :verified-release verified}))))

(defn rollback!
  [{:keys [root target revision expected-head seed]}]
  (require-target! target)
  (when-not (and (string? revision) (bytes? seed) (= 32 (count seed)))
    (throw (ex-info "rollback requires revision and deployment signing key"
                    {:problem :deploy/rollback-input-invalid})))
  (let [{:keys [controller]} (require-store! root)
        signer (ed/did-key-from-seed seed)
        _ (when-not (= controller signer)
            (throw (ex-info "rollback signer is not deployment controller"
                            {:problem :deploy/controller-mismatch})))
        target-dir (target-directory root target)
        lock-path (.toPath (file target-dir "LOCK"))]
    (with-open [channel
                (FileChannel/open
                 lock-path
                 (into-array StandardOpenOption
                             [StandardOpenOption/CREATE
                              StandardOpenOption/WRITE]))
                _lock (.lock channel)]
      (let [current (current-event root target controller)
            previous (cas! current expected-head revision)
            verified (verify-release! root target revision)
            event
            (sign-event
             seed
             {:schema "kotoba.deployment-event.v1"
              :operation :rollback
              :target target
              :release-cid revision
              :previous-release-cid previous
              :artifact-cid (:artifact-cid verified)
              :semantic-root-cid (:semantic-root-cid verified)
              :derivation-cid (:derivation-cid verified)
              :deployed-at (str (Instant/now))})
            event-cid (write-event! root target event)]
        (atomic-write-edn! (current-file root target) event-cid)
        {:target target
         :operation :rollback
         :current-release revision
         :previous-release previous
         :deployment-receipt event
         :verified-release verified}))))
