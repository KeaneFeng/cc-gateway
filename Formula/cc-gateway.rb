class CcGateway < Formula
  desc "Multi-provider aggregation gateway for Claude Code"
  homepage "https://github.com/KeaneFeng/cc-gateway"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-aarch64-apple-darwin.tar.gz"
      sha256 "dfe6c1d6df37ba575d7d19784a36aabd6d09a5ec3430ffaba51b1fe98b09e84c"
    else
      depends_on "rust" => :build
      install do
        system "cargo", "install", "--path", "."
      end
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "eb346916f93f89a695b463c7c779df25211749783510f9eb8d93d72bed07ee8f"
    else
      url "https://github.com/KeaneFeng/cc-gateway/releases/download/v0.3.0/cc-gateway-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a2dd3d9114c1cfa5fea277a5f99473cb7f12cc1a4588b025cecaa50d64bf5782"
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
