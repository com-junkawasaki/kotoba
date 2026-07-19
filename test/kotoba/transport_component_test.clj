(ns kotoba.transport-component-test
  (:require [clojure.edn :as edn]
            [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime])
  (:import (com.dylibso.chicory.wasm Parser)))

(def expected-ops
  '[transport-connect tls-open transport-write transport-read transport-close])

(deftest http-and-database-provider-components-lower-to-bounded-imports
  (let [policy (edn/read-string (slurp "providers/transport_policy.edn"))]
    (doseq [[path exports]
            [["providers/http_transport.kotoba"
              ["http-open" "http-write" "http-read" "http-close"
               "http-status-code" "http-find-header-end"
               "http-parse-content-length-digits" "http-skip-content-length-spaces"
               "http-ascii-ci-byte=?" "http-header-line-start?"
               "http-content-length-name-at?" "http-transfer-encoding-name-at?"
               "http-find-content-length-from" "http-has-transfer-encoding?"
               "http-find-transfer-encoding" "http-count-content-length"
               "http-find-content-length" "http-skip-spaces"
               "http-transfer-encoding-is-chunked?" "http-hex-value"
               "http-find-crlf" "http-parse-chunk-size" "http-copy-bytes!"
               "http-decode-chunks!" "http-normalize-response!"
               "http-response-valid?" "http-read-bounded"
               "http-get" "main"]]
             ["providers/db_transport.kotoba"
              ["db-open" "db-write" "db-read" "db-close" "db-frame-length"
               "db-frame-valid?" "db-read-bounded" "db-exchange"
               "pg-message-length" "pg-message-valid?"
               "pg-authentication-ok?" "pg-ready-for-query?"
               "pg-auth-ready-response?" "pg-parameter-status-valid?"
               "pg-backend-key-data?" "pg-startup-response-valid-from?"
               "pg-startup-response-valid?" "pg-read-auth-ready-exact"
               "pg-simple-query" "pg-read-exact"
               "pg-bytes-no-zero?" "pg-startup-field-valid?"
               "pg-query-text-valid?" "pg-open-tls" "pg-open-valid" "pg-open"
               "pg-tail-ready?" "pg-read-until-ready"
               "pg-query-tag-allowed?" "pg-error-find-zero"
               "pg-error-copy-bytes!" "pg-error-fields-valid-from?"
               "pg-error-response-valid?" "pg-notice-response-valid?"
               "pg-error-copy-field!"
               "pg-error-count-field"
               "pg-query-result-copy-sqlstate-from!"
               "pg-query-result-tag-allowed?" "pg-query-result-valid-from?"
               "pg-query-result-has-error-from?" "pg-query-ready-status"
               "pg-query-response-valid-from?" "pg-query-valid" "pg-query"
               "pg-query-state-valid" "pg-query-state"
               "pg-store-i16-be!" "pg-store-i32-be!"
               "pg-statement-name-valid?" "pg-parameter-valid?"
               "pg-i16-be" "pg-i32-be" "pg-null-length?"
               "pg-format-codes-valid?" "pg-bind-values-end"
               "pg-bind-params-valid?" "pg-oid-list-valid?"
               "pg-extended-fixed-message-valid?" "pg-extended-tag-valid?"
               "pg-extended-result-valid-from?" "pg-result-count-tag-from"
               "pg-result-write-meta!" "pg-prepare-valid" "pg-prepare"
               "pg-prepare-typed-valid" "pg-prepare-typed"
               "pg-execute-params2-valid" "pg-execute-params2"
               "pg-execute-params-valid" "pg-execute-params"
               "pg-bind-portal-valid" "pg-bind-portal"
               "pg-fetch-portal-valid" "pg-fetch-portal"
               "pg-close-portal-valid" "pg-close-portal"
               "pg-close-statement-valid" "pg-close-statement"
               "pg-copy-format-codes-valid?" "pg-copy-response-valid?"
               "pg-copy-out-result-valid-from?" "pg-copy-in-final-valid-from?"
               "pg-copy-out-valid" "pg-copy-out"
               "pg-copy-in-valid" "pg-copy-in"
               "pg-batch-items-end" "pg-batch-valid?"
               "pg-batch-write-items" "pg-execute-batch-valid"
               "pg-execute-batch" "pg-session-reset" "main"]]]]
      (testing path
        (let [forms (runtime/read-file path :kotoba)
              wasm (runtime/wasm-binary forms policy)]
          (is (:kotoba.wasm/ok? wasm))
          (is (= expected-ops (runtime/required-host-imports forms)))
          (is (= ["transport_connect" "tls_open" "transport_write"
                  "transport_read" "transport_close"]
                 (mapv :field (:kotoba.wasm/imports wasm))))
          (is (= exports (:kotoba.wasm/exports wasm)))
          (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm)))
              "nested let temporaries never alias a later i64 handle local"))))))

