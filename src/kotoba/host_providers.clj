(ns kotoba.host-providers
  "Capability-guarded host provider dispatch for the CLJ runtime slice
  (issue #263).

  Every host-import op from the kotoba core capability contract
  (clipboard_read, http_fetch, fs_read, ...) maps to a
  kotoba.lang.capability-values capability kind (:host/clipboard-read,
  :host/http, :host/fs-read, ...). At run time the launcher builds
  CACAO-style grants and a local policy from its existing policy EDN and
  dispatches each provider invocation through
  kotoba.lang.capability-host/guard-call, so:

  - a denied call NEVER reaches the provider handler (fail closed), and
  - every call — grant, denial, or handler error — leaves a receipt in the
    run's audit journal (surfaced as :kotoba.host/receipts in the launcher
    result).

  Legacy behavior is preserved: a `run` without `--policy` installs no guard
  and host-import ops remain statically rejected as :capability-not-granted,
  exactly as before. Enforcement happens only when a capability policy is
  supplied.

  Policy EDN vocabulary (superset of the existing wasm/provider policy):

  {:kotoba.policy/capabilities #{:clipboard/text :http/fetch ...}
   ;; optional per-capability resource scope (default :any)
   :kotoba.policy/capability-resources {:clipboard/text #{\"clipboard:system\"}}
   ;; optional per-capability grant expiry, enforced at call time
   :kotoba.policy/capability-expires {:clipboard/text \"2027-01-01\"}}

  The default handlers are deterministic Rust-free stubs (the interpreter
  slice has no real memory ABI); concrete native providers (pbcopy/pbpaste,
  an HTTP client, ...) plug in by passing a :handlers map to `host-call`."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as cstr]
            [json.core :as json]
            [kotoba.cap-table :as cap-table]
            [kotoba.core.contracts :as core-contracts]
            [kotoba.kgraph :as kgraph]
            [kotoba.lang.capability-host :as capability-host]
            [kotoba.lang.capability-values :as capability-values]
            [kotoba.security.information-flow :as flow]
            [kotoba.runtime :as runtime]))

(def op->kind
  "Host-import op (capability contract symbol) -> capability kind understood
  by kotoba.lang.capability-values/effect-for-kind. Owned by kotoba.runtime
  (the static capability gate needs it too); aliased here for provider code."
  runtime/op->kind)

(defn op-capability
  "Contract capability name (e.g. \"clipboard/text\") for a host-import op."
  [op]
  (get-in runtime/host-imports [op :capability]))

(defn- named-lookup
  "Look up CAP-NAME in a policy map keyed by capability keyword or string."
  [m cap-name]
  (some (fn [[k v]]
          (when (= cap-name (core-contracts/capability-name k))
            v))
        m))

(def network-cap-names
  "Contract capability names that are network egress. Safe default: a
  missing `:kotoba.policy/capability-resources` entry fails closed
  (empty set) unless the policy explicitly sets
  `:kotoba.policy/http-require-allowlist false` (legacy opt-out)."
  ;; web-wide crawl (com-junkawasaki/root ADR-2607252400): cdx 照会と WARC 取得は
  ;; commoncrawl.org への egress、corpus/publish は B2 への egress。ここに載せる
  ;; ことで :kotoba.policy/capability-resources の allowlist が無い限り
  ;; resource-scope が #{} を返して **全 URL 拒否** になる(fail closed)。
  ;; これが「egress は commoncrawl.org のみ / push 先は product-corpus のみ」を
  ;; policy で強制できる根拠で、汎用 shell-exec capability では表現できない。
  ;;
  ;; ⚠ この集合は「どの capability が egress か」という分類を **手で複製した
  ;; もの**で、正本ではない。capability contract
  ;; (kotoba-core-contracts の capability_contract.edn) は id と ABI を持つが
  ;; egress かどうかを持たないので、新しい egress capability を足した人が
  ;; ここに書き忘れると resource-scope が #{} ではなく #{:any} を返す ——
  ;; **ドリフトの失敗方向が fail-open** である。実際 2026-08-10 に
  ;; net/connect (contract id 238) と component/http (241) と
  ;; component/database (242) が漏れていて、resources 無しの policy が
  ;; 全ホストへの接続を得ていた。撤去条件: 分類が capability contract 側に
  ;; 入って、ここが contract の導出値になったとき。
  #{"http/fetch" "http/post"
    "cc/cdx-query" "cc/warc-extract" "corpus/publish"
    ;; 接続を張る側だけを載せる。net/transport と crypto/tls は既に許可された
    ;; handle の上で動くので、ここで #{} にしても許可の判断は増えず、
    ;; transport を使う全構成が壊れるだけになる。
    "net/connect" "component/http" "component/database"})

(defn http-require-allowlist?
  "True when POLICY requires network resource allowlists (safe default).
  Only an explicit `false` opts out."
  [policy]
  (not (false? (:kotoba.policy/http-require-allowlist policy))))

(defn normalize-policy
  "Apply safe-runtime defaults to a loaded policy map. Currently stamps
  `:kotoba.policy/http-require-allowlist true` when the key is absent so
  operators see the effective policy in receipts/debug."
  [policy]
  (cond
    (nil? policy) nil
    (contains? policy :kotoba.policy/http-require-allowlist) policy
    :else (assoc policy :kotoba.policy/http-require-allowlist true)))

(defn- resource-scope
  "Grant resource set for CAP-NAME under POLICY. Network caps default to
  empty (deny all URLs) unless the operator opts out with
  `:kotoba.policy/http-require-allowlist false` or supplies an explicit
  `:kotoba.policy/capability-resources` allowlist."
  [policy cap-name]
  (let [resources (named-lookup (:kotoba.policy/capability-resources policy)
                                cap-name)]
    (cond
      (nil? resources)
      (if (and (contains? network-cap-names cap-name)
               (http-require-allowlist? policy))
        #{}
        #{:any})

      (= :any resources) #{:any}
      :else (set resources))))

(defn- grant-expiry
  [policy cap-name]
  (named-lookup (:kotoba.policy/capability-expires policy) cap-name))

(defn- granted-kinds
  "Seq of [kind cap-name] for every guarded kind whose contract capability
  appears in the policy's :kotoba.policy/capabilities."
  [policy]
  (let [caps (core-contracts/policy-capabilities policy)]
    (distinct
     (keep (fn [[op kind]]
             (let [cap-name (op-capability op)]
               (when (and cap-name (contains? caps cap-name))
                 [kind cap-name])))
           op->kind))))

(defn policy-grants
  "CACAO-style grants derived from the launcher policy EDN: one grant per
  guarded host kind enabled by :kotoba.policy/capabilities, scoped by
  :kotoba.policy/capability-resources and expiring per
  :kotoba.policy/capability-expires."
  [policy]
  (vec (for [[kind cap-name] (granted-kinds policy)]
         {:grant/kind kind
          :grant/resources (resource-scope policy cap-name)
          :grant/expires (grant-expiry policy cap-name)
          :grant/id (str "policy:" cap-name)})))

(defn local-policy
  "kotoba.lang.capability-values local policy derived from the launcher
  policy EDN. Propagates :kotoba.policy/forbid-wildcard (S4b least-privilege)."
  [policy]
  (cond-> {:policy/allow
           (into {} (for [[kind cap-name] (granted-kinds policy)]
                      [kind (resource-scope policy cap-name)]))}
    (true? (get policy :kotoba.policy/forbid-wildcard))
    (assoc :policy/forbid-wildcard true)))

(defn grants->policy
  "Launcher policy EDN synthesized from externally supplied (CACAO chain)
  grants — used when `run --cacao` is given WITHOUT `--policy`. The static
  capability gate then admits exactly the contract capabilities whose host
  kinds the chain grants, and the derived local policy allows :any for those
  kinds (no `:kotoba.policy/capability-resources` narrowing): the local
  policy remains the narrowing side, and absent an explicit `--policy` it
  defaults to allowing whatever the chain grants. The grants themselves stay
  the authorization side of the intersection."
  [grants]
  (let [kinds (into #{} (map :grant/kind) grants)]
    {:kotoba.policy/capabilities
     (into #{} (keep (fn [[op kind]]
                       (when (contains? kinds kind)
                         (op-capability op))))
           op->kind)}))

(defn- canonical-path
  "Best-effort canonicalized (symlink/`..`-resolved) path string for S, used
  ONLY for capability resource-scope comparison below -- the actual fs-read/
  fs-write I/O always runs against the guest's ORIGINAL path string via
  `io/file`, exactly like a native fs call would resolve it against the JVM
  process's real cwd. Falls back to the raw string when canonicalization
  itself throws (e.g. `fs-write`'s target parent directory doesn't exist
  yet)."
  [^String s]
  (try (.getCanonicalPath (io/file s)) (catch Exception _ s)))

(defn- fs-path-permitted?
  "True when PATH (the guest-supplied literal fs-read/fs-write path
  argument) is inside CONCRETE's :cap/resource scope.
  kotoba.lang.capability-values/intersect-grants (run by guard-call, inside
  `host-call` below) only ever requests the UNIVERSAL `:any` capability for
  a host KIND -- the guest's actual path argument isn't known until AFTER
  the guard decision runs, so a policy that scopes `:fs/app-data` to
  specific paths (`:kotoba.policy/capability-resources`) would otherwise be
  silently ignored once the capability kind is granted at all. This extra,
  per-call check is what actually enforces that narrowing; it mirrors
  kotoba.wasm-exec's `resource-permitted?` (identical policy vocabulary:
  `:any`, a bare resource string, or a set of them) -- duplicated rather
  than shared because kotoba.wasm-exec already requires this namespace, so
  the reverse require would be circular. Comparison is on the canonicalized
  path so a granted resource string can't be trivially defeated by a
  `./`-prefixed or symlinked spelling of the same file.

  kbb slice 2 (ADR-2607181900): a scope entry that names a DIRECTORY also
  admits the files strictly beneath it (prefix boundary on a path
  separator, same rule as `fs-dir-permitted?`). This keeps the kbb verify
  gate's grant shape uniform — one granted directory covers
  fs-browse(dir) + fs-read(dir/file) — while a per-FILE scope entry keeps
  its exact per-file equality semantics. Escaping via `dir/..` is
  impossible: both sides are canonicalized before the prefix compare."
  [concrete path]
  (let [scope (:cap/resource concrete)]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (let [target (canonical-path path)
               scopes (cond
                        (string? scope) [scope]
                        (set? scope) scope
                        :else [])]
           (some (fn [s]
                   (let [s (canonical-path s)]
                     (or (= s target)
                         (cstr/starts-with? target (str s java.io.File/separator)))))
                 scopes))))))

