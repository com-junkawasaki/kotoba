class Kotoba < Formula
  desc "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution"
  homepage "https://github.com/jun784/kotoba"
  url "https://github.com/jun784/kotoba/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "e52940ce7c5ed56a4ae68a772147d006912002712bae59c354cdab9ec00fe899"
  license "MIT OR Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--features", "binary"
    bin.install "target/release/kotoba"
  end

  test do
    assert_match "Kotoba - Graph processing system inspired by Deno", shell_output("#{bin}/kotoba --help")
    assert_match "Kotoba 0.1.2", shell_output("#{bin}/kotoba version")
    assert_match "Kotoba Project Information", shell_output("#{bin}/kotoba info")
  end
end
