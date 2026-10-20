self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  settings = config.services.holyfurries;
in
{
  options.services.holyfurries = {
    enable = lib.mkEnableOption "Holy Furries Studio";
    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
    };
    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
    };
    autoUpdate.enable = lib.mkEnableOption "pulling verified GitHub releases";
  };
  config = lib.mkIf settings.enable {
    systemd.services.holyfurries = {
      description = "Holy Furries Studio website";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network.target"
      ]
      ++ lib.optional settings.autoUpdate.enable "holyfurries-profile.service";
      requires = lib.optional settings.autoUpdate.enable "holyfurries-profile.service";
      environment = {
        LISTEN_ADDRESS = "127.0.0.1:${toString settings.port}";
      };
      serviceConfig = {
        ExecStart =
          if settings.autoUpdate.enable then
            "/nix/var/nix/profiles/holyfurries/bin/holyfurries"
          else
            lib.getExe settings.package;
        DynamicUser = true;
        Restart = "on-failure";
        RestartSec = 3;
        KillMode = "control-group";
        TimeoutStopSec = 20;
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictSUIDSGID = true;
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_INET"
          "AF_INET6"
        ];
        CapabilityBoundingSet = "";
        MemoryMax = "256M";
        TasksMax = 64;
      };
    };
    assertions = lib.optional settings.autoUpdate.enable {
      assertion = pkgs.stdenv.hostPlatform.system == "x86_64-linux";
      message = "Holy Furries release deployment requires x86_64-linux.";
    };
    systemd.services.holyfurries-profile = lib.mkIf settings.autoUpdate.enable {
      description = "Initialize the Holy Furries release profile";
      unitConfig.ConditionPathExists = "!/nix/var/nix/profiles/holyfurries";
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = "${pkgs.nix}/bin/nix-env --profile /nix/var/nix/profiles/holyfurries --set ${settings.package}";
      };
    };
    systemd.services.holyfurries-update = lib.mkIf settings.autoUpdate.enable {
      description = "Deploy the latest verified Holy Furries release";
      wants = [
        "network-online.target"
        "holyfurries.service"
      ];
      after = [
        "network-online.target"
        "holyfurries.service"
      ];
      path = [
        pkgs.coreutils
        pkgs.gnutar
        pkgs.gzip
        pkgs.nix
        pkgs.systemd
      ];
      environment = {
        HOLYFURRIES_PORT = toString settings.port;
        HOME = "/var/lib/holyfurries-update";
        NIX_REMOTE = "daemon";
      };
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${settings.package}/bin/holyfurries-update";
        StateDirectory = "holyfurries-update";
        StateDirectoryMode = "0700";
        WorkingDirectory = "/var/lib/holyfurries-update";
        TimeoutStartSec = 600;
        UMask = "0077";
        ProtectSystem = "strict";
        ReadWritePaths = [ "/nix/var/nix/profiles" ];
        ProtectHome = true;
        PrivateTmp = true;
        NoNewPrivileges = true;
      };
    };
    systemd.timers.holyfurries-update = lib.mkIf settings.autoUpdate.enable {
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnBootSec = "2m";
        OnUnitInactiveSec = "5m";
        RandomizedDelaySec = "30s";
      };
    };
  };
}
