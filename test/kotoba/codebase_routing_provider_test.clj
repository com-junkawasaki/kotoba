(ns kotoba.codebase-routing-provider-test
  (:require [clojure.test :refer [deftest is]]
            [kotoba.codebase-routing :as routing]))

(deftest provider-identities-count-peer-ids-not-addresses
  (is (= "https://example.test"
         (routing/multiaddr->base-url "/dns4/example.test/tcp/443/https")))
  (is (= [{:peer-id "peer-a"
           :addrs ["/ip4/1.1.1.1/tcp/4001" "/ip6/::1/tcp/4001"]}
          {:peer-id "peer-b" :addrs ["/ip4/2.2.2.2/tcp/4001"]}]
         (routing/provider-records->identities
          [{:ID "peer-a" :Addrs ["/ip4/1.1.1.1/tcp/4001"]}
           {:ID "peer-b" :Addrs ["/ip4/2.2.2.2/tcp/4001"]}
           {:ID "peer-a" :Addrs ["/ip6/::1/tcp/4001"]}])))
  (is (= [] (routing/provider-records->identities [{:Addrs ["/ip4/1.1.1.1"]}]))))
