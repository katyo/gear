{ pkgs ? import <nixpkgs> {} }:
with pkgs;
mkShell {
  buildInputs = [
    pkg-config lzma
    llvmPackages.clang-unwrapped llvmPackages.llvm
    gcc-arm-embedded-svd
    gdc
    ldc
  ];
}
