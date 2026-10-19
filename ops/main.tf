terraform {
  required_version = ">= 1.8.0"
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.0"
    }
  }
}

provider "cloudflare" {}

variable "zone_id" {
  type        = string
  description = "Cloudflare zone ID for holyfurries.com. Set CLOUDFLARE_API_TOKEN in the environment."
}

variable "tunnel_id" {
  type        = string
  description = "Atlas Cloudflare Tunnel UUID."
  default     = "93c151d8-b148-4fb8-b256-e1a67e45c601"
  validation {
    condition     = can(regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", var.tunnel_id))
    error_message = "Provide a lowercase Cloudflare Tunnel UUID."
  }
}

resource "cloudflare_dns_record" "apex" {
  zone_id = var.zone_id
  name    = "holyfurries.com"
  type    = "CNAME"
  content = "${var.tunnel_id}.cfargotunnel.com"
  ttl     = 1
  proxied = true
}

resource "cloudflare_dns_record" "www" {
  zone_id = var.zone_id
  name    = "www.holyfurries.com"
  type    = "CNAME"
  content = "${var.tunnel_id}.cfargotunnel.com"
  ttl     = 1
  proxied = true
}

resource "cloudflare_zone_setting" "https" {
  zone_id    = var.zone_id
  setting_id = "always_use_https"
  value      = "on"
}

output "website" {
  value = "https://holyfurries.com"
}
