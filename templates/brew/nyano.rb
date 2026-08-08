# Documentation: https://docs.brew.sh/Formula-Cookbook
#                https://rubydoc.brew.sh/Formula
class Nyano < Formula
  desc "A simple nano replacement for quick file edits"
  homepage "https://github.com/shareui/nyano"
  version "AUTOLOAD_VERSION"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shareui/nyano/releases/download/AUTOLOAD_VERSION/nyano-macos-aarch64.tar.gz"
      sha256 "AUTOLOAD_SHA256_AARCH64"
    else
      url "https://github.com/shareui/nyano/releases/download/AUTOLOAD_VERSION/nyano-macos-x86_64.tar.gz"
      sha256 "AUTOLOAD_SHA256_X86_64"
    end
  end

  def install
    bin.install "nyano"
  end

  test do
    system "#{bin}/nyano", "--version"
  end
end
