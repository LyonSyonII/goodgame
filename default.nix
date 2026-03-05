{
  pkgs,
  lib,
  config,
  ...
}:
let
  package = (pkgs.callPackage ./package.nix { });
  cfg = config.programs.goodgame;
in
{
  options.programs.goodgame = {
    enable = lib.mkEnableOption "The Good Game Manager";
    package = lib.mkOption {
      type = lib.types.package;
      default = package;
      description = package.meta.description or "";
    };
    settings = {
      shell = lib.mkOption {
        type = lib.types.str;
        description = "Shell that will be used to execute the commands";
        default = lib.literalExpression "lib.getExe pkgs.bash";
        example = lib.literalExpression "lib.getExe pkgs.fish";
      };
      run = {
        commands = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          description = "List of commands to run the game";
          default = [ ];
          example = [ "WINEPREFIX=$(realpath ./wine) wine Minecraft.exe" ];
        };
        environment = lib.mkOption {
          type = lib.types.attrs;
          description = "Environment variables that will be set when the game runs";
          default = { };
          example = {
            MANGOHUD = 1;
            MANGOHUD_CONFIG = "read_cfg,no_display";
            PROTON_ADD_CONFIG = "fsr4rdna3,wayland";
          };
        };
      };
      backup = {
        cloudInitCommands = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          description = "List of commands to initialize the cloud backup.\nAll the commands will be concatenated with '&&'.";
          default = [ ];
          example = [
            "git init"
            "echo -e '*\\n!gg-saves\\n!.gitignore' > .gitignore"
            "glab repo create @GAME-SLUG --private --defaultBranch main --skipGitInit"
            "git add ."
            "git commit -m first || true"
            "git push --set-upstream origin main"
          ];
        };
        cloudCommitCommands = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          description = "List of commands to commit changes to cloud backup.\nAll the commands will be concatenated with '&&'.";
          default = [ ];
          example = [
            "git add ."
            "git commit -m 'backup'"
          ];
        };
        cloudPushCommands = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          description = "List of commands to push changes to cloud backup.\nAll the commands will be concatenated with '&&'.";
          default = [ ];
          example = [
            "git push"
          ];
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [
      cfg.package
    ];
    environment.etc."goodgame/config.yaml".text = lib.generators.toYAML { } cfg.settings;
  };
}
