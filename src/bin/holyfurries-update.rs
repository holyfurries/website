use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

type UpdateError = Box<dyn Error + Send + Sync>;
const PROFILE: &str = "/nix/var/nix/profiles/holyfurries";
const ARCHIVE: &str = "holyfurries-x86_64-linux.tar.gz";
const ARCHIVE_SIZE_MAX: usize = 128 * 1024 * 1024;

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), UpdateError> {
    let port: u16 = std::env::var("HOLYFURRIES_PORT")?.parse()?;
    let health_url = format!("http://127.0.0.1:{port}/healthz");
    let client = Client::builder()
        .user_agent("holyfurries-update")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()?;
    let response = client
        .get("https://api.github.com/repos/holyfurries/website/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(());
    }
    let metadata = response_bytes(response.error_for_status()?, 1024 * 1024).await?;
    let release: Release = serde_json::from_slice(&metadata)?;
    validate_tag(&release.tag_name)?;
    if release.draft || release.prerelease {
        return Err("Expected a published stable release".into());
    }
    let previous = fs::canonicalize(PROFILE)?;
    let previous = previous.to_str().ok_or("Invalid profile path")?;
    let state = format!("{}\n{previous}\n", release.tag_name);
    match fs::read_to_string("deployed") {
        Ok(deployed) if deployed == state => return Ok(()),
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }

    let base = format!(
        "https://github.com/holyfurries/website/releases/download/{}",
        release.tag_name
    );
    let checksum = response_bytes(
        client
            .get(format!("{base}/{ARCHIVE}.sha256"))
            .send()
            .await?
            .error_for_status()?,
        256,
    )
    .await?;
    let checksum = std::str::from_utf8(&checksum)?;
    validate_checksum(checksum)?;
    fs::write(format!("{ARCHIVE}.sha256"), checksum)?;
    let mut response = client
        .get(format!("{base}/{ARCHIVE}"))
        .send()
        .await?
        .error_for_status()?;
    let mut archive = File::create(ARCHIVE)?;
    let mut size_bytes: usize = 0;
    while let Some(chunk) = response.chunk().await? {
        size_bytes = size_bytes
            .checked_add(chunk.len())
            .ok_or("Archive size overflow")?;
        if size_bytes > ARCHIVE_SIZE_MAX {
            return Err("Release archive exceeds size limit".into());
        }
        archive.write_all(&chunk)?;
    }
    archive.sync_all()?;
    command("sha256sum", &["--check", &format!("{ARCHIVE}.sha256")])?;
    command(
        "tar",
        &[
            "--extract",
            "--gzip",
            "--file",
            ARCHIVE,
            "--no-same-owner",
            "--no-same-permissions",
            "./store-path",
            "./closure.nar",
        ],
    )?;
    let package = fs::read_to_string("store-path")?;
    let package = package.trim();
    validate_package(package)?;
    if !package.ends_with(&format!("-holyfurries-{}", &release.tag_name[1..])) {
        return Err("Package version does not match the release tag".into());
    }
    let status = Command::new("nix-store")
        .arg("--import")
        .stdin(File::open("closure.nar")?)
        .stdout(Stdio::null())
        .status()?;
    if !status.success() {
        return Err("Release closure import failed".into());
    }
    command("nix-store", &["--verify-path", package])?;
    activate(&client, &health_url, previous, package, command).await?;
    fs::write(
        "deployed.next",
        format!("{}\n{package}\n", release.tag_name),
    )?;
    fs::rename("deployed.next", "deployed")?;
    fs::remove_file("closure.nar")?;
    fs::remove_file(ARCHIVE)?;
    eprintln!("Deploy: {} ({package})", release.tag_name);
    Ok(())
}

async fn activate(
    client: &Client,
    health_url: &str,
    previous: &str,
    package: &str,
    run: impl Fn(&str, &[&str]) -> Result<(), UpdateError>,
) -> Result<(), UpdateError> {
    run("nix-env", &["--profile", PROFILE, "--set", package])?;
    let activation = async {
        run("systemctl", &["restart", "holyfurries.service"])?;
        check_health(client, health_url).await
    }
    .await;
    if let Err(error) = activation {
        run("nix-env", &["--profile", PROFILE, "--set", previous])?;
        run("systemctl", &["restart", "holyfurries.service"])?;
        check_health(client, health_url).await?;
        return Err(format!("Deployment rolled back: {error}").into());
    }
    Ok(())
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

fn validate_tag(tag: &str) -> Result<(), UpdateError> {
    if tag.len() > 64 {
        return Err("Release tag exceeds size limit".into());
    }
    let version = tag
        .strip_prefix('v')
        .ok_or("Release tag must start with v")?;
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("Expected a three-part release version".into());
    }
    for part in parts {
        if part.is_empty() {
            return Err("Empty release version component".into());
        }
        if !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("Invalid release version component".into());
        }
    }
    Ok(())
}

