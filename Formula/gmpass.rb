class Gmpass < Formula
  desc "Simple, fast TUI password manager with AES-256-GCM encryption"
  homepage "https://github.com/Sn0wAlice/GetMyPass"
  version "0.5.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-darwin-arm64.tar.gz"
      sha256 "8b836f8219b2689c844fcc45a0cb21f16cfc23d283777da499b22f2a41f3d7f9"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-amd64.tar.gz"
      sha256 "b8824dce1a67843178987f8cd184a0569be1cfca51a5349edfc896ffbfdd373a"
    elsif Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-arm64.tar.gz"
      sha256 "d138cb955145bf3c8a8ee1bda99f4a529d530549506add0a04da85ac88547aac"
    end
  end

  def install
    bin.install "gmpass"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gmpass --version", 0)
  end
end