(defn- fs-check-permitted!
  "Throws (fail closed) when PATH is outside CONCRETE's granted resource
  scope. Called from inside an already-GRANTED fs-read/fs-write handler
  (guard-call's own kind-level grant check already ran and passed) -- this
  is the finer per-path check `fs-path-permitted?` documents. Thrown from
  inside the :handler fn passed to guard-call, so `capability-host/guard-call`
  itself records the normal :error receipt and rethrows -- no separate
  receipt shape to invent here."
  [op concrete path]
  (when-not (fs-path-permitted? concrete path)
    (throw (ex-info (str op ": path outside granted capability resource scope")
                     {:kotoba.host/denied :resource-not-permitted
                      :kotoba.host/call op
                      :kotoba.host/path path}))))

(defn- env-name-permitted?
  "True when NAME (the guest-supplied literal env-read variable name
  argument) is inside CONCRETE's :cap/resource scope. Same per-call
  narrowing rationale as `fs-path-permitted?` above: the guard decision
  runs on the capability KIND before the guest's actual argument exists,
  so this per-call check is what enforces the policy's env var NAME
  allowlist. Env var names are compared EXACTLY (case-sensitive, no
  canonicalization) -- an allowlist of \"PATH\" must not silently admit
  \"path\" or \"Path\"."
  [concrete name]
  (let [scope (:cap/resource concrete)]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (cond
           (string? scope) (= scope name)
           (set? scope) (contains? scope name)
           :else false)))))

