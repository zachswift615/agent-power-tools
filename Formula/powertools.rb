class Powertools < Formula
  desc "Code indexing and navigation tools for AI agents"
  homepage "https://github.com/zachswift615/agent-powertools"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/zachswift615/agent-powertools/releases/download/v#{version}/powertools-macos-arm64.tar.gz"
      sha256 "REPLACE_WITH_ARM64_SHA256"
    else
      url "https://github.com/zachswift615/agent-powertools/releases/download/v#{version}/powertools-macos-x86_64.tar.gz"
      sha256 "REPLACE_WITH_X86_64_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/zachswift615/agent-powertools/releases/download/v#{version}/powertools-linux-x86_64.tar.gz"
      sha256 "REPLACE_WITH_LINUX_SHA256"
    end
  end

  def install
    bin.install "powertools"
  end

  def caveats
    <<~EOS
      Powertools has been installed!

      To use as an MCP server with Claude Code, add this to ~/.config/claude/mcp_settings.json:

      {
        "mcpServers": {
          "powertools": {
            "command": "#{bin}/powertools",
            "args": ["--mcp-server"],
            "description": "Semantic code navigation and analysis tools"
          }
        }
      }

      For more information, see: https://github.com/zachswift615/agent-powertools
    EOS
  end

  test do
    assert_match "powertools", shell_output("#{bin}/powertools --help")
  end
end
