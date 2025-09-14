class Kotoba < Formula
  desc "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution"
  homepage "https://github.com/jun784/kotoba"
  url "https://github.com/jun784/kotoba/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "114a54b76f6a6d7552d709be8746bc99406ecaac1b8b057f60b26ef53aa2ab7e"
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
