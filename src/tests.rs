use super::*;
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn test_state() -> AppState {
    AppState {
        releases: watch::channel(Vec::new()).0,
        streams: Arc::new(Semaphore::new(2)),
        nats: None,
        stopping: CancellationToken::new(),
    }
}

#[tokio::test]
async fn public_pages_links_metadata_and_assets() {
    let app = router(test_state());
    for path in [
        "/",
        "/games/crowns-adrift",
        "/games/cold-orbit",
        "/tools/spice-cad",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        let policy = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        let script_sources = policy
            .split(';')
            .map(str::trim)
            .find(|directive| directive.starts_with("script-src "))
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>();
        let nonce = script_sources
            .iter()
            .find_map(|source| {
                source
                    .strip_prefix("'nonce-")
                    .and_then(|value| value.strip_suffix('\''))
            })
            .map(str::to_owned);
        assert!(!policy.contains("unsafe-eval"));
        assert_eq!(
            script_sources
                .iter()
                .copied()
                .filter(|source| !source.starts_with("'nonce-"))
                .collect::<Vec<_>>(),
            [
                "script-src",
                "'self'",
                "https://static.cloudflareinsights.com/beacon.min.js",
                "https://static.cloudflareinsights.com/beacon.min.js/",
            ],
            "{path}: only the studio and Cloudflare beacon may supply scripts"
        );
        if path == "/" {
            let value = nonce.as_ref().unwrap();
            assert_eq!(value.len(), 32);
            assert!(value.bytes().all(|byte| byte.is_ascii_hexdigit()));
            let another = app
                .clone()
                .oneshot(Request::get("/").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_ne!(
                response.headers()[header::CONTENT_SECURITY_POLICY],
                another.headers()[header::CONTENT_SECURITY_POLICY]
            );
        } else {
            assert!(nonce.is_none());
        }
        assert!(
            policy
                .split(';')
                .any(|directive| directive.trim() == "connect-src 'self'"),
            "{path}: telemetry and live updates must stay same-origin"
        );
        let html = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        if let Some(nonce) = nonce {
            assert!(html.contains(&format!("data-nonce=\"{nonce}\"")));
            assert!(html.contains(assets::url("vendor/datastar-1.0.4.js")));
        } else {
            assert!(!html.contains("datastar-1.0.4.js"));
        }
        assert!(html.contains("<h1"), "{path}");
        assert!(html.contains("rel=\"canonical\""), "{path}");
        assert!(
            html.contains(&format!(
                "property=\"og:image\" content=\"https://holyfurries.com{}\"",
                assets::url("images/studio-social.png")
            )),
            "{path}"
        );
        assert!(
            html.contains("name=\"twitter:card\" content=\"summary_large_image\""),
            "{path}"
        );
        let description = html
            .split("name=\"description\" content=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        if path != "/" {
            assert!(
                html.contains(&format!(
                    "property=\"og:description\" content=\"{description}\""
                )),
                "{path}: sharing uses the project description"
            );
        }
        assert!(!html.contains("TODO"), "{path}");
        if path == "/games/crowns-adrift" {
            assert!(html.contains("id=\"how-it-plays\""));
            assert!(html.contains("crowns-adrift-foundry.webp"));
            assert!(html.contains("paid Steam release"));
            assert!(!html.contains("store.steampowered.com/app/"));
        }
        if path == "/tools/spice-cad" {
            assert!(html.contains("href=\"https://github.com/holyfurries/spice-cad\""));
            assert!(!html.contains("no public download"));
        }
        for sources in html.split("srcset=\"").skip(1) {
            for candidate in sources.split('"').next().unwrap().split(',') {
                let mut parts = candidate.split_whitespace();
                let target = parts.next().unwrap();
                let width = parts.next().unwrap().strip_suffix('w').unwrap();
                assert!(width.parse::<u32>().unwrap() > 0);
                assert!(parts.next().is_none());
                let response = app
                    .clone()
                    .oneshot(Request::get(target).body(Body::empty()).unwrap())
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK, "{path} -> {target}");
                assert_eq!(response.headers()[header::CONTENT_TYPE], "image/webp");
            }
        }
        for destination in html
            .split("href=\"")
            .skip(1)
            .chain(html.split("src=\"").skip(1))
            .filter_map(|part| part.split('"').next())
        {
            if destination.starts_with('/') {
                let target = destination.split('#').next().unwrap();
                let response = app
                    .clone()
                    .oneshot(Request::builder().uri(target).body(Body::empty()).unwrap())
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK, "{path} -> {target}");
            }
        }
    }
    for asset in assets::ASSETS {
        use sha2::{Digest, Sha256};
        let response = app
            .clone()
            .oneshot(Request::get(asset.url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{}", asset.url);
        assert_eq!(response.headers()[header::CONTENT_TYPE], asset.content_type);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            asset.url,
            format!("/static/{:x}/{}", Sha256::digest(&bytes), asset.path)
        );
        let stale_url = asset.url.replacen("/static/", "/static/invalid", 1);
        let response = app
            .clone()
            .oneshot(Request::get(stale_url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let css = assets::ASSETS
        .iter()
        .find(|asset| asset.path == "site.css")
        .unwrap();
    let css = std::str::from_utf8(css.bytes).unwrap();
    for preview in assets::ASSETS
        .iter()
        .filter(|asset| asset.path.ends_with("-preview.webp"))
    {
        assert!(css.contains(preview.url));
        assert!(!css.contains(&format!("/static/{}", preview.path)));
    }
    let response = app
        .clone()
        .oneshot(Request::get("/missing").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
    let html = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains(assets::url("site.css")));
    for path in [
        "/missing",
        "/games/no-such-game",
        "/static/../Cargo.toml",
        "/static/missing",
        "/static/site.css",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[test]
fn only_known_packages_and_bounded_versions_are_accepted() {
    for version in ["", "<script>", "1.0\ninjected", "1.0.0\"", &"1".repeat(33)] {
        assert!(
            !Release {
                package: "ClearMinimap".into(),
                version: version.into(),
                description: "A minimap.".into(),
                downloads_total: 0
            }
            .valid()
        );
    }
    assert!(
        !Release {
            package: "OtherOwner".into(),
            version: "1.0.0".into(),
            description: "A minimap.".into(),
            downloads_total: 0
        }
        .valid()
    );
    assert!(
        Release {
            package: "ClearMinimap".into(),
            version: "0.2.0".into(),
            description: "A minimap.".into(),
            downloads_total: 0
        }
        .valid()
    );
}

#[test]
fn fetched_descriptions_are_escaped_and_missing_metadata_keeps_the_written_copy() {
    let fallback = Mods {
        mods: &MODS,
        releases: &[],
    }
    .render()
    .unwrap();
    assert!(fallback.contains(MODS[0].description));
    assert!(!fallback.contains("mod_downloads"));
    let releases = [Release {
        package: "ClearMinimap".into(),
        version: "0.2.1".into(),
        description: "<img src=x onerror=alert(1)>".into(),
        downloads_total: 42,
    }];
    let html = Mods {
        mods: &MODS,
        releases: &releases,
    }
    .render()
    .unwrap();
    assert!(!html.contains("<img src=x"));
    assert!(html.contains("42 downloads total"));
    assert!(html.contains("v0.2.1"));
}

#[test]
fn combined_downloads_require_every_mod_and_sum_without_overflow() {
    let mut releases: Vec<Release> = MODS
        .iter()
        .map(|item| Release {
            package: item.package.into(),
            version: "1.0.0".into(),
            description: item.description.into(),
            downloads_total: u64::MAX,
        })
        .collect();
    let total = u128::from(u64::MAX) * MODS.len() as u128;
    assert_eq!(catalog::downloads_total(&releases), Some(total));
    let html = Mods {
        mods: &MODS,
        releases: &releases,
    }
    .render()
    .unwrap();
    assert!(html.contains(&format!("{total} total downloads across all 5 mods")));
    releases.pop();
    assert_eq!(catalog::downloads_total(&releases), None);
    let partial = Mods {
        mods: &MODS,
        releases: &releases,
    }
    .render()
    .unwrap();
    assert!(!partial.contains("mods_downloads_total"));
    assert_eq!(catalog::downloads_total(&[]), None);
}

#[tokio::test]
async fn streams_are_bounded_release_capacity_and_close_on_shutdown() {
    let state = test_state();
    let app = router(state.clone());
    let first = app
        .clone()
        .oneshot(Request::get("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let second = app
        .clone()
        .oneshot(Request::get("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let third = app
        .clone()
        .oneshot(Request::get("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::SERVICE_UNAVAILABLE);
    drop(second);
    assert_eq!(state.streams.available_permits(), 1);
    state.stopping.cancel();
    tokio::time::timeout(Duration::from_secs(1), first.into_body().collect())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.streams.available_permits(), 2);
}

#[tokio::test]
async fn nats_release_reaches_an_existing_datastar_stream() {
    let (mut server, client) = nats::Server::start(nats::PROGRAM).await.unwrap();
    let state = test_state();
    let subscriber = client.subscribe(RELEASE_SUBJECT).await.unwrap();
    client.flush().await.unwrap();
    let receiver = tokio::spawn(receive_releases(subscriber, state.releases.clone()));
    let response = router(state.clone())
        .oneshot(Request::get("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/event-stream"
    );
    let mut stream = response.into_body().into_data_stream();
    let initial = stream.next().await.unwrap().unwrap();
    let initial = std::str::from_utf8(&initial).unwrap();
    assert!(initial.contains("event: datastar-patch-elements"));
    assert!(initial.contains("data: elements <div id=\"mod_catalog\""));
    assert!(!initial.contains("data: data:"));
    client
        .publish(
            RELEASE_SUBJECT,
            r#"{"package":"ClearMinimap","version":"9.8.7","description":"Updated minimap.","downloads_total":42}"#.into(),
        )
        .await
        .unwrap();
    client.flush().await.unwrap();
    let update = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let update = std::str::from_utf8(&update).unwrap();
    assert!(update.contains("v9.8.7"));
    assert!(update.contains("Updated minimap."));
    assert!(update.contains("42 downloads total"));
    let mut changes = state.releases.subscribe();
    changes.borrow_and_update();
    for payload in [
        r#"{"package":"ClearMinimap","version":"<script>"}"#,
        r#"{"package":"Unknown","version":"1.0"}"#,
        "not json",
    ] {
        client
            .publish(RELEASE_SUBJECT, payload.into())
            .await
            .unwrap();
    }
    client.flush().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), changes.changed())
            .await
            .is_err()
    );
    assert_eq!(state.releases.borrow().len(), 1);
    state.stopping.cancel();
    receiver.abort();
    server.stop().await.unwrap();
}
