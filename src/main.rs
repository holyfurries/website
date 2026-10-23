#![expect(
    clippy::collapsible_if,
    reason = "Repository rules require explicit nested control flow"
)]
use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{
        Html, IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::{Stream, StreamExt};
use std::{
    convert::Infallible,
    error::Error,
    future::{Future, IntoFuture},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Semaphore, watch};
use tokio_stream::wrappers::WatchStream;
use tokio_util::sync::CancellationToken;
use tower_http::{compression::CompressionLayer, set_header::SetResponseHeaderLayer};

mod catalog;
mod nats;
use catalog::{MODS, Release};

type ServiceError = Box<dyn Error + Send + Sync>;
const RELEASE_SUBJECT: &str = "holyfurries.catalog.release";

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), ServiceError> {
    let shutdown = shutdown()?;
    let address: SocketAddr = std::env::var("LISTEN_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    let (mut server, nats) = nats::Server::start(nats::PROGRAM).await?;
    let result = serve(listener, &mut server, nats, shutdown).await;
    let cleanup = server.stop().await;
    if let Err(error) = &cleanup {
        eprintln!("NATS cleanup: {error}");
    }
    result?;
    cleanup
}

async fn serve(
    listener: tokio::net::TcpListener,
    server: &mut nats::Server,
    nats: async_nats::Client,
    shutdown: impl Future<Output = ()>,
) -> Result<(), ServiceError> {
    let subscriber =
        tokio::time::timeout(Duration::from_secs(2), nats.subscribe(RELEASE_SUBJECT)).await??;
    tokio::time::timeout(Duration::from_secs(2), nats.flush()).await??;
    let (releases, _) = watch::channel(Vec::new());
    let stopping = CancellationToken::new();
    let state = AppState {
        releases: releases.clone(),
        streams: Arc::new(Semaphore::new(256)),
        nats: Some(nats.clone()),
        stopping: stopping.clone(),
    };
    let receiver = tokio::spawn(receive_releases(subscriber, releases));
    let worker = tokio::spawn(refresh_releases(nats.clone()));
    eprintln!("Web: http://{}", listener.local_addr()?);
    let serving = axum::serve(listener, router(state))
        .with_graceful_shutdown(stopping.clone().cancelled_owned())
        .into_future();
    tokio::pin!(serving);
    let result: Result<(), ServiceError> = tokio::select! {
        biased;
        _ = shutdown => {
            stopping.cancel();
            match tokio::time::timeout(Duration::from_secs(10), &mut serving).await {
                Ok(result) => result.map_err(Into::into),
                Err(error) => Err(error.into()),
            }
        }
        status = server.wait() => match status {
            Ok(status) => Err(format!("Private NATS server exited: {status}").into()),
            Err(error) => Err(error),
        },
        result = &mut serving => result.map_err(Into::into),
    };
    stopping.cancel();
    worker.abort();
    receiver.abort();
    match tokio::time::timeout(Duration::from_secs(2), nats.drain()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => eprintln!("NATS drain: {error}"),
        Err(error) => eprintln!("NATS drain: {error}"),
    }
    result
}

#[derive(Clone)]
struct AppState {
    releases: watch::Sender<Vec<Release>>,
    streams: Arc<Semaphore>,
    nats: Option<async_nats::Client>,
    stopping: CancellationToken,
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/games/{slug}", get(game))
        .route("/tools/spice-cad", get(tool))
        .route("/events", get(events))
        .route("/healthz", get(health))
        .route("/static/{*path}", get(asset))
        .route("/robots.txt", get(|| async { "User-agent: *\nAllow: /\nSitemap: https://holyfurries.com/sitemap.xml\n" }))
        .route("/sitemap.xml", get(|| async { ([(header::CONTENT_TYPE, "application/xml")], include_str!("../static/sitemap.xml")) }))
        .fallback(|| async { (StatusCode::NOT_FOUND, Html(include_str!("../templates/404.html"))) })
        .layer(CompressionLayer::new())
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin")))
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")))
        .with_state(state)
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home<'a> {
    mods: &'a [catalog::Mod],
    releases: &'a [Release],
}

#[derive(Template)]
#[template(path = "mods.html")]
struct Mods<'a> {
    mods: &'a [catalog::Mod],
    releases: &'a [Release],
}

async fn home(State(state): State<AppState>) -> Response {
    render(
        Home {
            mods: &MODS,
            releases: &state.releases.borrow(),
        }
        .render(),
    )
}

