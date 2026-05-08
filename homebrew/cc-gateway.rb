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
      
        # Launch interactive dashboard (default)
        cc-gateway
        
        # Import from cc-switch (if you have it)
        cc-gateway import
        
        # Or add providers from presets
        cc-gateway add mimo      # Xiaomi Mimo
        cc-gateway add kimi      # Moonshot Kimi
        cc-gateway add glm       # Zhipu GLM
        cc-gateway add qwen      # Alibaba Qwen
        
        # Start the proxy server
        cc-gateway serve
        
        # Then configure Claude Code:
        export ANTHROPIC_BASE_URL=http://127.0.0.1:16789
        claude
      
      Config file: ~/.cc-gateway/config.toml
      Database: ~/.cc-gateway/cc-gateway.db
    EOS
  end

  test do
    assert_match "cc-gateway", shell_output("#{bin}/cc-gateway --version")
  end
end
