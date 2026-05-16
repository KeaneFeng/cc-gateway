class CcGateway < Formula
  desc "Multi-provider aggregation gateway for Claude Code"
  homepage "https://github.com/KeaneFeng/cc-gateway"
  version "0.4.6"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.4.6/cc-gateway-aarch64-apple-darwin.tar.gz"
      sha256 "2c98851a62292bb9a4cb13150d9d1c3bc1bd4f325cec8ac4e0ff5b567ffed92f"
    else
      depends_on "rust" => :build
      install do
        system "cargo", "install", "--path", "."
      end
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.4.6/cc-gateway-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    else
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.4.6/cc-gateway-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  def install
    if Hardware::CPU.arm? && OS.mac?
      bin.install "cc-gateway"
      generate_completions_from_executable(bin/"cc-gateway", "completion")
    else
      generate_completions_from_executable(bin/"cc-gateway", "completion") if File.exist?(bin/"cc-gateway")
    end
    (etc/"cc-gateway").mkpath
  end

  def caveats
    <<~EOS
      To get started with cc-gateway:

        # 1. Launch interactive dashboard
        cc-gateway

        # Or add providers from CLI:
        cc-gateway add mimo      # Xiaomi Mimo
        cc-gateway add kimi      # Moonshot Kimi

        # 2. Start the server (background)
        cc-gateway start -d

        # 3. Use Claude Code
        claude
        # In Claude Code: /model -> select provider

      Manage the server:
        cc-gateway start          # Start (foreground)
        cc-gateway start -d       # Start (background)
        cc-gateway stop           # Stop
        cc-gateway restart -d     # Restart

      Config: ~/.cc-gateway/config.toml
      Log:    ~/.cc-gateway/cc-gateway.log
    EOS
  end

  test do
    assert_match "cc-gateway", shell_output("#{bin}/cc-gateway --version")
  end
end
