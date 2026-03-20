class Gmpass < Formula
  desc "Simple, fast TUI password manager with AES-256-GCM encryption"
  homepage "https://github.com/Sn0wAlice/GetMyPass"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-darwin-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-amd64.tar.gz"
      sha256 "PLACEHOLDER"
    elsif Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "gmpass"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gmpass --version", 0)
  end
end