fn validate_checksum(checksum: &str) -> Result<(), UpdateError> {
    let expected_suffix = format!("  {ARCHIVE}\n");
    let hash = checksum
        .strip_suffix(&expected_suffix)
        .ok_or("Unexpected checksum filename")?;
    if hash.len() != 64 {
        return Err("Invalid checksum length".into());
    }
    if !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid checksum encoding".into());
    }
    Ok(())
}

fn validate_package(package: &str) -> Result<(), UpdateError> {
    let name = package
        .strip_prefix("/nix/store/")
        .ok_or("Expected a Nix store path")?;
    let (hash, version) = name
        .split_once("-holyfurries-")
        .ok_or("Unexpected package name")?;
    if hash.len() != 32 {
        return Err("Invalid store hash length".into());
    }
    if !hash
        .bytes()
        .all(|byte| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&byte))
    {
        return Err("Invalid store hash encoding".into());
    }
    validate_tag(&format!("v{version}"))?;
    if Path::new(package).components().count() != 4 {
        return Err("Expected a direct store path".into());
    }
    Ok(())
}

async fn response_bytes(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, UpdateError> {
    assert!(limit > 0);
    assert!(limit <= 1024 * 1024);
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > limit - bytes.len() {
            return Err("Response exceeds size limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn command(program: &str, arguments: &[&str]) -> Result<(), UpdateError> {
    let status = Command::new(program).args(arguments).status()?;
    if !status.success() {
        return Err(format!("{program} failed: {status}").into());
    }
    Ok(())
}

async fn check_health(client: &Client, url: &str) -> Result<(), UpdateError> {
    for _ in 0..20 {
        if let Ok(response) = client.get(url).timeout(Duration::from_secs(2)).send().await {
            if response.status() == StatusCode::OK {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err("Website health check failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Read, net::TcpListener, sync::Mutex};

    #[tokio::test]
    async fn unhealthy_release_restores_the_previous_package_and_checks_it() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/healthz", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(20);
            let mut responses = 0;
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = [0; 1024];
                        stream.read(&mut request).unwrap();
                        responses += 1;
                        let status = if responses <= 20 {
                            "503 Unavailable"
                        } else {
                            "200 OK"
                        };
                        write!(
                            stream,
                            "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        )
                        .unwrap();
                        if responses == 21 {
                            return responses;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            panic!("Health requests did not complete");
        });
        let calls = Mutex::new(Vec::new());
        let result = activate(
            &Client::new(),
            &url,
            "previous",
            "candidate",
            |program, arguments| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("{program} {}", arguments.join(" ")));
                Ok(())
            },
        )
        .await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .starts_with("Deployment rolled back:")
        );
        assert_eq!(server.join().unwrap(), 21);
        assert_eq!(
            *calls.lock().unwrap(),
            [
                format!("nix-env --profile {PROFILE} --set candidate"),
                "systemctl restart holyfurries.service".into(),
                format!("nix-env --profile {PROFILE} --set previous"),
                "systemctl restart holyfurries.service".into(),
            ]
        );
    }

    #[test]
    fn release_paths_are_bounded_and_cannot_escape_their_directory() {
        assert!(validate_tag("v0.1.0").is_ok());
        for tag in [
            "main",
            "v1",
            "v1..0",
            "v1.0.0/../other",
            "v1.0.0-rc1",
            "v１.0.0",
        ] {
            assert!(validate_tag(tag).is_err(), "{tag}");
        }
        assert!(validate_tag(&format!("v{}.0.0", "1".repeat(64))).is_err());
        assert!(
            validate_package("/nix/store/16jpdp386w4r21yvw2dh253fdkj1c55d-holyfurries-0.1.0")
                .is_ok()
        );
        for path in [
            "/tmp/holyfurries-0.1.0",
            "/nix/store/16jpdp386w4r21yvw2dh253fdkj1c55d-holyfurries-0.1.0/bin/holyfurries",
            "/nix/store/16jpdp386w4r21yvw2dh253fdkj1c55d-other-0.1.0",
        ] {
            assert!(validate_package(path).is_err(), "{path}");
        }
    }

    #[test]
    fn checksum_manifest_can_only_reference_the_release_archive() {
        let hash = "a".repeat(64);
        assert!(validate_checksum(&format!("{hash}  {ARCHIVE}\n")).is_ok());
        assert!(validate_checksum(&format!("{hash}  /etc/shadow\n")).is_err());
        assert!(validate_checksum(&format!("{hash}  {ARCHIVE}\n{hash}  other\n")).is_err());
        assert!(validate_checksum(&format!("{}  {ARCHIVE}\n", "z".repeat(64))).is_err());
    }
}
