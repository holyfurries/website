self:
{ config, ... }:
{
  imports = [ (import ./service.nix self) ];
  services.holyfurries = {
    enable = true;
    port = 8083;
  };
  services.cloudflared.tunnels."93c151d8-b148-4fb8-b256-e1a67e45c601".ingress = {
    "holyfurries.com".service = "http://127.0.0.1:${toString config.services.holyfurries.port}";
    "www.holyfurries.com".service = "http://127.0.0.1:${toString config.services.holyfurries.port}";
  };
}
