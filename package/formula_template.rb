class Dt < Formula
  desc "{{DESC}}"
  homepage "{{HOMEPAGE}}"
  url "{{URL}}"
  sha256 "{{SHA256}}"
  license "{{LICENSE}}"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    output = shell_output("#{bin}/dt --version")
    assert_match version.to_s, output
  end
end

