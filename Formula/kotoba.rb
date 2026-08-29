class Kotoba < Formula
  desc "Capability-safe Kotoba language compiler and CLI"
  homepage "https://github.com/kotoba-lang/kotoba"
  url "https://github.com/kotoba-lang/kotoba/archive/refs/tags/v0.7.3.tar.gz"
  sha256 "2ca97dd427754e0472c8efe02cd8f1c090811a3111bfa8afe6354bd497186a22"
  license "Apache-2.0"

  resource "binary" do
    on_macos do
      on_arm do
        url "https://github.com/kotoba-lang/kotoba/releases/download/v0.7.3/kotoba-darwin-arm64.tar.gz"
        sha256 "5ba2cdf04278e603cc89d7e7e581879664a2adcadbc66d65ae62beaa91ab4cd4"
      end
      on_intel do
        url "https://github.com/kotoba-lang/kotoba/releases/download/v0.7.3/kotoba-darwin-amd64.tar.gz"
        sha256 "3fc5ffbb76458ae0c833500ca71e4bf8e0837d4a229ecc402882cf3596a63f1f"
      end
    end
    on_linux do
      url "https://github.com/kotoba-lang/kotoba/releases/download/v0.7.3/kotoba-linux-amd64.tar.gz"
      sha256 "6b14c81619cf019feed5ca8a295fb27db34883bbaca4a547e98a3db398e2e058"
    end
  end

  def install
    resource("binary").stage do
      bin.install "kotoba"
    end
  end

  test do
    output = shell_output("#{bin}/kotoba selfhost check --json")
    assert_match '"kotoba.cli\\/ok?":true', output
    assert_match '"kotoba.cli\\/code":"valid"', output

    (testpath/"safe-window-name.kotoba").write <<~KOTOBA
      (ns homebrew.timing (:export [shot-hit]))
      (defn shot-hit [delta-present delta-ms window-ms]
        (if delta-present (if (<= delta-ms window-ms) 1 0) 0))
    KOTOBA
    output = shell_output(
      "#{bin}/kotoba compile #{testpath}/safe-window-name.kotoba " \
      "--target web -o #{testpath}/safe-window-name.mjs --json",
    )
    assert_match '"kotoba.cli\\/code":"emitted"', output
    assert_match "k$window$002dms", (testpath/"safe-window-name.mjs").read

    (testpath/"src/shared").mkpath
    (testpath/"src/shared/value.cljc").write <<~CLJC
      (ns shared.value "bounded bottle project documentation" (:export [answer]))
      (defn answer [] 42)
    CLJC
    (testpath/"main.cljc").write <<~CLJC
      (ns shared.app
        (:require [shared.value :as value])
        (:export [main]))
      (defn main [] (value/answer))
    CLJC
    output = shell_output(
      "#{bin}/kotoba compile #{testpath}/main.cljc " \
      "--source-path #{testpath}/src --unpinned --target web " \
      "--output #{testpath}/shared-app.mjs --json",
    )
    assert_match '"kotoba.cli\\/code":"emitted"', output
    assert_match '"kotoba.artifact\\/module-graph-digest"', output
    assert_path_exists testpath/"shared-app.mjs"

    (testpath/"typed/fixture").mkpath
    (testpath/"typed/fixture/coverage.kotoba").write <<~KOTOBA
      (ns fixture.coverage
        (:export [ready? make-report none-report choose-report covered-count map-score]))
      (def label-map-type [:map :keyword :string])
      (defn ready? [covered [:set :keyword]] :bool
        (typed-set-contains [:set :keyword] covered :ready))
      (defn none-report []
        [:option [:record :fixture/report
                  [[:label :string] [:covered [:set :keyword]]]]]
        (option-none-of
          [:option [:record :fixture/report
                    [[:label :string] [:covered [:set :keyword]]]]]))
      (defn make-report []
        [:record :fixture/report [[:label :string] [:covered [:set :keyword]]]]
        (record
          [:record :fixture/report [[:label :string] [:covered [:set :keyword]]]]
          "qualified" (typed-set [:set :keyword] :ready :reviewed)))
      (defn choose-report
        [left [:option [:record :fixture/report
                        [[:label :string] [:covered [:set :keyword]]]]]
         right [:option [:record :fixture/report
                         [[:label :string] [:covered [:set :keyword]]]]]]
        [:option [:record :fixture/report
                  [[:label :string] [:covered [:set :keyword]]]]]
        (match-option left
          [:option [:record :fixture/report
                    [[:label :string] [:covered [:set :keyword]]]]]
          (none right)
          (some left-report
            (match-option right
              [:option [:record :fixture/report
                        [[:label :string] [:covered [:set :keyword]]]]]
              (none left)
              (some right-report right)))))
      (defn covered-count
        [report [:record :fixture/report
                 [[:label :string] [:covered [:set :keyword]]]]]
        :i64
        (typed-set-count [:set :keyword]
          (record-get
            [:record :fixture/report [[:label :string] [:covered [:set :keyword]]]]
            report :covered)))
      (defn map-score [] :i64
        (let [labels (typed-map-assoc label-map-type
                       (typed-map-assoc label-map-type
                         (typed-map-new label-map-type) :ready "yes")
                       :reviewed "yes")
              first-entry (option-value-of
                            [:option [:vector [:keyword :string]]]
                            (typed-map-entry-at label-map-type labels 0)
                            (hetero-vector [:vector [:keyword :string]] :missing "no"))]
          (if (= (typed-map-count label-map-type labels) 2)
            (if (typed-map-contains label-map-type labels :ready)
              (if (string=?
                    (option-value-of [:option :string]
                      (typed-map-get label-map-type labels :reviewed) "no")
                    "yes")
                (if (= (hetero-vector-count
                         [:vector [:keyword :string]] first-entry) 2)
                  2
                  0)
                0)
              0)
            0)))
    KOTOBA
    (testpath/"typed/fixture/app.kotoba").write <<~KOTOBA
      (ns fixture.app
        (:require [fixture.coverage :as coverage])
        (:export [main]))
      (defn main [] :i64
        (let [covered (typed-set [:set :keyword] :ready :reviewed)]
          (if (coverage/ready? covered)
            (if (string=? "Kotoba" "Kotoba")
              (+ 38
                (coverage/map-score)
                (coverage/covered-count
                  (option-value-of
                    [:option [:record :fixture/report
                              [[:label :string] [:covered [:set :keyword]]]]]
                    (coverage/choose-report
                      (coverage/none-report)
                      (option-some-of
                        [:option [:record :fixture/report
                                  [[:label :string] [:covered [:set :keyword]]]]]
                        (coverage/make-report)))
                    (coverage/make-report))))
              1)
            0)))
    KOTOBA
    web = shell_output(
      "#{bin}/kotoba compile #{testpath}/typed/fixture/app.kotoba " \
      "--source-path #{testpath}/typed --unpinned --target web " \
      "--output #{testpath}/typed-app.mjs --json",
    )
    assert_match '"kotoba.artifact\\/value-profile":"typed-v1"', web
    assert_match '"kotoba.artifact\\/module-graph-digest"', web
    wasm = shell_output(
      "#{bin}/kotoba compile #{testpath}/typed/fixture/app.kotoba " \
      "--source-path #{testpath}/typed --unpinned --target wasm " \
      "--output #{testpath}/typed-app.wasm --json",
    )
    assert_match '"value-profile":"typed-v1"', wasm
    assert_match '"value-abi":"externref-v1"', wasm
    assert_path_exists testpath/"typed-app.wasm"
  end
end
