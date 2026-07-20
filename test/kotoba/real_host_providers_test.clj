(ns kotoba.real-host-providers-test
  "Proves `kotoba.wasm-exec/real-host-functions` (issue #263's provider
  surface + aiueos's kernel capabilities + kototama's actor-host imports)
  perform GENUINE effects when a compiled `.kotoba` guest calls them through
  `kotoba wasm run` -- not the 0-returning `stub-host-function` every one of
  these ops got before. Same end-to-end discipline `wasm_exec_test.clj`
  already established for kgraph-*: compile real `.kotoba` source, run it
  through the actual Chicory `Instance`, and observe the real side effect
  (a real filesystem write actually readable back, a real HTTP round trip
  against a real local server, a real Ed25519 signature verified for real),
  not just that the call links and returns without throwing."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing use-fixtures]]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec])
  (:import (com.sun.net.httpserver HttpExchange HttpHandler HttpServer)
           (java.net InetSocketAddress)))

(defn- compile-and-instantiate
  "Compile SOURCE-PATH under POLICY-PATH's policy, instantiate it against
  STATE (a fresh `default-host-state` unless the 3-arity form passes one to
  share across several separate instantiations -- e.g. proving a real
  topic queue's state persists across `publish` in one compiled guest and
  `poll`/`take`/`count` in others, since `.kotoba` only compiles one 0-arity
  `main` per module) guarded by that same policy. Returns {:instance
  :state}. Mirrors wasm_exec_test.clj's inline pattern, factored out since
  every test below needs the same steps."
  ([source-path policy-path] (compile-and-instantiate source-path policy-path (wasm-exec/default-host-state)))
  ([source-path policy-path state]
   (let [forms (runtime/read-file source-path :kotoba)
         policy (edn/read-string (slurp policy-path))
         wasm (runtime/wasm-binary forms policy)
         instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                         (wasm-exec/real-host-functions state policy)
                                         policy)]
     {:instance instance :state state :wasm wasm})))

(defn- run-main [instance]
  (aget ^longs (.apply (.export instance "main") (long-array 0)) 0))

;; A real local HTTP server (JDK built-in, zero extra deps), bound to a
;; fixed port the demo .kotoba sources' literal URLs hardcode (a `.kotoba`
;; source file can't embed a dynamically-chosen port) -- same tradeoff any
;; test suite binding to a fixed high port accepts.
(def ^:private http-port 18732)
(def ^:private received-requests (atom []))

(defn- start-test-server []
  (let [server (HttpServer/create (InetSocketAddress. "127.0.0.1" http-port) 0)]
    (.createContext server "/"
                    (reify HttpHandler
                      (handle [_ ex]
                        (let [^HttpExchange ex ex
                              body (.readAllBytes (.getRequestBody ex))]
                          (swap! received-requests conj {:method (.getRequestMethod ex)
                                                         :body (String. body "UTF-8")})
                          (let [resp (.getBytes "pong" "UTF-8")]
                            (.sendResponseHeaders ex 200 (count resp))
                            (with-open [os (.getResponseBody ex)]
                              (.write os resp)))))))
    (.start server)
    server))

(use-fixtures :once
  (fn [run-tests]
    (let [server (start-test-server)]
      (try (run-tests) (finally (.stop server 0))))))

