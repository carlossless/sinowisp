{ pkgs ? import <nixpkgs> { } }:
with pkgs;
mkShell {
  buildInputs = [
    rust-bin.nightly.latest.default
    pkg-config
    libusb1
  ] ++
  (lib.optionals (stdenv.hostPlatform.isLinux) [
    udev
  ]) ++
  (lib.optionals (stdenv.hostPlatform.isDarwin) [
    darwin.apple_sdk.frameworks.IOKit
    darwin.apple_sdk.frameworks.AppKit
    iconv
  ]);
}
