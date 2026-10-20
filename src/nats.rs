use crate::ServiceError;
use serde::Deserialize;
use std::{io::Read, net::SocketAddr, path::Path, process::ExitStatus, time::Duration};
use tokio::process::{Child, Command};

pub const PROGRAM: &str = match option_env!("NATS_SERVER") {
    Some(program) => program,
    None => "nats-server",
};

pub struct Server {
    child: Child,
    directory: tempfile::TempDir,
}

impl Server {
    pub async fn start(program: &str) -> Result<(Self, async_nats::Client), ServiceError> {
        let directory = tempfile::Builder::new()
            .prefix("holyfurries-nats-")
            .tempdir()?;
        let config = directory.path().join("server.json");
        std::fs::write(
            &config,
            br#"{"host":"127.0.0.1","port":-1,"max_payload":65536}"#,
        )?;
        let child = Command::new(program)
            .arg("--config")
            .arg(config)
            .arg("--ports_file_dir")
            .arg(directory.path())
            .kill_on_drop(true)
            .spawn()?;
        let mut server = Self { child, directory };
        match server.connect(program).await {
            Ok(client) => Ok((server, client)),
            Err(error) => {
                if let Err(cleanup) = server.stop().await {
                    eprintln!("NATS cleanup: {cleanup}");
                }
                Err(error)
            }
        }
    }

    async fn connect(&mut self, program: &str) -> Result<async_nats::Client, ServiceError> {
        let process_id = self.child.id().ok_or("NATS has no process ID")?;
        let name = Path::new(program)
            .file_name()
            .ok_or("NATS executable has no filename")?;
        let ports = self
            .directory
            .path()
            .join(format!("{}_{process_id}.ports", name.to_string_lossy()));
        for _ in 0..100 {
            if let Some(status) = self.child.try_wait()? {
                return Err(format!("NATS exited during startup: {status}").into());
            }
            match std::fs::File::open(&ports) {
                Ok(file) => {
                    let mut bytes = Vec::new();
                    file.take(4097).read_to_end(&mut bytes)?;
                    if bytes.len() > 4096 {
                        return Err("NATS ports file exceeds 4096 bytes".into());
                    }
                    match serde_json::from_slice::<Ports>(&bytes) {
                        Ok(ports) => {
                            let address: SocketAddr = ports.nats[0]
                                .strip_prefix("nats://")
                                .ok_or("NATS reported an invalid client URL")?
                                .parse()?;
                            if address.ip() != std::net::Ipv4Addr::LOCALHOST || address.port() == 0
                            {
                                return Err("NATS must listen on a private loopback port".into());
                            }
                            let client = tokio::time::timeout(
                                Duration::from_secs(2),
                                async_nats::ConnectOptions::new()
                                    .client_capacity(64)
                                    .subscription_capacity(64)
                                    .connect(&ports.nats[0]),
                            )
                            .await??;
                            eprintln!("NATS: {} pid={process_id}", ports.nats[0]);
                            return Ok(client);
                        }
                        Err(error) if error.is_eof() => {}
                        Err(error) => return Err(error.into()),
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err("NATS did not become ready within five seconds".into())
    }

    pub async fn wait(&mut self) -> Result<ExitStatus, ServiceError> {
        Ok(self.child.wait().await?)
    }

    pub async fn stop(&mut self) -> Result<(), ServiceError> {
        if self.child.try_wait()?.is_none() {
            tokio::time::timeout(Duration::from_secs(5), self.child.kill()).await??;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Ports {
    nats: [String; 1],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn owned_server_connects_and_is_reaped() {
        let (mut server, client) = Server::start(PROGRAM).await.unwrap();
        client.flush().await.unwrap();
        assert_eq!(
            client.connection_state(),
            async_nats::connection::State::Connected
        );
        assert_eq!(client.server_info().max_payload, 65536);
        let directory = server.directory.path().to_owned();
        server.stop().await.unwrap();
        assert!(server.child.try_wait().unwrap().is_some());
        drop(server);
        assert!(!directory.exists());
    }

    #[tokio::test]
    async fn missing_executable_is_reported() {
        let result = Server::start("/nonexistent/holyfurries-nats-server").await;
        assert!(result.is_err());
    }
}
