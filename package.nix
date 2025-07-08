{ lib, rustPlatform, installShellFiles } : rustPlatform.buildRustPackage (finalAttrs: {
  pname = "goodgame";
  version = "0.1.0";

  src = ./.;

  nativeBuildInputs = [ installShellFiles ];

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

   postInstall = ''
    installShellCompletion --cmd gg \
      --fish <(echo "source (COMPLETE=fish $out/bin/gg | psub)") \
      --bash <(echo "source (COMPLETE=bash $out/bin/gg)") \
      --zsh  <(echo "source (COMPLETE=zsh $out/bin/gg)")
  '';

  meta = {
    mainProgram = "gg";
  };
})