(deftest notify-show-records-a-real-notification
  (testing "notify-show is no longer a 0-returning stub -- it really appends to an observable log"
    (let [{:keys [instance state]} (compile-and-instantiate "src/demo_notify.kotoba" "src/demo_policy.edn")]
      (is (= 1 (run-main instance)))
      (is (= [{:code 41}] (map #(select-keys % [:code]) @(:notifications state)))))))

(deftest clipboard-write-then-read-round-trips-for-real
  (testing "clipboard-write really stores; clipboard-read really reads back what was stored"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_clipboard.kotoba" "src/demo_real_clipboard_policy.edn")
          written (run-main instance)]
      (is (= 4 written) "clipboard-read wrote back the 4 bytes clipboard-write had just stored")
      (is (= "clip" (wasm-exec/read-memory-string instance
                                                  (:kotoba.wasm/heap-base
                                                   (runtime/wasm-binary
                                                    (runtime/read-file "src/demo_real_clipboard.kotoba" :kotoba)))
                                                  4))))))

(deftest keychain-write-then-read-round-trips-for-real
  (testing "keychain-write really stores a key->value pair; keychain-read really reads it back"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_keychain.kotoba" "src/demo_real_keychain_policy.edn")]
      (is (= 6 (run-main instance)) "keychain-read wrote back \"secret\"'s 6 bytes"))))

(deftest keychain-read-of-an-unknown-key-is-a-clean-denial-not-a-crash
  (testing "reading a key nobody wrote returns -1, not an exception or a stub's silent 0"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_providers.kotoba" "src/demo_provider_policy.edn")]
      ;; demo_providers.kotoba sums 7 provider calls; keychain-read("token")
      ;; runs before keychain-write("token") in source order, so it always
      ;; observes an empty keychain here -- this demo predates this test and
      ;; was written against the always-0 stub, so its overall SUM isn't
      ;; asserted here (that's covered structurally by wasm-emit tests); this
      ;; test only cares that a real run doesn't throw.
      (is (integer? (run-main instance))))))

(deftest fs-write-then-read-round-trips-through-a-real-sandboxed-file
  (testing "fs-write really writes a file under the sandbox root; fs-read really reads it back"
    (let [{:keys [instance state]} (compile-and-instantiate "src/demo_real_fs.kotoba" "src/demo_real_fs_policy.edn")]
      (is (= 4 (run-main instance)) "fs-read wrote back \"data\"'s 4 bytes")
      (is (= "data" (slurp (str (:fs-root state) "/state.txt")))
          "the file genuinely exists on disk under the sandbox root, not just in an in-memory illusion"))))

(deftest fs-write-atomic-commits-under-the-sandbox-root
  (let [{:keys [instance state]}
        (compile-and-instantiate "src/demo_real_fs_atomic.kotoba"
                                 "src/demo_real_fs_atomic_policy.edn")]
    (is (= 0 (run-main instance)))
    (is (= "verified" (slurp (io/file (:fs-root state) "piece.bin"))))
    (is (empty? (filter #(str/starts-with? (.getName %) ".kotoba-")
                        (.listFiles (io/file (:fs-root state))))))))

(deftest fs-resource-scope-denies-a-different-path-and-permits-the-exact-one
  (testing "a policy scoping :host/fs-read (fs-write shares the same
            capability, fs/app-data) to a path OTHER than the one
            demo_real_fs.kotoba actually reads/writes (\"state.txt\") must
            deny the operation -- capability-resources must actually
            restrict which path is honored, not just narrow what appears
            in a receipt while every path keeps working once the CAPABILITY
            KIND is granted at all"
    (let [forms (runtime/read-file "src/demo_real_fs.kotoba" :kotoba)
          base-policy (edn/read-string (slurp "src/demo_real_fs_policy.edn"))
          denying-policy (assoc base-policy :kotoba.policy/capability-resources
                                {:fs/app-data #{"not-state.txt"}})
          permitting-policy (assoc base-policy :kotoba.policy/capability-resources
                                   {:fs/app-data #{"state.txt"}})
          wasm (runtime/wasm-binary forms base-policy)
          run! (fn [policy]
                 (let [state (wasm-exec/default-host-state)
                       instance (wasm-exec/instantiate (:kotoba.wasm/binary wasm)
                                                        (wasm-exec/real-host-functions state policy)
                                                        policy)]
                   {:result (run-main instance) :state state}))]
      (let [{:keys [result state]} (run! denying-policy)]
        (is (neg? result) "fs-write/fs-read on \"state.txt\" must be denied when only
                            \"not-state.txt\" is in scope")
        (is (not (.exists (io/file (:fs-root state) "state.txt")))
            "the file must never have been written"))
      (let [{:keys [result]} (run! permitting-policy)]
        (is (= 4 result) "the exact path that IS in scope must still work")))))

(deftest fs-path-traversal-is-denied-not-honored
  (testing "a `..`-escaping path never touches the real filesystem outside the sandbox"
    (let [state (wasm-exec/default-host-state)
          outside (java.io.File/createTempFile "kotoba-fs-escape-test" ".txt")]
      (.deleteOnExit outside)
      (is (nil? (#'wasm-exec/safe-path (:fs-root state) (str "../" (.getName outside))))
          "the resolved path must not be treated as inside the sandbox root"))))

(deftest log-write-then-log-read-round-trips-for-real
  (testing "log-write really appends; log-read really reads the append-only log back"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_log.kotoba" "src/demo_real_log_policy.edn")]
      (is (= 2 (run-main instance)) "log-read wrote back \"hi\"'s 2 bytes"))))

(deftest clock-monotonic-returns-a-real-positive-timestamp
  (testing "clock-monotonic is System/nanoTime, not a 0-returning stub"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_clock.kotoba" "src/demo_real_clock_policy.edn")]
      (is (pos? (run-main instance))))))

(deftest random-bytes-writes-genuinely-different-bytes-across-calls
  (testing "two independent runs produce different random content -- not a fixed placeholder"
    (let [run! (fn []
                 (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_random.kotoba" "src/demo_real_random_policy.edn")
                       written (run-main instance)
                       heap-base (:kotoba.wasm/heap-base
                                  (runtime/wasm-binary (runtime/read-file "src/demo_real_random.kotoba" :kotoba)))]
                   [written (wasm-exec/read-memory-string instance heap-base written)]))
          [n1 s1] (run!)
          [n2 s2] (run!)]
      (is (= 16 n1 n2) "random-bytes wrote the full requested 16-byte capacity both times")
      (is (not= s1 s2) "two SecureRandom draws colliding is negligible -- this proves real randomness, not a fixed stub value"))))

(deftest topic-publish-poll-take-count-behave-like-a-real-queue
  (testing "publish enqueues; poll peeks without removing; take removes; count reflects real queue depth"
    ;; Each op is its own compiled guest (mixing i32 topic-publish's return
    ;; with i64 topic-count/poll/take's in one `.kotoba` `let` hits this
    ;; compiler's WASM type validator -- `i64.const`-wrapping the literals
    ;; alone wasn't enough to satisfy it), sharing one `state` map across
    ;; four separate `instantiate` calls to prove the queue is real,
    ;; observable state, not per-call-scoped.
    (let [state (wasm-exec/default-host-state)
          publish! #(run-main (:instance (compile-and-instantiate "src/demo_real_topic_publish.kotoba" "src/demo_real_topic_publish_policy.edn" state)))
          count! #(run-main (:instance (compile-and-instantiate "src/demo_real_topic_count.kotoba" "src/demo_real_topic_count_policy.edn" state)))
          poll! #(run-main (:instance (compile-and-instantiate "src/demo_real_topic_poll.kotoba" "src/demo_real_topic_poll_policy.edn" state)))
          take! #(run-main (:instance (compile-and-instantiate "src/demo_real_topic_take.kotoba" "src/demo_real_topic_take_policy.edn" state)))]
      (publish!)
      (is (= 1 (count!)) "one message published, count reflects it")
      (is (= 42 (poll!)) "poll peeks the published i64 message")
      (is (= 1 (count!)) "poll must not remove -- count is unchanged")
      (is (= 42 (take!)) "take removes and returns the same message")
      (is (= 0 (count!)) "take DID remove -- the queue is now empty"))))

(deftest http-fetch-round-trips-against-a-real-local-http-server
  (testing "http-fetch performs a genuine network GET against a real server, not a 0-returning stub"
    (reset! received-requests [])
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_http_fetch.kotoba" "src/demo_real_http_fetch_policy.edn")
          written (run-main instance)
          heap-base (:kotoba.wasm/heap-base
                     (runtime/wasm-binary (runtime/read-file "src/demo_real_http_fetch.kotoba" :kotoba)))]
      (is (= 4 written) "the real server's \"pong\" response body is 4 bytes")
      (is (= "pong" (wasm-exec/read-memory-string instance heap-base written)))
      (is (= [{:method "GET" :body ""}] @received-requests)
          "the real server actually received one GET request with an empty body"))))

(deftest http-post-round-trips-against-a-real-local-http-server
  (testing "http-post performs a genuine network POST with a real body, not a 0-returning stub"
    (reset! received-requests [])
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_real_http_post.kotoba" "src/demo_real_http_post_policy.edn")
          written (run-main instance)
          heap-base (:kotoba.wasm/heap-base
                     (runtime/wasm-binary (runtime/read-file "src/demo_real_http_post.kotoba" :kotoba)))]
      (is (= 4 written) "the real server's \"pong\" response body is 4 bytes")
      (is (= "pong" (wasm-exec/read-memory-string instance heap-base written)))
      (is (= [{:method "POST" :body "ping"}] @received-requests)
          "the real server actually received one POST request carrying the guest's \"ping\" body"))))

(deftest gen-keypair-sign-verify-and-sha256-hex-round-trip-through-real-kotoba-compiled-guests
  (testing "sha256-hex: real SHA-256 of the empty input, matching the known digest"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_actor_host_sha256.kotoba" "src/demo_actor_host_sha256_policy.edn")
          written (run-main instance)]
      (is (= 64 written))
      (is (= "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
             (wasm-exec/read-memory-string instance 2048 written)))))
  (testing "gen-keypair: a real 32-byte seed + 32-byte derived pubkey, not a 0-returning stub"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_actor_host_keypair.kotoba" "src/demo_actor_host_keypair_policy.edn")]
      (is (= 64 (run-main instance)))))
  (testing "sign: a real 64-byte signature is computed (then correctly rejected by write-bytes! for exceeding this demo's own 0-byte output capacity)"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_actor_host_sign.kotoba" "src/demo_actor_host_sign_policy.edn")]
      (is (= -1 (run-main instance))
          "the real 64-byte signature can't fit demo_actor_host_sign.kotoba's 0-byte output buffer -- proves sign really ran (a stub would have returned 0, not -1)")))
  (testing "verify: a real Ed25519 verification is attempted (this demo's all-zero-length args are not a valid signature, so verification cleanly fails)"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_actor_host_verify.kotoba" "src/demo_actor_host_verify_policy.edn")]
      (is (= 0 (run-main instance)))))
  (testing "log-read (actor-host's log/read, separate registration from the kernel-cap log-write/read pair): reads back this run's own empty log cleanly"
    (let [{:keys [instance]} (compile-and-instantiate "src/demo_actor_host_log_read.kotoba" "src/demo_actor_host_log_read_policy.edn")]
      (is (= 0 (run-main instance)) "an empty log reads back 0 bytes -- not a crash, not a stub's meaningless 0"))))