(defn- env-check-permitted!
  "Throws (fail closed) when NAME is outside CONCRETE's granted resource
  scope. Called from inside an already-GRANTED env-read handler, mirroring
  `fs-check-permitted!` above; thrown from inside the :handler fn passed
  to guard-call, so the normal :error receipt records it."
  [concrete name]
  (when-not (env-name-permitted? concrete name)
    (throw (ex-info "env-read: variable name outside granted capability resource scope"
                    {:kotoba.host/denied :resource-not-permitted
                     :kotoba.host/call 'env-read
                     :kotoba.host/name name}))))

(defn- http-url-permitted?
  "True when URL (the guest-supplied literal http-fetch argument) is inside
  CONCRETE's granted resource scope. Scope entries are URL PREFIXES
  (e.g. \"https://api.github.com/\"): the URL must start with one of them.
  Prefix matching is exact string starts-with -- no wildcard, no
  normalization, so a scope of \"https://api.github.com/\" admits
  \"https://api.github.com/repos\" but not \"https://api.github.com.evil.io\"
  (prefix boundary falls inside the URL string, which the operator chose).
  A scope of :any (universal grant) admits everything -- the same meaning
  resource-scope :any has for the other kinds."
  [concrete url]
  (let [scope (:cap/resource concrete)]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (cond
           (string? scope) (clojure.string/starts-with? url scope)
           (set? scope) (some #(clojure.string/starts-with? url %) scope)
           :else false)))))

(defn- http-check-permitted!
  "Throws (fail closed) when URL is outside CONCRETE's granted resource
  scope. Called from inside an already-GRANTED http-fetch handler; thrown
  from inside the :handler fn passed to guard-call, so the normal :error
  receipt records it."
  [concrete url]
  (when-not (http-url-permitted? concrete url)
    (throw (ex-info "http-fetch: url outside granted capability resource scope"
                    {:kotoba.host/denied :resource-not-permitted
                     :kotoba.host/call 'http-fetch
                     :kotoba.host/url url}))))

(defn- proc-command-permitted?
  "True when COMMAND (the resolved command name for the guest-supplied grant
  INDEX) is inside CONCRETE's :cap/resource scope. Same per-call narrowing
  rationale as `env-name-permitted?`: the guard decision runs on the
  capability KIND before the resolved command name exists. Command names
  are compared EXACTLY (no canonicalization, no path expansion)."
  [concrete command]
  (let [scope (:cap/resource concrete)]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (cond
           (string? scope) (= scope command)
           (set? scope) (contains? scope command)
           :else false)))))

(defn- proc-check-permitted!
  "Throws (fail closed) when the invocation resolved from the guest-supplied
  grant INDEX is outside CONCRETE's granted resource scope. Called from
  inside an already-GRANTED proc-exec handler; thrown from inside the
  :handler fn passed to guard-call, so the normal :error receipt records it."
  [concrete command]
  (when-not (proc-command-permitted? concrete command)
    (throw (ex-info "proc-exec: command outside granted capability resource scope"
                    {:kotoba.host/denied :resource-not-permitted
                     :kotoba.host/call 'proc-exec
                     :kotoba.host/command command}))))
