class Kotoba < Formula
  desc "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution"
  homepage "https://github.com/jun784/kotoba"
  url "https://github.com/jun784/kotoba/archive/refs/tags/v0.1.16.tar.gz"
  sha256 "96123936249eea21ce9760240fb500cdce896009a43f1bd10b43fe3b1e97b61e"
  license "MIT OR Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--features", "binary"
    bin.install "target/release/kotoba"
  end

  test do
    assert_match "Kotoba - Graph processing system inspired by Deno", shell_output("#{bin}/kotoba --help")
    assert_match "Kotoba 0.1.16", shell_output("#{bin}/kotoba version")
    assert_match "Kotoba Project Information", shell_output("#{bin}/kotoba info")
  end
end