(deftest transport-components-fail-closed-without-grants
  (doseq [path ["providers/http_transport.kotoba"
                "providers/db_transport.kotoba"]]
    (let [forms (runtime/read-file path :kotoba)
          result (runtime/check nil
                                {:kotoba.source/path path
                                 :kotoba.source/reader-target :kotoba}
                                forms nil)]
      (is (false? (:kotoba.runtime/ok? result)))
      (is (some #(= :capability-not-granted (:kotoba.runtime/problem %))
                (:kotoba.runtime/problems result))))))

(deftest postgresql-scram-component-uses-only-purpose-bound-secret-import
  (let [path "providers/pg_scram.kotoba"
        forms (runtime/read-file path :kotoba)
        policy (edn/read-string (slurp "providers/pg_scram_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[transport-connect tls-open tls-server-end-point
             transport-write transport-read
             transport-close scram-sha256 pg-cancel-register pg-cancel
             random-bytes]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["transport_connect" "tls_open" "tls_server_end_point"
            "transport_write" "transport_read"
            "transport_close" "scram_sha256" "pg_cancel_register"
            "pg_cancel" "random_bytes"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (= ["pg-scram-message-length" "pg-scram-message-valid?"
            "pg-scram-auth-code?" "pg-scram-ascii-equal?"
            "pg-authentication-sasl-mechanism"
            "pg-authentication-sasl?"
            "pg-authentication-sasl-continue?" "pg-authentication-sasl-final?"
            "pg-scram-nonce-byte-valid?" "pg-scram-nonce-valid-from?"
            "pg-scram-client-nonce-valid?" "pg-scram-server-nonce-valid?"
            "pg-scram-base64-value" "pg-scram-base64-decode-from!"
            "pg-scram-base64-decode!" "pg-scram-base64-char"
            "pg-scram-base64-encode-from!" "pg-scram-base64-encode!"
            "pg-scram-copy!" "pg-scram-client-first-bare!"
            "pg-scram-gs2-header!" "pg-scram-client-first-for!"
            "pg-scram-client-first!" "pg-scram-client-final-without-proof-for!"
            "pg-scram-client-final-without-proof!" "pg-scram-auth-message!"
            "pg-scram-auth-message-for!" "pg-scram-client-final!"
            "pg-scram-client-final-for!" "pg-scram-store-i32-be!"
            "pg-scram-sasl-initial-for!" "pg-scram-sasl-initial!"
            "pg-scram-sasl-response-for!" "pg-scram-sasl-response!"
            "pg-scram-find-byte"
            "pg-scram-parse-iterations-from" "pg-scram-server-first-valid?"
            "pg-scram-bytes-equal-count" "pg-scram-server-final-valid?"
            "pg-scram-read-exact" "pg-scram-read-message"
            "pg-scram-authenticate" "pg-scram-parameter-status-valid?"
            "pg-scram-ready-for-query?" "pg-scram-authentication-ok?"
            "pg-scram-read-startup-tail"
            "pg-scram-i32-be" "pg-scram-read-startup-tail-cancellable"
            "pg-scram-bytes-no-zero?" "pg-scram-startup-field-valid?"
            "pg-scram-open-tls" "pg-scram-open-tls-cancellable"
            "pg-open-scram-valid" "pg-open-scram-cancellable-valid"
            "pg-open-scram-internal" "pg-open-scram"
            "pg-open-scram-random" "pg-open-scram-cancellable-random"
            "pg-cancel-authority-use" "pg-close-scram" "pg-scram-proof" "main"]
           (:kotoba.wasm/exports wasm)))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest http-consumer-compiles-to-high-level-component-imports
  (let [path "providers/http_consumer.kotoba"
        forms (runtime/read-file path :kotoba)
        policy (edn/read-string (slurp "providers/http_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[http-get]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["http_get"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm)))
        "emitted consumer is Wasm-validator clean across i64 handles")))

(deftest database-consumer-compiles-to-high-level-component-import
  (let [forms (runtime/read-file "providers/db_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[db-exchange] (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["db_exchange"] (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-session-consumer-compiles-affine-handle-flow
  (let [forms (runtime/read-file "providers/pg_session_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[db-close pg-open pg-query] (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["db_close" "pg_open" "pg_query"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-scram-consumer-composes-authentication-and-query-providers
  (let [forms (runtime/read-file "providers/pg_scram_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-query pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_query" "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-transaction-consumer-compiles-error-recovery-state-flow
  (let [forms (runtime/read-file "providers/pg_transaction_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-query-state pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_query_state" "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-prepared-consumer-compiles-parameterized-reuse-flow
  (let [forms (runtime/read-file "providers/pg_prepared_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-prepare pg-execute-params2 pg-close-statement
             pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_prepare" "pg_execute_params2" "pg_close_statement"
            "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-typed-prepared-consumer-compiles-variable-parameter-flow
  (let [forms (runtime/read-file
               "providers/pg_typed_prepared_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-prepare-typed pg-execute-params pg-close-statement
             pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_prepare_typed" "pg_execute_params" "pg_close_statement"
            "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-portal-consumer-compiles-bounded-cursor-flow
  (let [forms (runtime/read-file "providers/pg_portal_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-query-state pg-prepare pg-bind-portal pg-fetch-portal
             pg-close-portal pg-close-statement pg-open-scram-random
             pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_query_state" "pg_prepare" "pg_bind_portal"
            "pg_fetch_portal" "pg_close_portal" "pg_close_statement"
            "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-copy-consumer-compiles-bounded-in-out-flow
  (let [forms (runtime/read-file "providers/pg_copy_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-query-state pg-copy-out pg-copy-in
             pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_query_state" "pg_copy_out" "pg_copy_in"
            "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-batch-consumer-compiles-single-sync-pipeline
  (let [forms (runtime/read-file "providers/pg_batch_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-prepare pg-execute-batch pg-close-statement
             pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_prepare" "pg_execute_batch" "pg_close_statement"
            "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-pool-reset-consumer-compiles-dirty-lease-cleanup
  (let [forms (runtime/read-file "providers/pg_pool_reset_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-query-state pg-prepare pg-execute-params pg-session-reset
             pg-open-scram-random pg-close-scram]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_query_state" "pg_prepare" "pg_execute_params"
            "pg_session_reset" "pg_open_scram_random" "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-pool-consumer-imports-only-opaque-lease-operations
  (let [forms (runtime/read-file "providers/pg_pool_consumer.kotoba" :kotoba)
        policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        wasm (runtime/wasm-binary forms policy)]
    (is (= '[pg-pool-open pg-pool-acquire pg-pool-query
             pg-pool-release pg-pool-close]
           (runtime/required-host-imports forms)))
    (is (:kotoba.wasm/ok? wasm))
    (is (= ["pg_pool_open" "pg_pool_acquire" "pg_pool_query"
            "pg_pool_release" "pg_pool_close"]
           (mapv :field (:kotoba.wasm/imports wasm))))
    (is (not-any? #{"transport_connect" "tls_open" "pg_open_scram_random"
                    "pg_query_state" "pg_session_reset"}
                  (map :field (:kotoba.wasm/imports wasm))))
    (is (some? (Parser/parse ^bytes (:kotoba.wasm/binary wasm))))))

(deftest postgresql-cancellation-consumers-use-only-opaque-authority
  (let [policy (edn/read-string (slurp "providers/db_component_policy.edn"))
        query-forms (runtime/read-file
                     "providers/pg_cancellable_query_consumer.kotoba" :kotoba)
        query-wasm (runtime/wasm-binary query-forms policy)
        cancel-forms (runtime/read-file "providers/pg_cancel_consumer.kotoba" :kotoba)
        cancel-wasm (runtime/wasm-binary cancel-forms policy)]
    (is (= '[pg-query-state pg-open-scram-cancellable-random pg-close-scram]
           (runtime/required-host-imports query-forms)))
    (is (= ["pg_query_state" "pg_open_scram_cancellable_random"
            "pg_close_scram"]
           (mapv :field (:kotoba.wasm/imports query-wasm))))
    (is (= '[pg-cancel-authority-use]
           (runtime/required-host-imports cancel-forms)))
    (is (= ["pg_cancel_authority_use"]
           (mapv :field (:kotoba.wasm/imports cancel-wasm))))
    (is (:kotoba.wasm/ok? query-wasm))
    (is (:kotoba.wasm/ok? cancel-wasm))))
