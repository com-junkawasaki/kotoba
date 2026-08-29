(ns kotoba.principal-identity
  "Public Stable Principal enrollment for `kotoba id`.

  The browser owns the Passkey ceremony.  The CLI receives only a one-time
  public projection after the signed-in user approves the displayed device
  code; no Passkey private key, session cookie, wallet seed, or bearer token
  is written here."
  (:require [clojure.data.json :as json]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str])
  (:import [java.awt Desktop Desktop$Action]
           [java.net URI]
           [java.net.http HttpClient HttpRequest HttpRequest$BodyPublishers HttpResponse$BodyHandlers]
           [java.nio.charset StandardCharsets]
           [java.nio.file Files OpenOption StandardOpenOption]
           [java.nio.file.attribute FileAttribute PosixFilePermissions]
           [java.time Duration Instant]))

(def canonical-rp-id "auth.kotoba.cloud")
(def canonical-origin (str "https://" canonical-rp-id))
(def principal-relpath "kotoba/principal.edn")

(def ^:dynamic *principal-path* nil)
(def ^:dynamic *browser-open!* nil)
(def ^:dynamic *device-authorize!* nil)

(defn principal-path*
  [xdg-data-home home]
  (.getPath (io/file (if (and (string? xdg-data-home)
                              (not (str/blank? xdg-data-home)))
                       xdg-data-home
                       (str home "/.local/share"))
                     principal-relpath)))

(defn principal-path
  []
  (or *principal-path*
      (principal-path* (System/getenv "XDG_DATA_HOME")
                       (or (System/getenv "HOME")
                           (System/getProperty "user.home")))))

(defn- set-posix-mode!
  [path mode]
  (try
    (Files/setPosixFilePermissions path (PosixFilePermissions/fromString mode))
    (catch UnsupportedOperationException _)))

(defn write-principal!
  "Persist only the verified public projection, mode 0600."
  [path identity]
  (let [nio (.toPath (io/file path))
        parent (.getParent nio)
        record {:kotoba.principal/version 1
                :kotoba.principal/id (:principalId identity)
                :kotoba.principal/username (:handle identity)
                :kotoba.principal/account-did (:accountDid identity)
                :kotoba.principal/controller (:activeDid identity)
                :kotoba.principal/rp-id canonical-rp-id
                :kotoba.principal/verified-at (str (Instant/now))}]
    (when parent
      (Files/createDirectories parent (make-array FileAttribute 0))
      (set-posix-mode! parent "rwx------"))
    (Files/write nio
                 (.getBytes (str (pr-str record) "\n") StandardCharsets/UTF_8)
                 (into-array OpenOption [StandardOpenOption/CREATE
                                         StandardOpenOption/TRUNCATE_EXISTING
                                         StandardOpenOption/WRITE]))
    (set-posix-mode! nio "rw-------")
    record))

(defn read-principal
  ([] (read-principal (principal-path)))
  ([path]
   (let [file (io/file path)]
     (when (.isFile file)
       (try
         (let [record (edn/read-string {:readers {} :default (fn [_ _] nil)} (slurp file))]
           (when (= 1 (:kotoba.principal/version record)) record))
         (catch Exception _ nil))))))

(defn public-identity?
  [identity]
  (and (true? (:valid identity))
       (string? (:principalId identity))
       (or (str/starts-with? (:principalId identity) "did:")
           (str/starts-with? (:principalId identity) "urn:kotoba:principal:"))
       (string? (:accountDid identity))
       (str/starts-with? (:accountDid identity) "did:")
       (string? (:activeDid identity))
       (str/starts-with? (:activeDid identity) "did:")
       (string? (:handle identity))
       (not (str/blank? (:handle identity)))
       (every? #(not (str/includes? % "\n"))
               ((juxt :principalId :accountDid :activeDid :handle) identity))))

(defn- post-json
  [path body]
  (let [client (.build (-> (HttpClient/newBuilder)
                           (.connectTimeout (Duration/ofSeconds 5))))
        request (-> (HttpRequest/newBuilder (URI/create (str canonical-origin path)))
                    (.timeout (Duration/ofSeconds 10))
                    (.header "accept" "application/json")
                    (.header "content-type" "application/json")
                    (.POST (HttpRequest$BodyPublishers/ofString (json/write-str body)))
                    (.build))
        response (.send client request (HttpResponse$BodyHandlers/ofString))
        retry-after-value (.firstValue (.headers response) "retry-after")
        retry-after (when (.isPresent retry-after-value)
                      (try
                        (Long/parseLong (.get retry-after-value))
                        (catch NumberFormatException _ nil)))]
    {:status (.statusCode response)
     :retry-after retry-after
     :body (try
             (json/read-str (.body response) :key-fn keyword)
             (catch Exception _ {}))}))