(defn- fs-dir-permitted?
  "True when DIR (the guest-supplied literal fs-browse directory argument)
  equals CONCRETE's granted resource scope, or is a strict descendant of a
  granted scope directory. Unlike fs-read/fs-write (per-FILE path
  equality), browse grants name a directory tree: a scope of \"src\"
  admits \"src\", \"src/kotoba\", ... but NOT \"srcs\" (prefix boundary must
  fall on a path separator) and not \"..\" (canonicalized first, so a
  granted \"src\" cannot be escaped via \"src/..\"). Comparison runs on
  canonicalized paths exactly like `fs-path-permitted?`."
  [concrete dir]
  (let [scope (:cap/resource concrete)]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (let [target (canonical-path dir)
               scopes (cond
                        (string? scope) [scope]
                        (set? scope) scope
                        :else [])]
           (some (fn [s]
                   (let [s (canonical-path s)]
                     (or (= s target)
                         (cstr/starts-with? target (str s java.io.File/separator)))))
                 scopes))))))

(defn- fs-browse-check-permitted!
  "Throws (fail closed) when DIR is outside CONCRETE's granted directory
  scope. Called from inside an already-GRANTED fs-browse handler."
  [concrete dir]
  (when-not (fs-dir-permitted? concrete dir)
    (throw (ex-info "fs-browse: directory outside granted capability resource scope"
                    {:kotoba.host/denied :resource-not-permitted
                     :kotoba.host/call 'fs-browse
                     :kotoba.host/dir dir}))))

(defn- json-permitted?
  "True when the guest-supplied JSON OP-ARGUMENT (a field name for
  json-extract-field; json-encode takes no external resource) is inside
  CONCRETE's :cap/resource scope. Same per-call narrowing rationale as
  `env-name-permitted?` above: the guard decision runs on the capability
  KIND before the guest's actual argument exists. Field names are compared
  EXACTLY. json-encode builds output FROM guest-supplied pairs, so its
  authorization surface is the capability kind alone; json-extract-field
  reads out of a guest-supplied buffer but the FIELD allowlist bounds what
  may be pulled out (the contract's bounded string scan), so a scope entry
  narrows the extractable field NAMES."
  [concrete field]
  (let [scope (:cap/resource concrete)]
    (boolean
     (or (nil? concrete)
         (= :any scope)
         (cond
           (string? scope) (= scope field)
           (set? scope) (contains? scope field)
           :else false)))))

(defn- json-check-permitted!
  "Throws (fail closed) when FIELD is outside CONCRETE's granted resource
  scope. Called from inside an already-GRANTED json-extract-field handler,
  mirroring `env-check-permitted!`; thrown from inside the :handler fn
  passed to guard-call, so the normal :error receipt records it."
  [concrete field]
  (when-not (json-permitted? concrete field)
    (throw (ex-info "json-extract-field: field outside granted capability resource scope"
                    {:kotoba.host/denied :resource-not-permitted
                     :kotoba.host/call 'json-extract-field
                     :kotoba.host/field field}))))

