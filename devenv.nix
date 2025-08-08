{ pkgs, lib, config, inputs, ... }:

{
  stdenv = pkgs.clang12Stdenv;

  env.NIX_ENFORCE_PURITY = 0;
  env.RUST_BACKTRACE = "full";
  env.LIBCLANG_PATH = "${pkgs.llvmPackages_12.libclang.lib}/lib";

  processes.build.exec = "cargo build --release";

  packages = [
    pkgs.glib.dev
    pkgs.pipewire.dev
    pkgs.pkg-config
  ];

  languages.rust = {
    channel = "stable"; # <-- notice this
    enable = true;
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
  };

  enterShell = ''
    echo "Rust version: $(rustc --version)"
    echo "Cargo version: $(cargo --version)"
    echo "RUST_SRC_PATH: $RUST_SRC_PATH"
  '';
}
