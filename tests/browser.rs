#![expect(
    clippy::collapsible_if,
    reason = "Repository rules require explicit nested control flow"
)]
use reqwest::{Client, Method};
use serde_json::{Value, json};
use std::{
    io::Read,
    net::TcpListener,
    process::{Child, Command},
    time::Duration,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Run nix build .#checks.x86_64-linux.browser for pinned browsers and drivers"]
async fn browser_navigation_layout_images_motion_and_live_catalog() {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();
    let website_process = start_website(&client).await;
    let website = &website_process.url;
    let nats = &website_process.nats;

    for engine in ["chromium", "firefox"] {
        let driver_port = port();
        let program = if engine == "chromium" {
            "chromedriver"
        } else {
            "geckodriver"
        };
        let _driver = Process(
            Command::new(program)
                .arg(format!("--port={driver_port}"))
                .spawn()
                .unwrap(),
        );
        let driver = format!("http://127.0.0.1:{driver_port}");
        ready(&client, &format!("{driver}/status")).await;
        for reduced in [false, true] {
            let options = if engine == "chromium" {
                let mut arguments =
                    vec!["--headless=new", "--no-sandbox", "--disable-dev-shm-usage"];
                if reduced {
                    arguments.push("--force-prefers-reduced-motion");
                }
                json!({"goog:chromeOptions": {
                    "binary": std::env::var("BROWSER_CHROMIUM").unwrap(), "args": arguments
                }})
            } else {
                json!({"moz:firefoxOptions": {
                    "binary": std::env::var("BROWSER_FIREFOX").unwrap(), "args": ["-headless"],
                    "prefs": {"ui.prefersReducedMotion": i32::from(reduced)}
                }})
            };
            let session = request(
                &client,
                Method::POST,
                &format!("{driver}/session"),
                json!({"capabilities": {"alwaysMatch": options}}),
            )
            .await;
            let session = format!(
                "{driver}/session/{}",
                session["sessionId"].as_str().unwrap()
            );
            request(
                &client,
                Method::POST,
                &format!("{session}/window/rect"),
                json!({"width":1280,"height":1000}),
            )
            .await;
            request(
                &client,
                Method::POST,
                &format!("{session}/url"),
                json!({"url":website}),
            )
            .await;
            let motion = script(&client, &session, r#"
                const main = getComputedStyle(document.querySelector('main'));
                return {reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
                    property: main.transitionProperty, duration: main.transitionDuration, opacity: main.opacity};
            "#, json!([])).await;
            assert_eq!(motion["reduced"], reduced, "{engine}: {motion}");
            assert_eq!(motion["opacity"], "1");
            let hero = script(&client, &session, r#"
                const mark = document.querySelector('.hero_mark');
                const animations = mark.getAnimations({subtree: true});
                return {durations: animations.map(animation => animation.effect.getTiming().duration).sort((a, b) => a - b),
                    halo: getComputedStyle(mark.querySelector('.halo')).visibility,
                    title: getComputedStyle(document.querySelector('#hero_title')).opacity};
            "#, json!([])).await;
            assert_eq!(hero["title"], "1", "{engine}: hero copy stays visible");
            if reduced {
                assert_eq!(
                    hero["durations"],
                    json!([]),
                    "{engine}: reduced-motion hero"
                );
                assert_eq!(hero["halo"], "visible");
            } else {
                assert_eq!(
                    hero["durations"],
                    json!([255, 675, 870, 1065]),
                    "{engine}: hero timing"
                );
            }
            if reduced {
                assert_eq!(motion["duration"], "0s");
            } else {
                assert_eq!(motion["property"], "translate");
                assert_eq!(motion["duration"], "0.14s");
            }
            if !reduced {
                for route in [
                    "/",
                    "/games/crowns-adrift",
                    "/games/cold-orbit",
                    "/tools/spice-cad",
                    "/missing",
                ] {
                    request(
                        &client,
                        Method::POST,
                        &format!("{session}/url"),
                        json!({"url":format!("{website}{route}")}),
                    )
                    .await;
                    for width in [240, 320, 375, 640, 960, 1280] {
                        let fits = script(
                            &client,
                            &session,
                            r#"
                            document.body.style.width = 'min(' + arguments[0] + 'px, 100%)';
                            return document.body.scrollWidth <= document.body.clientWidth + 1;
                        "#,
                            json!([width]),
                        )
                        .await;
                        assert_eq!(fits, true, "{engine}: overflow on {route} at {width}");
                    }
                    let fits = script(
                        &client,
                        &session,
                        r#"
                        document.body.style.width = 'min(375px, 100%)';
                        document.documentElement.style.fontSize = '200%';
                        const fits = document.body.scrollWidth <= document.body.clientWidth + 1;
                        document.documentElement.style.fontSize = '';
                        document.body.style.width = '';
                        return fits;
                    "#,
                        json!([]),
                    )
                    .await;
                    assert_eq!(
                        fits, true,
                        "{engine}: overflow with enlarged text on {route}"
                    );
                    for width in [480, 768, 1280] {
                        request(
                            &client,
                            Method::POST,
                            &format!("{session}/window/rect"),
                            json!({"width":width,"height":1000}),
                        )
                        .await;
                        let fits = script(
                            &client,
                            &session,
                            "return document.documentElement.scrollWidth <= innerWidth",
                            json!([]),
                        )
                        .await;
                        assert_eq!(
                            fits, true,
                            "{engine}: viewport overflow on {route} at {width}"
                        );
                    }
                }
                request(
                    &client,
                    Method::POST,
                    &format!("{session}/url"),
                    json!({"url":website}),
                )
                .await;
                for width in [480, 1280] {
                    request(
                        &client,
                        Method::POST,
                        &format!("{session}/window/rect"),
                        json!({"width":width,"height":1000}),
                    )
                    .await;
                    if width == 480 {
                        let compact = script(&client, &session, r#"
                            const header = document.querySelector('.site_header').getBoundingClientRect();
                            return header.height <= 80 && [...document.querySelectorAll('.site_header nav a')]
                                .every(link => link.getBoundingClientRect().height >= 44);
                        "#, json!([])).await;
                        assert_eq!(
                            compact, true,
                            "{engine}: compact mobile navigation with usable tap targets"
                        );
                    }
                    script(&client, &session,
                        "window.scrollTo({top: document.documentElement.scrollHeight, behavior: 'instant'}); return true;",
                        json!([])).await;
                    wait(&client, &session, r#"
                        const header = document.querySelector('.site_header').getBoundingClientRect();
                        return Math.abs(header.top) < 1 && header.bottom < innerHeight;
                    "#).await;
                    click(&client, &session, ".site_header nav a[href='/#mods']").await;
                    wait(&client, &session, r#"
                        const title = document.querySelector('#mods_title').getBoundingClientRect();
                        const header = document.querySelector('.site_header').getBoundingClientRect();
                        const target = document.querySelector('#mods').getBoundingClientRect();
                        const padding = parseFloat(getComputedStyle(document.documentElement).scrollPaddingTop);
                        return location.hash === '#mods' && Math.abs(target.top - padding) < 2
                            && title.top >= header.bottom && title.top < innerHeight;
                    "#).await;
                    script(
                        &client,
                        &session,
                        "document.body.dataset.navigationTest = 'preserved'; return true;",
                        json!([]),
                    )
                    .await;
                    click(&client, &session, ".site_header .brand").await;
                    wait(
                        &client,
                        &session,
                        "return location.hash === '#top' && scrollY < 1",
                    )
                    .await;
                    let preserved = script(
                        &client,
                        &session,
                        "return document.body.dataset.navigationTest === 'preserved'",
                        json!([]),
                    )
                    .await;
                    assert_eq!(
                        preserved, true,
                        "{engine}: home logo scrolls without reloading"
                    );
                }
                let images = script(
                    &client,
                    &session,
                    r#"
                    for (const image of document.images) image.loading = 'eager';
                    return document.images.length;
                "#,
                    json!([]),
                )
                .await;
                assert!(images.as_u64().unwrap() >= 4);
                wait(&client, &session, "return [...document.images].every(image => image.complete && image.naturalWidth > 0)").await;
                click(&client, &session, ".crowns .text_link").await;
                wait(&client, &session, "return location.pathname === '/games/crowns-adrift' && document.querySelector('h1').textContent.includes('Crowns')").await;
                click(&client, &session, ".site_header .brand").await;
                wait(
                    &client,
                    &session,
                    "return location.pathname === '/' && location.hash === '#top' && scrollY < 1",
                )
                .await;
                request(&client, Method::POST, &format!("{session}/back"), json!({})).await;
                wait(
                    &client,
                    &session,
                    "return location.pathname === '/games/crowns-adrift'",
                )
                .await;
                request(&client, Method::POST, &format!("{session}/back"), json!({})).await;
                wait(&client, &session, "return location.pathname === '/'").await;
                click(&client, &session, ".site_header nav a").await;
                wait(
                    &client,
                    &session,
                    "return location.hash === '#games' && scrollY > 0",
                )
                .await;
                let version = if engine == "chromium" {
                    "9.8.7"
                } else {
                    "9.8.8"
                };
                nats.publish("holyfurries.catalog.release", serde_json::to_vec(&json!({
                    "package":"ClearMinimap", "version":version, "description":"Browser-tested minimap.", "downloads_total":42
                })).unwrap().into()).await.unwrap();
                nats.flush().await.unwrap();
                wait(&client, &session, &format!("return document.querySelector('#mod_ClearMinimap').textContent.includes('v{version}') && document.querySelector('#mod_ClearMinimap').textContent.includes('Browser-tested minimap.') && document.querySelector('#mod_ClearMinimap').textContent.includes('42 downloads total')")).await;
            }
            request(&client, Method::DELETE, &session, json!({})).await;
            eprintln!("Browser: {engine}, reduced motion {reduced} passed");
        }
    }
}

struct Website {
    process: Process,
    url: String,
    nats: async_nats::Client,
    nats_id: i32,
}

async fn start_website(client: &Client) -> Website {
    let website_port = port();
    let log = tempfile::NamedTempFile::new().unwrap();
    let process = Process(
        Command::new(env!("CARGO_BIN_EXE_holyfurries"))
            .env("LISTEN_ADDRESS", format!("127.0.0.1:{website_port}"))
            .stderr(log.reopen().unwrap())
            .spawn()
            .unwrap(),
    );
    let url = format!("http://127.0.0.1:{website_port}");
    ready(client, &format!("{url}/healthz")).await;
    let mut messages = String::new();
    log.reopen()
        .unwrap()
        .take(65536)
        .read_to_string(&mut messages)
        .unwrap();
    let endpoint = messages
        .lines()
        .find_map(|line| line.strip_prefix("NATS: "))
        .expect("website reports its private NATS endpoint");
    let mut fields = endpoint.split_whitespace();
    let nats = async_nats::connect(fields.next().unwrap()).await.unwrap();
    let nats_id: i32 = fields
        .next()
        .unwrap()
        .strip_prefix("pid=")
        .unwrap()
        .parse()
        .unwrap();
    assert!(nats_id > 0);
    assert!(process.0.id() != u32::try_from(nats_id).unwrap());
    Website {
        process,
        url,
        nats,
        nats_id,
    }
}

impl Drop for Website {
    fn drop(&mut self) {
        if self
            .process
            .0
            .try_wait()
            .expect("read website status")
            .is_none()
        {
            let process_id = i32::try_from(self.process.0.id()).unwrap();
            unsafe {
                libc::kill(process_id, libc::SIGTERM);
            }
            for _ in 0..250 {
                if self
                    .process
                    .0
                    .try_wait()
                    .expect("read website status")
                    .is_some()
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[tokio::test]
#[ignore = "Run the Nix browser check for the packaged NATS server"]
async fn website_owns_nats_startup_failure_and_shutdown() {
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    for stop_website in [true, false] {
        let mut website = start_website(&client).await;
        let mut stream = client
            .get(format!("{}/events", website.url))
            .send()
            .await
            .unwrap();
        assert!(stream.chunk().await.unwrap().is_some());
        let process_id = if stop_website {
            i32::try_from(website.process.0.id()).unwrap()
        } else {
            website.nats_id
        };
        let signal = if stop_website {
            libc::SIGTERM
        } else {
            libc::SIGKILL
        };
        assert_eq!(unsafe { libc::kill(process_id, signal) }, 0);
        let mut status = None;
        for _ in 0..200 {
            if let Some(exited) = website.process.0.try_wait().unwrap() {
                status = Some(exited);
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert_eq!(
            status.expect("website exits within five seconds").success(),
            stop_website
        );
        assert_eq!(
            unsafe { libc::kill(website.nats_id, 0) },
            -1,
            "NATS child must be reaped"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
        assert!(
            tokio::time::timeout(Duration::from_secs(2), stream.chunk())
                .await
                .is_ok()
        );
    }
}

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        if self.0.try_wait().expect("read child status").is_none() {
            self.0.kill().expect("stop child");
        }
        self.0.wait().expect("reap child");
    }
}

fn port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

async fn ready(client: &Client, url: &str) {
    for _ in 0..100 {
        if let Ok(response) = client.get(url).timeout(Duration::from_secs(1)).send().await {
            if response.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("Service did not become ready: {url}");
}

async fn request(client: &Client, method: Method, url: &str, body: Value) -> Value {
    let response = client
        .request(method, url)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let mut result: Value = response.json().await.unwrap();
    assert!(status.is_success(), "{url}: {result}");
    assert!(result["value"]["error"].is_null(), "{url}: {result}");
    result["value"].take()
}

async fn script(client: &Client, session: &str, code: &str, arguments: Value) -> Value {
    request(
        client,
        Method::POST,
        &format!("{session}/execute/sync"),
        json!({"script":code,"args":arguments}),
    )
    .await
}

async fn wait(client: &Client, session: &str, condition: &str) {
    for _ in 0..100 {
        if script(client, session, condition, json!([])).await == true {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("Browser condition did not become true: {condition}");
}

async fn click(client: &Client, session: &str, selector: &str) {
    script(
        client,
        session,
        "document.querySelector(arguments[0]).scrollIntoView({block:'center',behavior:'instant'})",
        json!([selector]),
    )
    .await;
    wait(
        client,
        session,
        &format!(
            r#"
        const element = document.querySelector({});
        const rect = element.getBoundingClientRect();
        const target = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
        return rect.width > 0 && rect.height > 0 && element.contains(target);
    "#,
            serde_json::to_string(selector).unwrap()
        ),
    )
    .await;
    let element = request(
        client,
        Method::POST,
        &format!("{session}/element"),
        json!({"using":"css selector","value":selector}),
    )
    .await;
    let element = element["element-6066-11e4-a52e-4f735466cecf"]
        .as_str()
        .unwrap();
    request(
        client,
        Method::POST,
        &format!("{session}/element/{element}/click"),
        json!({}),
    )
    .await;
}