(defn- json-encode-handler
  "data/json (capability id 246) json-encode provider. Takes the contract's
  flat wire buffer AS A STRING (key<TAB>value pairs, one per LF-separated
  line, dotted-path keys addressing one level of object/array nesting --
  exactly the format the contract documents) and returns the JSON object
  text. A malformed buffer (a line with no TAB, or an empty key) fails
  closed with an ex-info -- receipted as :error -- never a partially-built
  object. No ambient input: every byte comes from the guest's own
  arguments, so the resource scope does not narrow this op (the KIND-level
  grant is the whole authorization)."
  [_concrete args]
  (let [pairs (str (first args))
        insert (fn insert [m path v]
                 (if (= 1 (count path))
                   (assoc m (first path) v)
                   (let [k (first path)]
                     (assoc m k (insert (get m k {}) (rest path) v)))))]
    (try
      (let [obj (reduce (fn [m line]
                          (if (pos? (count (cstr/trim line)))
                            (let [tab (cstr/index-of line "	")]
                              (when (or (nil? tab) (zero? tab))
                                (throw (ex-info "json-encode: malformed pairs buffer"
                                                {:kotoba.host/call 'json-encode})))
                              (insert m (cstr/split (subs line 0 tab) #"\.")
                                      (subs line (inc tab))))
                            m))
                        {} (cstr/split pairs #"
"))]
        ;; canonical emit: keys sorted (deterministic bytes; the guest
        ;; must not depend on JVM map iteration order)
        #_:clj-kondo/ignore
        (let [sort-keys (fn sort-keys [m]
                          (into (sorted-map)
                                (map (fn [[k v]]
                                       [k (cond
                                            (map? v) (sort-keys v)
                                            :else v)]))
                                m))]
          (json/encode (sort-keys obj))))
      (catch Exception e
        (if (:kotoba.host/call (ex-data e))
          (throw e)
          (throw (ex-info (str "json-encode: " (ex-message e))
                          {:kotoba.host/call 'json-encode
                           :kotoba.host/cause (ex-message e)})))))))

(def default-handlers
  "Deterministic Rust-free provider stubs, keyed by host-import op, EXCEPT
  fs-read/fs-write and clock-monotonic below, which are real (issue #263 v0.1
  slice, ADR-2607182430 in the com-junkawasaki/root superproject): fs-read
  really reads a file's bytes off disk (as a UTF-8 string) and fs-write
  really writes one, both gated a second time by `fs-check-permitted!`
  above; clock-monotonic is a real System/nanoTime read (no filesystem/
  network surface to gate beyond the capability KIND itself). Each handler
  is (fn [concrete-cap args] result); args here are plain literal Kotoba
  values (a path string, file content), NOT the (ptr,len) WASM-ABI shape --
  see `kgraph-handlers`' docstring for why the CLJ interpreter slice uses
  this convention (it has no linear memory to marshal a ptr/len pair
  through). host-i64-roundtrip echoes its argument (matching the
  interpreter builtin); the remaining pointer-ABI providers return 0 (the
  success status of the provider result ABI) since the interpreter has no
  real memory to read a ptr/len pair out of — `str-ptr` always evaluates to
  0 here (kotoba.runtime/builtin-fns). Real native providers for those (or,
  for kgraph-*, `kgraph-handlers` below / kotoba.wasm-exec's Chicory host
  functions for genuine WASM execution, which ALREADY has real fs-read/
  fs-write/http-fetch/clipboard/keychain/etc. behind a sandboxed fs-root —
  see kotoba.wasm-exec/real-op-effects, a separate execution path from this
  interpreter slice) replace these via the :handlers option of `host-call`."
  {'notify-show (fn [_cap _args] 0)
   'clipboard-read (fn [_cap _args] 0)
   'clipboard-write (fn [_cap _args] 0)
   'clipboard-write-str (fn [_cap _args] 0)
   ;; kbb ops-script surface slice 3 (ADR-2607181900 readiness gate): real
   ;; GET fetch, URL allowlisted by the policy's resource scope (URL
   ;; prefixes). Mirrors kotoba.wasm-exec's real http-fetch semantics
   ;; (java.net.http.HttpClient GET) but in interpreter mode: the guest
   ;; passes the URL literal and receives the response BODY as a string
   ;; (the same string-result convention as fs-read). Non-2xx answers and
   ;; transport failures return the sentinel -1 (the guest sees a failure
   ;; VALUE, not an exception). ⚠ -1 is TRUTHY in this language — guests
   ;; must distinguish success with `string?` (the body is a string, the
   ;; sentinel is a number), never with a bare `if`. A URL outside the
   ;; granted prefixes fails closed BEFORE any network I/O happens.
   'http-fetch (fn [concrete args]
                 (let [url (str (first args))]
                   (http-check-permitted! concrete url)
                   (try
                     (let [req (-> (java.net.http.HttpRequest/newBuilder
                                    (java.net.URI/create url))
                                   (.timeout java.time.Duration/ofSeconds 30)
                                   (.GET)
                                   (.build))
                           resp (.send (java.net.http.HttpClient/newHttpClient)
                                       req
                                       (java.net.http.HttpResponse$BodyHandlers/ofString))]
                       (if (<= 200 (.statusCode resp) 299)
                         (.body resp)
                         -1))
                     (catch Exception _ -1))))
   'keychain-read (fn [_cap _args] 0)
   'keychain-write (fn [_cap _args] 0)
   'fs-read (fn [concrete args]
              (let [path (first args)]
                (fs-check-permitted! 'fs-read concrete path)
                (let [f (io/file path)]
                  (when (.isFile f)
                    (slurp f)))))
   'fs-write (fn [concrete args]
               (let [[path content] args]
                 (fs-check-permitted! 'fs-write concrete path)
                 (let [f (io/file path)]
                   (when-let [parent (.getParentFile f)]
                     (.mkdirs parent))
                   (spit f (str content))
                   (count (str content)))))
   'host-i64-roundtrip (fn [_cap args] (first args))
   ;; web-wide crawl (ADR-2607252400)。このインタプリタスライスには実メモリが
   ;; 無く str-ptr が常に 0 なので、他の ptr/len ABI provider と同じく決定論
   ;; スタブ。実装は kotoba.wasm-exec の Chicory host functions 側(sandboxed
   ;; fs-root 付きの実 fs-read/http-fetch と同じ経路)に置き、:handlers で差す。
   ;; **ここを実装で埋めない**のは、guard-call を通らない裏口を作らないため。
   'cc-cdx-query (fn [_cap _args] 0)
   'cc-warc-extract (fn [_cap _args] 0)
   'corpus-append (fn [_cap _args] 0)
   'corpus-publish (fn [_cap _args] 0)
   'kgraph-assert! (fn [_cap _args] 0)
   'kgraph-retract! (fn [_cap _args] 0)
   'kgraph-get-objects (fn [_cap _args] 0)
   'kgraph-query (fn [_cap _args] 0)
   ;; aiueos default kernel capabilities (ADR-2607022700) -- deterministic
   ;; stubs like every other provider here; a native aiueos kototama adapter
   ;; (wasmtime hosting, real MMIO/DMA/IRQ) plugs in via :handlers, never by
   ;; editing these defaults. i64-result ops return 0 (a null/no-op
   ;; sentinel, same convention as the ptr/len ABI's 0-status stubs above).
   'log-write (fn [_cap _args] 0)
   'clock-monotonic (fn [_cap _args] (System/nanoTime))
   'random-bytes (fn [_cap _args] 0)
   ;; kbb ops-script surface (ADR-2607181900 readiness gate): real named
   ;; env var read, per-name narrowed by `env-check-permitted!` above. A
   ;; variable that is granted but unset returns nil (the guest sees an
   ;; absent value, same as a native getenv miss); a NON-granted name
   ;; fails closed before the lookup ever happens.
   'env-read (fn [concrete args]
               (let [name (first args)]
                 (env-check-permitted! concrete name)
                 (System/getenv (str name))))
   ;; kbb ops-script surface slice 2 (ADR-2607181900 readiness gate): real
   ;; process exec of ONE allowlisted invocation. The guest passes a grant
   ;; INDEX (a decimal string) into the policy's fixed invocation table;
   ;; argv, cwd, and timeout are POLICY-side literals -- the guest supplies
   ;; zero command bytes and can never compose an unlisted invocation. The
   ;; handler needs the policy's table, which host-call threads through
   ;; :kotoba.policy/*; the closure reads it from the dynamic binding set
   ;; by guarded-host-call (see host-call below). No shell is ever spawned:
   ;; argv goes straight to ProcessBuilder, arg-by-arg.
'proc-exec (fn [_concrete _args] 0)
   ;; data/json (capability id 246, kotoba-core-contracts): real JSON
   ;; wire-format handlers backed by the JVM's json.data-json
   ;; (cheshire-compatible) parser/encoder -- NOT the interpreter's stub
   ;; convention. The interpreter slice has no linear memory, so like
   ;; fs-browse/fs-read these take plain values instead of the contract's
   ;; (ptr,len) ABI:
   ;;   json-encode takes the contract's flat wire buffer AS A STRING
   ;;   (key<TAB>value pairs, one per LF-separated line, dotted-path keys
   ;;   addressing one level of nesting -- exactly the format the contract
   ;;   documents) and returns the JSON object text; a malformed buffer
   ;;   fails closed with an ex-info (receipted as :error), never a
   ;;   partially-written buffer. No ambient input: every byte comes from
   ;;   the guest's own arguments.
   ;;   json-extract-field takes (json-string field-name) and returns the
   ;;   extracted string value or nil (field not found), narrowed per call
   ;;   to the granted FIELD NAMES by `json-check-permitted!` -- the
   ;;   contract's bounded `"field":"value"` scan, fail closed.
   'json-encode json-encode-handler
   'json-extract-field (fn [concrete args]
                         (let [[json-text field] args
                               field (str field)]
                           (json-check-permitted! concrete field)
                           (let [m (json/decode (str json-text))]
                             (when (map? m)
                               (get m field)))))
   ;; kbb ops-script surface (ADR-2607181900 readiness gate slice 2): real
   ;; directory listing, narrowed to the granted directory TREE by
   ;; `fs-browse-check-permitted!` above (scope = the directory itself or
   ;; any descendant, prefix boundary on a path separator). Returns the
   ;; entry NAMES of a directory as a vector of strings (not paths); a
   ;; granted-but-missing directory returns nil (absent, like env-read's
   ;; unset miss). The result flows through the interpreter slice's plain
   ;; Clojure values, so the guest can (count ...) it and index it —
   ;; pairs/vectors are interpreter-native. Bounded: one directory per
   ;; call, names only, no recursion (the guest recurses through fs-browse
   ;; + fs-read, which keeps every step inside the capability guard).
   'fs-browse (fn [concrete args]
                (let [dir (first args)]
                  (fs-browse-check-permitted! concrete dir)
                  (let [f (io/file dir)]
                    (when (.isDirectory f)
                      (vec (.list f))))))
   'topic-publish (fn [_cap _args] 0)
   'topic-poll (fn [_cap _args] 0)
   'topic-take (fn [_cap _args] 0)
   'topic-count (fn [_cap _args] 0)
   'pci-config (fn [_cap _args] 0)
   'dma-map (fn [_cap _args] 0)
   'irq-subscribe (fn [_cap _args] 0)
   'mmio-map (fn [_cap _args] 0)})

(defn kgraph-handlers
  "Real (non-stub) interpreter-mode kgraph-* handlers backed by STORE (an
  atom of `kotoba.kgraph` datoms; a fresh one per call when omitted). Unlike
  the memory-ABI-oriented default stubs, these read their FIRST argument as a
  literal EDN string directly (the interpreter has no linear memory to marshal
  a ptr/len pair through, so callers pass the request/query EDN as a plain
  string literal instead of `(str-ptr ...)`). Pass as `:handlers` to
  `host-call`/`guarded-run-result` to exercise the real store from `run`
  without going through kotoba.wasm-exec's WASM/Chicory path."
  ([] (kgraph-handlers (atom [])))
  ([store]
   (merge default-handlers
          {'kgraph-assert! (fn [_cap [datom-edn]]
                             (swap! store kgraph/assert-datom (edn/read-string datom-edn))
                             0)
           'kgraph-retract! (fn [_cap [datom-edn]]
                              (swap! store kgraph/retract-datom (edn/read-string datom-edn))
                              0)
           'kgraph-get-objects (fn [_cap [entity-edn]]
                                 (pr-str (kgraph/get-objects @store (edn/read-string entity-edn))))
           'kgraph-query (fn [_cap [query-edn]]
                           (pr-str (kgraph/query @store (edn/read-string query-edn))))})))

(defn journal
  "Append-only receipt recorder ({:record! fn :entries fn}) for a guarded run."
  []
  (capability-host/journal))

(defn capability-query-fn
  "Interpreter binding for `has-capability?`: a policy lookup, not a host
  effect, so it is not receipted."
  [policy]
  (let [caps (core-contracts/policy-capabilities policy)]
    (fn [cap]
      (contains? caps (core-contracts/capability-name cap)))))

(defn proc-exec-handler
  "Real proc-exec handler for kbb slice 2 (ADR-2607181900 readiness gate):
  runs ONE allowlisted invocation. INVOCATIONS is the policy's fixed
  invocation table (a vector of {:command :argv :cwd :timeout-seconds});
  the guest passes a grant INDEX (decimal string) into it -- the guest
  supplies zero command bytes and can never compose an unlisted
  invocation. argv goes straight to ProcessBuilder arg-by-arg: no shell
  is ever spawned. The command name is re-checked against the concrete
  cap's resource scope (fail closed) before exec."
  [invocations]
  (fn [concrete args]
    (let [idx-str (str (first args))
          idx (parse-long idx-str)
          {:keys [command argv cwd timeout-seconds]}
          (when idx (nth invocations idx nil))]
      (when (nil? command)
        (throw (ex-info "proc-exec: grant index out of range"
                        {:kotoba.host/denied :resource-not-permitted
                         :kotoba.host/call 'proc-exec
                         :kotoba.host/index idx-str})))
      (proc-check-permitted! concrete command)
      (let [pb (java.lang.ProcessBuilder. ^java.util.List (vec argv))]
        (when cwd (.directory pb (java.io.File. ^String cwd)))
        (.redirectErrorStream pb true)
        (let [p (.start pb)
              done (.waitFor p
                             (or timeout-seconds 30)
                             java.util.concurrent.TimeUnit/SECONDS)]
          (if-not done
            (do (.destroyForcibly p)
                (throw (ex-info "proc-exec: timeout"
                                {:kotoba.host/call 'proc-exec
                                 :kotoba.host/command command})))
            {:exit (.exitValue p)
             :stdout (slurp (.getInputStream p))}))))))

(defn host-call
  "Build the guarded host-call fn handed to kotoba.runtime/run:
  (fn [op args] result). Every invocation goes through
  kotoba.lang.capability-host/guard-call with grants/policy derived from the
  launcher policy EDN; receipts flow to :record! (see `journal`). A denial
  throws ex-info carrying :kotoba.host/denied, :kotoba.host/call, and the
  denial :kotoba.host/receipt — the provider handler is never invoked.

  When OPTS carries :cacao-grants (verified CACAO delegation-chain grants,
  see kotoba.lang.capability-cacao), those REPLACE the policy-derived grants
  in the intersection; the local policy side still derives from POLICY, so an
  explicit policy narrows the chain's resource set."
  ([policy] (host-call policy nil))
  ([policy {:keys [record! now handlers cacao-grants information-flow-context]}]
   (let [grants (or cacao-grants (policy-grants policy))
         allow (local-policy policy)
         now (or now (str (java.time.LocalDate/now)))
         information-flow-context
         (or information-flow-context (:kotoba.policy/information-flow policy))
         ;; kbb slice 2 (ADR-2607181900): the policy's fixed invocation
         ;; table closes over the proc-exec handler, so the guest never
         ;; carries command bytes -- it names a grant INDEX.
         invocations (:kotoba.policy/proc-exec-invocations policy)
         handlers (or handlers
                      (if invocations
                        (assoc default-handlers
                               'proc-exec (proc-exec-handler invocations))
                        default-handlers))]
     (fn guarded-host-call [op args]
       (let [kind (get op->kind op)
             handler (get handlers op)]
         (when-not (and kind handler)
           (throw (ex-info "host op has no capability guard"
                           {:kotoba.host/op op})))
         (when information-flow-context
           (let [decision (flow/evaluate-egress
                           (assoc information-flow-context :now
                                  (or (:now information-flow-context) now)))]
             (when-not (:information-flow/allowed? decision)
               (let [receipt {:receipt/call (keyword "kotoba.host" (str op))
                              :receipt/outcome :denied
                              :receipt/denied :information-flow
                              :receipt/information-flow decision}]
                 (when record! (record! receipt))
                 (throw (ex-info "host call denied by information-flow policy"
                                 {:kotoba.host/denied :information-flow
                                  :kotoba.host/call op
                                  :kotoba.host/receipt receipt}))))))
         (let [outcome (capability-host/guard-call
                        {:call (keyword "kotoba.host" (str op))
                         :requested (capability-values/make-cap kind :any)
                         :cacao-grants grants
                         :local-policy allow
                         :now now
                         :record! record!
                         :handler (fn [concrete] (handler concrete (vec args)))})]
           (if (:kotoba.host/ok? outcome)
             (:kotoba.host/result outcome)
             (throw (ex-info "host call denied by capability guard"
                             {:kotoba.host/denied (:kotoba.host/denied outcome)
                              :kotoba.host/call op
                              :kotoba.host/receipt (:kotoba.host/receipt outcome)})))))))))

;; ---------------------------------------------------------------------------
;; Capability-passing (S4b): cap-acquire + <op>-with use variants

(defn- use-receipt
  [concrete now call outcome handle extra]
  (merge (assoc (capability-values/receipt concrete now call)
                :receipt/outcome outcome
                :receipt/cap-handle handle)
         extra))

(defn host-call-with
  "Build the capability-passing host-call fn: (fn [base-op handle args] result).

  HANDLE must have been issued by kotoba.cap-table/acquire! on TABLE. The
  stored capability IS the intersected one, so no re-intersection happens at
  use time; expiry is re-checked against :now, and the capability kind must
  match the op (kotoba.cap-table/resolve-use). Provider handlers are looked
  up by the BASE op — a `<op>-with` call reaches the same provider as `<op>`,
  only the authorization path differs. Every use — grant, denial (unknown
  handle, kind mismatch, expiry), or handler error — leaves a receipt
  carrying :receipt/cap-handle; :receipt/call is the `<op>-with` surface.
  A denial throws ex-info with :kotoba.host/denied (fail closed, the provider
  handler is never invoked)."
  [table {:keys [record! now handlers]}]
  (let [now (or now (str (java.time.LocalDate/now)))
        handlers (or handlers default-handlers)]
    (fn guarded-host-call-with [op handle args]
      (let [kind (get op->kind op)
            handler (get handlers op)
            with-op (get runtime/op->with-op op)]
        (when-not (and kind handler with-op)
          (throw (ex-info "host op has no capability guard"
                          {:kotoba.host/op op})))
        (let [call (keyword "kotoba.host" (str with-op))
              ;; consume-use!: handle is affine at runtime (S2 defense-in-depth).
              resolved (cap-table/consume-use! table handle kind now)]
          (if-not (:ok? resolved)
            (let [receipt (use-receipt (cap-table/resolve-cap table handle)
                                       now call :denied handle
                                       {:receipt/denied (:denied resolved)})]
              (when record! (record! receipt))
              (throw (ex-info "capability handle rejected at host-call time"
                              {:kotoba.host/denied (:denied resolved)
                               :kotoba.host/call with-op
                               :kotoba.host/receipt receipt})))
            (let [concrete (:cap resolved)
                  invoked (try
                            {:value (handler concrete (vec args))}
                            (catch Exception e
                              {:error e}))]
              (if (contains? invoked :error)
                (let [e (:error invoked)
                      receipt (use-receipt concrete now call :error handle
                                           {:receipt/error (or (ex-message e) (str e))})]
                  (when record! (record! receipt))
                  (throw e))
                (let [receipt (use-receipt concrete now call :ok handle nil)]
                  (when record! (record! receipt))
                  (:value invoked))))))))))

(defn capability-passing-fns
  "Interpreter bindings for the S4b capability-passing surface: 'cap-acquire
  plus every '<op>-with use variant (kotoba.runtime/op->with-op). Handles are
  per-run, issued and resolved against TABLE (kotoba.cap-table/make-table).

  (cap-acquire <kind-kw> <resource>) intersects policy ∩ grants ∩ requested
  ONCE and returns the handle; a denial at acquisition throws the same
  ex-info shape as a denied host call (:kotoba.host/denied, so the run fails
  closed with a :host-call-denied problem and :kotoba.runtime/call
  :cap/acquire). (<op>-with <handle> <args...>) resolves the handle through
  `host-call-with` above. As with `host-call`, OPTS :cacao-grants replaces
  the policy-derived grants at acquisition time."
  [table policy {:keys [record! now cacao-grants] :as opts}]
  (let [now (or now (str (java.time.LocalDate/now)))
        opts (assoc opts :now now)
        grants (or cacao-grants (policy-grants policy))
        allow (local-policy policy)
        call-with (host-call-with table opts)
        acquire (fn cap-acquire [kind resource]
                  (let [outcome (cap-table/acquire! table {:kind kind
                                                           :resource resource
                                                           :grants grants
                                                           :policy allow
                                                           :now now
                                                           :record! record!})]
                    (if (:kotoba.host/ok? outcome)
                      (:kotoba.host/result outcome)
                      (throw (ex-info "capability acquisition denied"
                                      {:kotoba.host/denied (:kotoba.host/denied outcome)
                                       :kotoba.host/call :cap/acquire
                                       :kotoba.host/receipt (:kotoba.host/receipt outcome)})))))]
    (into {'cap-acquire acquire}
          (map (fn [[base with-op]]
                 [with-op (fn [handle & args]
                            (call-with base handle (vec args)))]))
          runtime/op->with-op)))
