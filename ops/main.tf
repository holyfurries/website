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

resource "cloudflare_zone_setting" "minimum_tls" {
  zone_id    = var.zone_id
  setting_id = "min_tls_version"
  value      = "1.2"
}

resource "cloudflare_zone_setting" "transport" {
  for_each = {
    ssl              = "strict"
    tls_1_3          = "on"
    http3            = "on"
    brotli           = "on"
    cname_flattening = "flatten_at_root"
  }

  zone_id    = var.zone_id
  setting_id = each.key
  value      = each.value
}

resource "cloudflare_zone_setting" "browser_cache" {
  zone_id    = var.zone_id
  setting_id = "browser_cache_ttl"
  value      = 0
}

resource "cloudflare_zone_setting" "hsts" {
  zone_id    = var.zone_id
  setting_id = "security_header"
  value = {
    strict_transport_security = {
      enabled            = true
      max_age            = 2592000
      include_subdomains = false
      preload            = false
      nosniff            = true
    }
  }
}

resource "cloudflare_zone_dnssec" "website" {
  zone_id = var.zone_id
  status  = "active"

  lifecycle {
    prevent_destroy = true
  }
}

resource "cloudflare_ruleset" "redirects" {
  zone_id = var.zone_id
  name    = "Website redirects"
  kind    = "zone"
  phase   = "http_request_dynamic_redirect"

  rules = [{
    ref         = "www_to_apex"
    description = "Redirect www to the canonical HTTPS hostname"
    expression  = "http.host eq \"www.holyfurries.com\""
    action      = "redirect"
    enabled     = true
    action_parameters = {
      from_value = {
        status_code           = 301
        preserve_query_string = true
        target_url = {
          expression = "concat(\"https://holyfurries.com\", http.request.uri.path)"
        }
      }
    }
  }]
}

resource "cloudflare_ruleset" "cache" {
  zone_id = var.zone_id
  name    = "Website cache"
  kind    = "zone"
  phase   = "http_request_cache_settings"

  rules = [
    {
      ref         = "bypass_dynamic_content"
      description = "Bypass cache for pages, live updates, and health checks"
      expression  = "http.host in {\"holyfurries.com\" \"www.holyfurries.com\"}"
      action      = "set_cache_settings"
      enabled     = true
      action_parameters = {
        cache = false
      }
    },
    {
      ref         = "cache_static_assets"
      description = "Cache static GET and HEAD responses using origin cache headers"
      expression  = "http.host eq \"holyfurries.com\" and starts_with(http.request.uri.path, \"/static/\") and http.request.method in {\"GET\" \"HEAD\"}"
      action      = "set_cache_settings"
      enabled     = true
      action_parameters = {
        cache = true
        edge_ttl = {
          mode = "bypass_by_default"
        }
        browser_ttl = {
          mode = "respect_origin"
        }
      }
    }
  ]
}

resource "cloudflare_ruleset" "compression" {
  zone_id = var.zone_id
  name    = "Website compression"
  kind    = "zone"
  phase   = "http_response_compression"

  rules = [{
    ref         = "compress_text_responses"
    description = "Prefer Brotli with gzip fallback for text, scripts, and SVGs"
    expression  = "http.host eq \"holyfurries.com\" and http.request.uri.path ne \"/events\" and (any(http.response.headers[\"content-type\"][*] contains \"text/\") or any(http.response.headers[\"content-type\"][*] contains \"javascript\") or any(http.response.headers[\"content-type\"][*] contains \"image/svg+xml\") or any(http.response.headers[\"content-type\"][*] contains \"application/xml\"))"
    action      = "compress_response"
    enabled     = true
    action_parameters = {
      algorithms = [{ name = "brotli" }, { name = "gzip" }]
    }
  }]
}

output "website" {
  value = "https://holyfurries.com"
}

output "dnssec_ds_record" {
  description = "DS record for verifying Cloudflare Registrar's DNSSEC activation."
  value       = cloudflare_zone_dnssec.website.ds
}
