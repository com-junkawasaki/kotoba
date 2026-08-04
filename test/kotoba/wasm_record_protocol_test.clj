(ns kotoba.wasm-record-protocol-test
  (:require [clojure.test :refer [deftest is testing]]
            [kotoba.runtime :as runtime]
            [kotoba.wasm-exec :as wasm-exec]))

(defn- emit-and-run [source]
  (let [forms (runtime/read-forms (str "(ns record-protocol-test)\n" source) :kotoba)
        wasm (runtime/wasm-binary forms)]
    (is (:kotoba.wasm/ok? wasm)
        (str "record/protocol forms should emit: " (:kotoba.wasm/problems wasm)))
    (when (:kotoba.wasm/ok? wasm)
      (wasm-exec/run-main (:kotoba.wasm/binary wasm) []))))

(deftest extend-protocol-default-specializes-only-declared-records
  (is (= 109
         (emit-and-run
          "(defprotocol Value (value [this]))
           (defrecord Special [x])
           (defrecord Ordinary [x])
           (extend-protocol Value
             Special (value [this] (+ 100 (get this :x)))
             default (value [this] (get this :x)))
           (defn main [] (+ (value (->Special 4))
                            (value (->Ordinary 5))))"))))

(deftest unknown-protocol-receiver-traps-even-when-a-default-was-authored
  (is (thrown?
       Exception
       (emit-and-run
        "(defprotocol Value (value [this]))
         (defrecord Box [x])
         (extend-protocol Value default (value [this] (get this :x)))
         (defn main []
           (value {:kotoba.record/type :Ghost :x 9}))"))))

(deftest protocol-sections-are-complete-and-module-closed
  (doseq [[source pattern]
          [["(defprotocol Value (value [this]) (label [this]))
             (defrecord Box [x])
             (extend-type Box Value (value [this] (get this :x)))
             (defn main [] 0)"
            #"every declared method exactly once"]
           ["(defprotocol Value (value [this]))
             (extend-type External Value (value [this] 1))
             (defn main [] 0)"
            #"record declared in the sealed module"]
           ["(defprotocol Value (value [this]))
             (extend-protocol Value default (value [this] 1))
             (defn main [] 0)"
            #"default requires records declared"]]]
    (testing source
      (is (thrown-with-msg?
           clojure.lang.ExceptionInfo pattern
           (runtime/lower-language-forms
            (runtime/read-forms (str "(ns invalid-record-protocol) " source)
                                :kotoba)))))))
