# Homebrew Formula for Harmonium CLI
# This file should be placed in bascanada/homebrew-tap/Formula/harmonium.rb

class Harmonium < Formula
  desc "AI-powered procedural music generator CLI"
  homepage "https://github.com/bascanada/harmonium"
  url "https://github.com/bascanada/harmonium/releases/download/v0.0.4/harmonium-cli-macos-universal.tar.gz"
  sha256 "1b61c2d9748e31a884823a5b9243bf3a652a0aaf115d5bb5ecb9e40b5b5dce0f"
  license "MIT"
  version "0.0.4"

  def install
    bin.install "harmonium"
  end

  test do
    system "#{bin}/harmonium", "--version"
  end
end
