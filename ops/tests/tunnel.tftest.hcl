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
