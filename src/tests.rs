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
        assert!(
            response
                .headers()
                .contains_key(header::CONTENT_SECURITY_POLICY)
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
        assert!(html.contains("<h1"), "{path}");
        assert!(html.contains("rel=\"canonical\""), "{path}");
        assert!(html.contains("property=\"og:image\" content=\"https://holyfurries.com/static/images/studio-social.png\""), "{path}");
        assert!(
            html.contains("name=\"twitter:card\" content=\"summary_large_image\""),
            "{path}"
        );
        assert!(!html.contains("TODO"), "{path}");
        if path == "/tools/spice-cad" {
            assert!(html.contains("href=\"https://github.com/holyfurries/spice-cad\""));
            assert!(!html.contains("no public download"));
        }
        for destination in html
            .split("href=\"")
            .skip(1)
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
    for (path, mime) in [
        ("/static/images/studio-social.png", "image/png"),
        ("/static/images/crowns-adrift-city.webp", "image/webp"),
        ("/static/images/cold-orbit-flight.webp", "image/webp"),
        ("/static/images/spice-cad-editor.webp", "image/webp"),
        (
            "/static/images/crowns-adrift-city-preview.webp",
            "image/webp",
        ),
        (
            "/static/images/cold-orbit-flight-preview.webp",
            "image/webp",
        ),
        ("/static/images/spice-cad-editor-preview.webp", "image/webp"),
        ("/static/images.js", "text/javascript; charset=utf-8"),
        ("/static/navigation.js", "text/javascript; charset=utf-8"),
        (
            "/static/vendor/datastar-1.0.4.js",
            "text/javascript; charset=utf-8",
        ),
        ("/sitemap.xml", "application/xml"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], mime);
    }
    for path in [
        "/missing",
        "/games/no-such-game",
        "/static/../Cargo.toml",
        "/static/missing",
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