fn render(result: Result<String, askama::Error>) -> Response {
    match result {
        Ok(html) => ([(header::CACHE_CONTROL, "no-cache")], Html(html)).into_response(),
        Err(error) => {
            eprintln!("Template: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The page could not be loaded.",
            )
                .into_response()
        }
    }
}

#[derive(Template)]
#[template(path = "crowns.html")]
struct Crowns;

#[derive(Template)]
#[template(path = "project.html")]
struct Project {
    path: &'static str,
    name: &'static str,
    category: &'static str,
    tagline: &'static str,
    description: &'static str,
    image: &'static str,
    image_width: u32,
    image_height: u32,
    image_description: &'static str,
    repository: Option<&'static str>,
    features: [(&'static str, &'static str); 3],
}

async fn game(Path(slug): Path<String>) -> Response {
    let project = match slug.as_str() {
        "crowns-adrift" => return render(Crowns.render()),
        "cold-orbit" => Project {
            path: "games/cold-orbit",
            name: "Cold Orbit",
            category: "Ship-building sandbox · In development",
            tagline: "Build and fly your own ship.",
            description: "A top-down ship-building sandbox. Assemble a ship in the editor, launch it into space, and manage its systems and crew.",
            image: "cold-orbit-flight.webp",
            image_width: 1413,
            image_height: 1004,
            image_description: "Cold Orbit development screenshot: a custom white spacecraft under thrust",
            repository: None,
            features: [
                (
                    "Shipbuilding",
                    "Assemble hulls, armor, reactors, weapons, quarters, and directional thrusters in the shipyard.",
                ),
                (
                    "Ship systems",
                    "Fuel, power, heat, and cabin pressure shape how your ship flies and how its crew survives.",
                ),
                (
                    "Crew and interiors",
                    "Switch between exterior and cutaway views to inspect rooms, equipment, and crew.",
                ),
            ],
        },
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Html(include_str!("../templates/404.html")),
            )
                .into_response();
        }
    };
    render(project.render())
}

async fn tool() -> Response {
    render(Project {
        path: "tools/spice-cad", name: "Spice CAD", category: "Desktop CAD modeler · In development",
        tagline: "A desktop CAD modeler for scripted designs.",
        description: "Write models in Luau and inspect the resulting geometry in a native Rust application. The project began as a parametric spice-rack model.",
        image: "spice-cad-editor.webp", image_width: 1700, image_height: 1050,
        image_description: "Spice CAD showing typed Luau code, a rounded cube, and size and corner-radius controls",
        repository: Some("https://github.com/holyfurries/spice-cad"),
        features: [("Luau editor", "An integrated Luau editor keeps your construction logic alongside the resulting model."), ("3D viewport", "Orbit the viewport, select geometry, and inspect dimensions in the native desktop interface."), ("Parametric parts", "The original spice-rack model uses dimensions for the cabinet, jars, and printable parts.")],
    }.render())
}

async fn health(State(state): State<AppState>) -> StatusCode {
    if let Some(nats) = &state.nats {
        if nats.connection_state() != async_nats::connection::State::Connected {
            return StatusCode::SERVICE_UNAVAILABLE;
        }
    }
    StatusCode::OK
}

