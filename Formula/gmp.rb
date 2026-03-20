class Gmp < Formula
  desc "Simple, fast TUI password manager with AES-256-GCM encryption"
  homepage "https://github.com/Sn0wAlice/GetMyPass"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmp-darwin-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    elsif Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmp-darwin-amd64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmp-linux-amd64.tar.gz"
      sha256 "PLACEHOLDER"
    elsif Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmp-linux-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "gmp"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gmp --version", 0)
  end
end
