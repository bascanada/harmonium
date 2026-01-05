# Homebrew Cask for Harmonium Plugins
# This file should be placed in bascanada/homebrew-tap/Casks/harmonium-plugins.rb

cask "harmonium-plugins" do
  version "0.0.4"
  sha256 "e82c1f90b163fdb5dcb59eab04e3f03e23c2c096b8e78e252c71d7e822f18f8c"

  url "https://github.com/bascanada/harmonium/releases/download/v#{version}/harmonium-plugins-macos-universal.zip"
  name "Harmonium Plugins"
  desc "AI-powered generative music VST3/CLAP plugins"
  homepage "https://github.com/bascanada/harmonium"

  artifact "harmonium.vst3", target: "~/Library/Audio/Plug-Ins/VST3/harmonium.vst3"
  artifact "harmonium.clap", target: "~/Library/Audio/Plug-Ins/CLAP/harmonium.clap"

  # Remove quarantine to bypass Gatekeeper for unsigned plugins
  postflight do
    system_command "/usr/bin/xattr", args: ["-cr", "#{Dir.home}/Library/Audio/Plug-Ins/VST3/harmonium.vst3"]
    system_command "/usr/bin/xattr", args: ["-cr", "#{Dir.home}/Library/Audio/Plug-Ins/CLAP/harmonium.clap"]
  end

  uninstall delete: [
    "~/Library/Audio/Plug-Ins/VST3/harmonium.vst3",
    "~/Library/Audio/Plug-Ins/CLAP/harmonium.clap",
  ]

  caveats <<~EOS
    Restart your DAW to detect the new plugins.

    The plugins will be installed to:
      ~/Library/Audio/Plug-Ins/VST3/harmonium.vst3
      ~/Library/Audio/Plug-Ins/CLAP/harmonium.clap
  EOS
end