(defn device-retry-delay-seconds
  "Return the bounded delay for a retryable device-token response.  428 keeps
  the advertised interval; 429 honors Retry-After and otherwise applies the
  OAuth device-flow slow-down rule."
  [status interval retry-after]
  (when (#{428 429} status)
    (long (max 1
               (min 300
                    (if (= 429 status)
                      (max interval (or retry-after (+ interval 5)))
                      interval))))))

(defn open-browser!
  [url]
  (try
    (cond
      (and (Desktop/isDesktopSupported)
           (.isSupported (Desktop/getDesktop) Desktop$Action/BROWSE))
      (do (.browse (Desktop/getDesktop) (URI/create url)) true)

      (= "Mac OS X" (System/getProperty "os.name"))
      (do (.start (ProcessBuilder. ^java.util.List ["open" url])) true)

      :else false)
    (catch Exception _ false)))

(defn device-authorize!
  []
  (let [{:keys [status body]} (post-json "/v1/device/start" {})]
    (when-not (= 200 status)
      (throw (ex-info "Kotoba identity service refused device authorization"
                      {:status status :error (:error body)})))
    (let [code (:deviceCode body)
          verification (:verificationUriComplete body)
          interval (long (max 1 (or (:interval body) 2)))
          deadline (+ (System/currentTimeMillis)
                      (* 1000 (long (or (:expiresIn body) 600))))
          opened? ((or *browser-open!* open-browser!) verification)]
      (binding [*out* *err*]
        (println "PasskeyでKotoba CLIを接続してください:")
        (println verification)
        (println "確認コード:" (:userCode body))
        (when-not opened? (println "ブラウザを自動で開けなかったため、上のURLを開いてください。")))
      (loop []
        (when (>= (System/currentTimeMillis) deadline)
          (throw (ex-info "Kotoba Passkey approval timed out" {:error :expired-token})))
        (let [{poll-status :status poll-body :body poll-retry-after :retry-after}
              (post-json "/v1/device/token" {:deviceCode code})]
          (cond
            (= 200 poll-status) (:identity poll-body)
            (#{428 429} poll-status)
            (do (Thread/sleep (* 1000 (device-retry-delay-seconds
                                       poll-status interval poll-retry-after)))
                (recur))
            :else (throw (ex-info "Kotoba Passkey approval failed"
                                  {:status poll-status :error (:error poll-body)}))))))))

(defn enroll-result
  [authority-result]
  (let [requested-rp (get-in authority-result [:kotoba.cli/data :controller :rp-id])]
    (if-not (= canonical-rp-id requested-rp)
      {:kotoba.cli/ok? false
       :kotoba.cli/code :id/noncanonical-rp
       :kotoba.cli/message (str "kotoba id new uses the canonical Passkey RP " canonical-rp-id)
       :kotoba.cli/data {:requested-rp-id requested-rp :rp-id canonical-rp-id}}
      (try
        (let [identity ((or *device-authorize!* device-authorize!))]
          (if-not (public-identity? identity)
            {:kotoba.cli/ok? false :kotoba.cli/code :id/invalid-principal-projection
             :kotoba.cli/data {:rp-id canonical-rp-id}}
            (let [path (principal-path)
                  stored (write-principal! path identity)]
              {:kotoba.cli/ok? true
               :kotoba.cli/code :id/enrolled
               :kotoba.cli/data
               (cond-> {:principal (:kotoba.principal/id stored)
                        :username (:kotoba.principal/username stored)
                        :account-did (:kotoba.principal/account-did stored)
                        :controller {:kind :passkey
                                     :rp-id canonical-rp-id
                                     :did (:kotoba.principal/controller stored)}
                        :method :passkey-smart-account
                        :proof :webauthn-passkey
                        :custody :passkey-provider
                        :authority :verified-device-flow
                        :path path
                        :chain-default nil}
                 (seq (get-in authority-result [:kotoba.cli/data :accounts]))
                 (assoc :accounts (get-in authority-result [:kotoba.cli/data :accounts])))})))
        (catch Exception error
          {:kotoba.cli/ok? false
           :kotoba.cli/code :id/enrollment-failed
           :kotoba.cli/message (ex-message error)
           :kotoba.cli/data (merge {:rp-id canonical-rp-id}
                                   (select-keys (ex-data error) [:status :error]))})))))

(defn show-result
  []
  (if-let [record (read-principal)]
    {:kotoba.cli/ok? true :kotoba.cli/code :id/shown
     :kotoba.cli/data {:principal (:kotoba.principal/id record)
                       :username (:kotoba.principal/username record)
                       :account-did (:kotoba.principal/account-did record)
                       :controller {:kind :passkey
                                    :rp-id (:kotoba.principal/rp-id record)
                                    :did (:kotoba.principal/controller record)}
                       :path (principal-path)}}
    {:kotoba.cli/ok? false :kotoba.cli/code :id/not-enrolled
     :kotoba.cli/data {:path (principal-path)
                       :hint "run `kotoba id new`"}}))
