class Kotoba < Formula
  desc "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution"
  homepage "https://github.com/com-junkawasaki/kotoba"
  url "https://github.com/com-junkawasaki/kotoba/archive/refs/tags/v0.1.22.tar.gz"
  sha256 "4891308f08564b10b5ad079e4f0eaa8be1485ae8f4eb11ca1ea734ee9d89a92f"
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--bin", "kotoba"
    bin.install "target/release/kotoba"
  end

  test do
    assert_match "Kotoba - Graph processing system core", shell_output("#{bin}/kotoba --help")
    assert_match "Kotoba 0.1.21", shell_output("#{bin}/kotoba --version")
    assert_match "Graph Processing System Core", shell_output("#{bin}/kotoba info")
  end
end
