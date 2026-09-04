(ns kotoba.key-hierarchy-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.key-hierarchy :as hierarchy]))

(deftest shared-hierarchy-covers-products-purposes-and-lifecycle
  (let [result (hierarchy/report (hierarchy/read-policy))]
    (is (:key-hierarchy/valid? result) (pr-str result))
    (is (= #{:kotoba :kototama :aiueos :kotoba-lang :kotobase}
           (set (keys (:key-hierarchy/product-errors result)))))))

(deftest omitted-control-is-rejected
  (let [policy (assoc-in (hierarchy/read-policy)
                         [:products :aiueos :lifecycles] #{:issuance})
        result (hierarchy/report policy)]
    (is (false? (:key-hierarchy/valid? result)))
    (is (contains? (set (:key-hierarchy/errors result))
                   :key-hierarchy/lifecycle))))
