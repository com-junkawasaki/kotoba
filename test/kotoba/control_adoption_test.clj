(ns kotoba.control-adoption-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.control-adoption :as adoption]))

(deftest all-products-adopt-the-current-control-catalog
  (let [result (adoption/report (adoption/read-edn adoption/catalog-path)
                                (adoption/read-edn adoption/adoption-path))]
    (is (:control-adoption/valid? result) (pr-str result))
    (is (= #{:kotoba :kototama :aiueos :kotoba-lang :kotobase}
           (set (keys (:control-adoption/product-errors result)))))))

(deftest missing-control-is-rejected
  (let [catalog (adoption/read-edn adoption/catalog-path)
        broken (update-in (adoption/read-edn adoption/adoption-path)
                          [:adoptions :kotoba :controls] disj :sealed-egress)
        result (adoption/report catalog broken)]
    (is (false? (:control-adoption/valid? result)))
    (is (contains? (set (:control-adoption/errors result))
                   :control-adoption/coverage))))
