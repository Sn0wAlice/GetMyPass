class Gmpass < Formula
  desc "Simple, fast TUI password manager with AES-256-GCM encryption"
  homepage "https://github.com/Sn0wAlice/GetMyPass"
  version "0.4.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-darwin-arm64.tar.gz"
      sha256 "1aae44f28125399f40b7a490e9449a19a8401920d9bdee5b33af30bcbb631dd5"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-amd64.tar.gz"
      sha256 "07d87b610fdfa3fab34f09e3cb8fe53a0efe41855798e36c339722ec025809e7"
    elsif Hardware::CPU.arm?
      url "https://github.com/Sn0wAlice/GetMyPass/releases/download/v#{version}/gmpass-linux-arm64.tar.gz"
      sha256 "75de21ec30df03c8a0463985d8705f28d04918a54818fbb05a5029533f993a0c"
    end
  end

  def install
    bin.install "gmpass"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gmpass --version", 0)
  end
end
