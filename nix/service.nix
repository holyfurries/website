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
  };
  config = lib.mkIf settings.enable {
    services.nats = {
      enable = true;
      settings = {
        host = "127.0.0.1";
        port = 4222;
        max_payload = 65536;
      };
    };
    systemd.services.holyfurries = {
      description = "Holy Furries Studio website";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network.target"
        "nats.service"
      ];
      requires = [ "nats.service" ];
      environment = {
        LISTEN_ADDRESS = "127.0.0.1:${toString settings.port}";
        NATS_URL = "nats://127.0.0.1:4222";
      };
      serviceConfig = {
        ExecStart = lib.getExe settings.package;
        DynamicUser = true;
        Restart = "on-failure";
        RestartSec = 3;
        TimeoutStopSec = 15;
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
  };
}
