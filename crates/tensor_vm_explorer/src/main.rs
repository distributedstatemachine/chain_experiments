use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use clap::{Args, Parser, Subcommand};
use tensor_vm_explorer::{
    DEFAULT_EXPLORER_LISTEN, DEFAULT_EXPLORER_WS_URL, explorer_health_json, explorer_shell_html,
};

fn main() {
    let result = Cli::parse().run();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "tensorvm-explorer",
    version,
    about = "Serve the standalone TensorVM browser explorer."
)]
struct Cli {
    #[command(subcommand)]
    command: ExplorerCommand,
}

impl Cli {
    fn run(self) -> std::io::Result<()> {
        match self.command {
            ExplorerCommand::Serve(args) => serve(args),
            ExplorerCommand::HealthCheck(args) => run_health_check(args),
        }
    }
}

#[derive(Debug, Subcommand)]
enum ExplorerCommand {
    Serve(ServeArgs),
    HealthCheck(HealthCheckArgs),
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(
        long,
        env = "TENSORVM_EXPLORER_LISTEN",
        default_value_t = default_explorer_listen()
    )]
    listen: SocketAddr,
    #[arg(
        long,
        env = "TENSORVM_EXPLORER_WS_URL",
        default_value = DEFAULT_EXPLORER_WS_URL
    )]
    ws_url: String,
}

#[derive(Debug, Args)]
struct HealthCheckArgs {
    #[arg(
        long,
        env = "TENSORVM_EXPLORER_HEALTH_CHECK_ADDR",
        default_value_t = default_explorer_health_check_addr()
    )]
    addr: SocketAddr,
}

fn default_explorer_listen() -> SocketAddr {
    DEFAULT_EXPLORER_LISTEN
        .parse()
        .expect("default explorer listen address must be a socket address")
}

fn default_explorer_health_check_addr() -> SocketAddr {
    "127.0.0.1:8080"
        .parse()
        .expect("default explorer health-check address must be a socket address")
}

fn serve(args: ServeArgs) -> std::io::Result<()> {
    let listener = TcpListener::bind(args.listen)?;
    for stream in listener.incoming() {
        handle_client(stream?, &args.ws_url)?;
    }
    Ok(())
}

fn run_health_check(args: HealthCheckArgs) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(args.addr)?;
    stream.write_all(b"GET /health HTTP/1.1\r\nhost: tensorvm-explorer\r\n\r\n")?;
    stream.flush()?;
    let mut text = String::new();
    stream.read_to_string(&mut text)?;
    if text.contains("200 OK") && text.contains("tensorvm_explorer_ready") {
        Ok(())
    } else {
        Err(std::io::Error::other("explorer health-check failed"))
    }
}

fn handle_client(mut stream: TcpStream, ws_url: &str) -> std::io::Result<()> {
    let mut buf = [0_u8; 2048];
    let read = stream.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let (status, content_type, body) = match path {
        "/" | "/explorer" => (
            "200 OK",
            "text/html; charset=utf-8",
            explorer_shell_html(ws_url),
        ),
        "/health" => ("200 OK", "application/json", explorer_health_json(ws_url)),
        _ => (
            "404 Not Found",
            "application/json",
            "{\"error\":\"route not found\"}".to_owned(),
        ),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_explicit_serve_and_health_check_commands() {
        let serve = Cli::try_parse_from([
            "tensorvm-explorer",
            "serve",
            "--listen",
            "127.0.0.1:8081",
            "--ws-url",
            "ws://127.0.0.1:8546/explorer/ws",
        ])
        .unwrap();
        match serve.command {
            ExplorerCommand::Serve(args) => {
                assert_eq!(args.listen, "127.0.0.1:8081".parse().unwrap());
                assert_eq!(args.ws_url, "ws://127.0.0.1:8546/explorer/ws");
            }
            ExplorerCommand::HealthCheck(_) => panic!("expected serve command"),
        }

        let health_check = Cli::try_parse_from([
            "tensorvm-explorer",
            "health-check",
            "--addr",
            "127.0.0.1:8081",
        ])
        .unwrap();
        match health_check.command {
            ExplorerCommand::HealthCheck(args) => {
                assert_eq!(args.addr, "127.0.0.1:8081".parse().unwrap());
            }
            ExplorerCommand::Serve(_) => panic!("expected health-check command"),
        }
    }

    #[test]
    fn cli_does_not_preserve_legacy_healthcheck_command() {
        assert!(Cli::try_parse_from(["tensorvm-explorer", "healthcheck"]).is_err());
    }
}
