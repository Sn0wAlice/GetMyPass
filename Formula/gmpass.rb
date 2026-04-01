class Gmpass < Formula
  desc "Simple, fast TUI password manager with AES-256-GCM encryption"
  homepage "https://github.com/Sn0wAlice/GetMyPass"
  version "0.6.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-darwin-arm64.tar.gz"
      sha256 "f9d35dcfbffedbc19704e75ec19d618332e681da022cb652531ee6385d25c56b"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-amd64.tar.gz"
      sha256 "fe8d67812f1ddba64a6e2a2de2210cf5580c0f85e73a1aed900b8208c33b9f43"
    elsif Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-arm64.tar.gz"
      sha256 "e46f748eac0d947fcc4b00c936a5378983ee0148a1594360cbdc750e263943a4"
    end
  end

  def install
    bin.install "gmpass"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gmpass --version", 0)
  end
end
