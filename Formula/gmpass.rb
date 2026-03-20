class Gmpass < Formula
  desc "Simple, fast TUI password manager with AES-256-GCM encryption"
  homepage "https://github.com/Sn0wAlice/GetMyPass"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-darwin-arm64.tar.gz"
      sha256 "1a41b2f46aa92213317f2b25c0fde52c58b701cf7e3f8037c9c9f33f38f2bc73"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-amd64.tar.gz"
      sha256 "bd484691e893981110dcbd21a6ea5f32d7385f0f87a22d3749c9e3fb15cf9727"
    elsif Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-arm64.tar.gz"
      sha256 "fd86b655342442bc5e221815b1cd74fbd690aa4c3f917d87c283f3fc1786d1fe"
    end
  end

  def install
    bin.install "gmpass"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gmpass --version", 0)
  end
end
