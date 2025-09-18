class Kotoba < Formula
  desc "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution"
  homepage "https://github.com/com-junkawasaki/kotoba"
  url "https://github.com/com-junkawasaki/kotoba/archive/refs/tags/v0.1.17.tar.gz"
  sha256 "e1e23e782f999ab665aa9da48fd8774be61924fe3c6557789f280d79eac4e50c"
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--features", "binary"
    bin.install "target/release/kotoba"
  end

  test do
    assert_match "Kotoba - Graph processing system inspired by Deno", shell_output("#{bin}/kotoba --help")
    assert_match "Kotoba 0.1.17", shell_output("#{bin}/kotoba version")
    assert_match "Kotoba Project Information", shell_output("#{bin}/kotoba info")
  end
end