async fn events(State(state): State<AppState>) -> Response {
    let Ok(permit) = state.streams.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let stream = WatchStream::new(state.releases.subscribe())
        .map(move |releases| {
            let _permit = &permit;
            match (Mods {
                mods: &MODS,
                releases: &releases,
            })
            .render()
            {
                Ok(html) => Ok::<Event, Infallible>(
                    Event::default().event("datastar-patch-elements").data(
                        html.lines()
                            .map(|line| format!("elements {line}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                ),
                Err(error) => {
                    eprintln!("Stream: {error}");
                    Ok(Event::default().comment("Rendering unavailable"))
                }
            }
        })
        .take_until(state.stopping.cancelled_owned())
        .take_until(tokio::time::sleep(Duration::from_secs(300)));
    stream_response(stream)
}

fn stream_response(
    stream: impl Stream<Item = Result<Event, Infallible>> + Send + 'static,
) -> Response {
    let response = Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20)));
    (
        [
            (header::CACHE_CONTROL, "no-cache"),
            ("x-accel-buffering".parse().expect("valid header"), "no"),
        ],
        response,
    )
        .into_response()
}

async fn receive_releases(
    mut subscriber: async_nats::Subscriber,
    releases: watch::Sender<Vec<Release>>,
) {
    while let Some(message) = subscriber.next().await {
        if message.payload.len() > 2048 {
            continue;
        }
        let Ok(release) = serde_json::from_slice::<Release>(&message.payload) else {
            continue;
        };
        if !release.valid() {
            continue;
        }
        releases.send_if_modified(|current| {
            if let Some(previous) = current
                .iter_mut()
                .find(|item| item.package == release.package)
            {
                if *previous == release {
                    return false;
                }
                *previous = release;
            } else {
                current.push(release);
            }
            assert!(current.len() <= MODS.len());
            true
        });
    }
}

async fn refresh_releases(nats: async_nats::Client) {
    let client = match reqwest::Client::builder()
        .user_agent(concat!(
            "HolyFurries/",
            env!("CARGO_PKG_VERSION"),
            " (+https://holyfurries.com)"
        ))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Catalog client: {error}");
            return;
        }
    };
    let mut interval = tokio::time::interval(Duration::from_secs(1800));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        for item in &MODS {
            match catalog::fetch_release(&client, item.package).await {
                Ok(release) => match serde_json::to_vec(&release) {
                    Ok(payload) => {
                        if let Err(error) = nats.publish(RELEASE_SUBJECT, payload.into()).await {
                            eprintln!("Catalog publish: {error}");
                        }
                    }
                    Err(error) => eprintln!("Catalog encoding: {error}"),
                },
                Err(error) => eprintln!("Catalog {}: {error}", item.package),
            }
        }
    }
}

fn shutdown() -> std::io::Result<impl Future<Output = ()>> {
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(unix)]
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    Ok(async move {
        #[cfg(unix)]
        tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
        #[cfg(not(unix))]
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("Signal: {error}");
        }
    })
}

async fn asset(Path(path): Path<String>) -> Response {
    let (content_type, bytes): (&str, &[u8]) = match path.as_str() {
        "site.css" => (
            "text/css; charset=utf-8",
            include_bytes!("../static/site.css"),
        ),
        "mark.svg" => ("image/svg+xml", include_bytes!("../static/mark.svg")),
        "favicon.svg" => ("image/svg+xml", include_bytes!("../static/favicon.svg")),
        "images.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!("../static/images.js"),
        ),
        "navigation.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!("../static/navigation.js"),
        ),
        "vendor/datastar-1.0.4.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!("../static/vendor/datastar-1.0.4.js"),
        ),
        "images/studio-social.png" => (
            "image/png",
            include_bytes!("../static/images/studio-social.png"),
        ),
        "images/crowns-adrift-island.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-island.webp"),
        ),
        "images/crowns-adrift-detail.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-detail.webp"),
        ),
        "images/crowns-adrift-detail-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-detail-preview.webp"),
        ),
        "images/crowns-adrift-fields.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-fields.webp"),
        ),
        "images/crowns-adrift-fields-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-fields-preview.webp"),
        ),
        "images/crowns-adrift-granary.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-granary.webp"),
        ),
        "images/crowns-adrift-granary-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-granary-preview.webp"),
        ),
        "images/crowns-adrift-foundry.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-foundry.webp"),
        ),
        "images/crowns-adrift-foundry-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-foundry-preview.webp"),
        ),
        "images/crowns-adrift-levy.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-levy.webp"),
        ),
        "images/crowns-adrift-levy-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-levy-preview.webp"),
        ),
        "images/crowns-adrift-commander.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-commander.webp"),
        ),
        "images/crowns-adrift-commander-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-commander-preview.webp"),
        ),
        "images/cold-orbit-flight.webp" => (
            "image/webp",
            include_bytes!("../static/images/cold-orbit-flight.webp"),
        ),
        "images/spice-cad-editor.webp" => (
            "image/webp",
            include_bytes!("../static/images/spice-cad-editor.webp"),
        ),
        "images/crowns-adrift-island-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/crowns-adrift-island-preview.webp"),
        ),
        "images/cold-orbit-flight-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/cold-orbit-flight-preview.webp"),
        ),
        "images/spice-cad-editor-preview.webp" => (
            "image/webp",
            include_bytes!("../static/images/spice-cad-editor-preview.webp"),
        ),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        bytes,
    )
        .into_response()
}

#[cfg(test)]
mod tests;
