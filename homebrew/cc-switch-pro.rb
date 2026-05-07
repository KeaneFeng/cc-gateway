class CcSwitchPro < Formula
  desc "Lightweight multi-provider aggregation proxy for Claude Code"
  homepage "https://github.com/yourusername/cc-switch-pro"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yourusername/cc-switch-pro/releases/download/v0.2.0/cc-switch-pro-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    else
      url "https://github.com/yourusername/cc-switch-pro/releases/download/v0.2.0/cc-switch-pro-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/yourusername/cc-switch-pro/releases/download/v0.2.0/cc-switch-pro-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    else
      url "https://github.com/yourusername/cc-switch-pro/releases/download/v0.2.0/cc-switch-pro-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install "cc-switch-pro"
    
    # Generate shell completions
    generate_completions_from_executable(bin/"cc-switch-pro", "completion")
    
    # Create config directory
    (etc/"cc-switch-pro").mkpath
  end

  def caveats
    <<~EOS
      To get started with cc-switch-pro:
      
        # Initialize config
        cc-switch-pro init
        
        # Import from cc-switch (if you have it)
        cc-switch-pro import
        
        # Or add providers from presets
        cc-switch-pro presets
        cc-switch-pro add --preset deepseek --key YOUR_API_KEY
        
        # Start the proxy server
        cc-switch-pro serve
        
        # Then configure Claude Code:
        export ANTHROPIC_BASE_URL=http://127.0.0.1:16789
        claude
      
      Interactive mode:
        cc-switch-pro interactive
      
      Config file: ~/.cc-switch-pro/config.toml
      Database: ~/.cc-switch-pro/cc-switch-pro.db
    EOS
  end

  test do
    assert_match "cc-switch-pro", shell_output("#{bin}/cc-switch-pro --version")
  end
end
