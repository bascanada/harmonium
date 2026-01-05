# Homebrew Formula for Harmonium CLI
# This file should be placed in bascanada/homebrew-tap/Formula/harmonium.rb

class Harmonium < Formula
  desc "AI-powered procedural music generator CLI"
  homepage "https://github.com/bascanada/harmonium"
  url "https://github.com/bascanada/harmonium/releases/download/v0.0.2/harmonium-cli-macos-universal.tar.gz"
  sha256 "6c3888b7725805d4cdf549864ad9329a26c1f4a1c71f6e2f1bc117f001df513a"
  license "MIT"
  version "0.0.2"

  def install
    bin.install "harmonium"
  end

  test do
    system "#{bin}/harmonium", "--version"
  end
end
