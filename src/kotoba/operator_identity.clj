(ns kotoba.operator-identity
  "Shared local operator seed for kotoba and murakumo.

  One file, one Ed25519 seed, two CLIs. kotoba reads it when
  `KOTOBA_CODEBASE_SEED` is unset; murakumo should read the same path when
  `MURAKUMO_OPERATOR_SEED` is unset (sibling PR). The seed itself is never a
  CLI return value — callers receive only did:key and the IPNS name.

  Path contract (copy this string):
    ${XDG_DATA_HOME:-$HOME/.local/share}/kotoba/operator.seed

  The file is 64 lowercase hex characters plus a trailing newline, mode 0600.
  The parent directory `kotoba/` is created at 0700. Existing files are not
  overwritten unless the caller passes `:force?`."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [ed25519.core :as ed25519]
            [kotoba.codebase-ipns :as codebase-ipns])
  (:import [java.nio.charset StandardCharsets]
           [java.nio.file Files OpenOption StandardOpenOption]
           [java.nio.file.attribute FileAttribute PosixFilePermissions]
           [java.security SecureRandom]))

(def operator-seed-relpath
  "Relative path under the XDG data home. Both products agree on this string."
  "kotoba/operator.seed")

(def ^:dynamic *operator-seed-path*
  "Test hook. When bound, `operator-seed-path` uses this file and never the
  real home directory."
  nil)

(defn operator-seed-path*
  "Resolve the shared seed file from XDG_DATA_HOME and HOME.

  Blank `xdg-data-home` is treated as unset, matching
  `${XDG_DATA_HOME:-$HOME/.local/share}/kotoba/operator.seed`."
  [xdg-data-home home]
  (.getPath (io/file (if (and (string? xdg-data-home)
                              (not (str/blank? xdg-data-home)))
                       xdg-data-home
                       (str home "/.local/share"))
                     "kotoba"
                     "operator.seed")))

(defn operator-seed-path
  "The shared local operator seed file. Tests bind `*operator-seed-path*`."
  []
  (or *operator-seed-path*
      (operator-seed-path* (System/getenv "XDG_DATA_HOME")
                           (or (System/getenv "HOME")
                               (System/getProperty "user.home")))))

(defn valid-seed-hex?
  "True when `s` is exactly 32 bytes written as 64 hex characters."
  [s]
  (boolean (and (string? s)
                (= 64 (count s))
                (re-matches #"[0-9a-fA-F]{64}" s))))

(defn generate-seed-hex
  "32 cryptographically random bytes as lowercase hex. Not a CLI return."
  []
  (let [bytes (byte-array 32)]
    (.nextBytes (SecureRandom.) bytes)
    (apply str (map #(format "%02x" (bit-and % 0xff)) bytes))))

(defn- set-posix-mode!
  [path posix]
  (try
    (Files/setPosixFilePermissions
     path (PosixFilePermissions/fromString posix))
    (catch UnsupportedOperationException _)))

(defn write-operator-seed!
  "Write HEX to PATH at mode 0600. Creates the parent directory at 0700.

  HEX is the caller's secret; this function does not return it and does not
  log it. Parent chmod applies only to the immediate `kotoba/` directory,
  not to `$HOME` or `.local/share`."
  [path hex]
  (let [file (io/file path)
        nio (.toPath file)
        parent (.getParent nio)]
    (when parent
      (Files/createDirectories parent (make-array FileAttribute 0))
      (set-posix-mode! parent "rwx------"))
    (Files/write nio
                 (.getBytes (str hex "\n") StandardCharsets/US_ASCII)
                 (into-array OpenOption
                             [StandardOpenOption/CREATE
                              StandardOpenOption/TRUNCATE_EXISTING
                              StandardOpenOption/WRITE]))
    (set-posix-mode! nio "rw-------")
    path))

(defn read-operator-seed-hex
  "64-hex contents of the shared file after trim, or nil.

  A missing or malformed file is treated as absent — the caller then reports
  seed-required rather than echoing garbage. Does not throw on a missing file."
  ([] (read-operator-seed-hex (operator-seed-path)))
  ([path]
   (let [f (io/file path)]
     (when (.isFile f)
       (let [hex (str/trim (slurp f))]
         (when (valid-seed-hex? hex) hex))))))

(defn public-identity
  "did:key + IPNS name for a 32-byte hex seed. Does not return the seed."
  [hex]
  {:publisher (ed25519/did-key-from-seed-hex hex)
   :ipns-name (codebase-ipns/name-of (ed25519/unhex hex))})

(defn create-operator-seed!
  "Generate one seed, write it once, return only public identity + path.

  Refuses to overwrite an existing file unless `:force?` is true. The hex
  stays local to this function and is not part of the returned map."
  [{:keys [path force?]}]
  (let [path (or path (operator-seed-path))
        file (io/file path)]
    (if (and (.exists file) (not force?))
      {:ok? false :error :identity/exists :path path}
      (let [hex (generate-seed-hex)]
        (write-operator-seed! path hex)
        (assoc (public-identity hex) :ok? true :path path)))))
