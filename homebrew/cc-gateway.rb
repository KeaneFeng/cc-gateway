class CcGateway < Formula
  desc "Multi-provider aggregation gateway for Claude Code"
  homepage "https://github.com/KeaneFeng/cc-gateway"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    else
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    else
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install "cc-gateway"

    # Generate shell completions
    generate_completions_from_executable(bin/"cc-gateway", "completion")

    # Create config directory
    (etc/"cc-gateway").mkpath
  end

  def caveats
    <<~EOS
      To get started with cc-gateway:

        # 1. Add providers from presets
        cc-gateway add mimo      # Xiaomi Mimo
        cc-gateway add kimi      # Moonshot Kimi
        cc-gateway add glm       # Zhipu GLM
        cc-gateway add qwen      # Alibaba Qwen

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
