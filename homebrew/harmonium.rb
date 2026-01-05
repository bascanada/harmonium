# Homebrew Formula for Harmonium CLI
# This file should be placed in bascanada/homebrew-tap/Formula/harmonium.rb

class Harmonium < Formula
  desc "AI-powered procedural music generator CLI"
  homepage "https://github.com/bascanada/harmonium"
  url "https://github.com/bascanada/harmonium/releases/download/v0.1.0/harmonium-cli-macos-universal.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"
  version "0.1.0"

  def install
    bin.install "harmonium"
  end

  test do
    system "#{bin}/harmonium", "--version"
  end
end
