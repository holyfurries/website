mock_provider "cloudflare" {}

variables {
  zone_id = "00000000000000000000000000000000"
}

run "route_both_names_to_atlas" {
  command = plan

  assert {
    condition     = cloudflare_dns_record.apex.type == "CNAME" && cloudflare_dns_record.apex.proxied && cloudflare_dns_record.apex.content == "93c151d8-b148-4fb8-b256-e1a67e45c601.cfargotunnel.com"
    error_message = "The apex must use the proxied Atlas tunnel."
  }

  assert {
    condition     = cloudflare_dns_record.www.content == cloudflare_dns_record.apex.content && cloudflare_dns_record.www.proxied
    error_message = "The www name must reach the same proxied tunnel."
  }

  assert {
    condition     = cloudflare_zone_setting.https.value == "on"
    error_message = "Public HTTP requests must redirect to HTTPS."
  }
}

run "reject_arbitrary_hostname" {
  command = plan

  variables {
    tunnel_id = "another.example.com"
  }

  expect_failures = [var.tunnel_id]
}

run "require_modern_tls_and_hsts" {
  command = plan

  assert {
    condition     = cloudflare_zone_setting.minimum_tls.setting_id == "min_tls_version" && cloudflare_zone_setting.minimum_tls.value == "1.2"
    error_message = "Both public hostnames must reject TLS versions older than 1.2."
  }

  assert {
    condition     = cloudflare_zone_setting.hsts.setting_id == "security_header" && cloudflare_zone_setting.hsts.value.strict_transport_security.enabled && cloudflare_zone_setting.hsts.value.strict_transport_security.max_age == 2592000
    error_message = "HTTPS responses must enable HSTS for 30 days."
  }

  assert {
    condition     = !cloudflare_zone_setting.hsts.value.strict_transport_security.include_subdomains && !cloudflare_zone_setting.hsts.value.strict_transport_security.preload
    error_message = "HSTS must not enforce HTTPS on other subdomains or request browser preloading."
  }

  assert {
    condition     = cloudflare_zone_setting.hsts.value.strict_transport_security.nosniff
    error_message = "The edge must preserve content-type sniffing protection."
  }
}

run "sign_website_dns" {
  command = plan

  assert {
    condition     = cloudflare_zone_dnssec.website.zone_id == var.zone_id && cloudflare_zone_dnssec.website.status == "active"
    error_message = "DNSSEC signing must be enabled for the website zone."
  }
}

run "redirect_www_without_losing_request_details" {
  command = plan

  assert {
    condition     = cloudflare_ruleset.redirects.phase == "http_request_dynamic_redirect" && length(cloudflare_ruleset.redirects.rules) == 1
    error_message = "The website must have one canonical hostname redirect."
  }

  assert {
    condition     = cloudflare_ruleset.redirects.rules[0].enabled && cloudflare_ruleset.redirects.rules[0].expression == "http.host eq \"www.holyfurries.com\""
    error_message = "Only www requests may match the canonical hostname redirect."
  }

  assert {
    condition     = cloudflare_ruleset.redirects.rules[0].action_parameters.from_value.status_code == 301 && cloudflare_ruleset.redirects.rules[0].action_parameters.from_value.preserve_query_string && cloudflare_ruleset.redirects.rules[0].action_parameters.from_value.target_url.expression == "concat(\"https://holyfurries.com\", http.request.uri.path)"
    error_message = "The permanent redirect must preserve the path and query string and use HTTPS."
  }
}

run "cache_only_static_assets" {
  command = plan

  assert {
    condition     = cloudflare_ruleset.cache.phase == "http_request_cache_settings" && length(cloudflare_ruleset.cache.rules) == 2
    error_message = "Website caching must have a bypass default followed by a static asset exception."
  }

  assert {
    condition     = cloudflare_ruleset.cache.rules[0].enabled && !cloudflare_ruleset.cache.rules[0].action_parameters.cache && cloudflare_ruleset.cache.rules[0].expression == "http.host in {\"holyfurries.com\" \"www.holyfurries.com\"}"
    error_message = "All website requests, including events and health checks, must bypass cache by default."
  }

  assert {
    condition     = cloudflare_ruleset.cache.rules[1].enabled && cloudflare_ruleset.cache.rules[1].action_parameters.cache && cloudflare_ruleset.cache.rules[1].expression == "http.host eq \"holyfurries.com\" and starts_with(http.request.uri.path, \"/static/\") and http.request.method in {\"GET\" \"HEAD\"}"
    error_message = "Only static GET and HEAD requests on the canonical hostname may be cached."
  }

  assert {
    condition     = cloudflare_ruleset.cache.rules[1].action_parameters.edge_ttl.mode == "bypass_by_default" && cloudflare_ruleset.cache.rules[1].action_parameters.browser_ttl.mode == "respect_origin" && cloudflare_ruleset.cache.rules[1].action_parameters.cache_key == null
    error_message = "Asset caching must honor origin cache lifetimes and retain the default query-sensitive cache key."
  }
}
