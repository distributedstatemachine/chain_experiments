use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use tensor_vm_explorer::{
    DEFAULT_EXPLORER_LISTEN, DEFAULT_EXPLORER_WS_URL, explorer_health_json, explorer_shell_html,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args.first().is_some_and(|arg| arg == "healthcheck") {
        run_healthcheck(&args)
    } else {
        serve()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn serve() -> std::io::Result<()> {
    let listen = std::env::var("TENSORVM_EXPLORER_LISTEN")
        .unwrap_or_else(|_| DEFAULT_EXPLORER_LISTEN.to_owned());
    let ws_url = std::env::var("TENSORVM_EXPLORER_WS_URL")
        .unwrap_or_else(|_| DEFAULT_EXPLORER_WS_URL.to_owned());
    let listener = TcpListener::bind(&listen)?;
    for stream in listener.incoming() {
        handle_client(stream?, &ws_url)?;
    }
    Ok(())
}

fn run_healthcheck(args: &[String]) -> std::io::Result<()> {
    let addr = args
        .windows(2)
        .find_map(|window| (window[0] == "--addr").then(|| window[1].as_str()))
        .unwrap_or(DEFAULT_EXPLORER_LISTEN);
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(b"GET /health HTTP/1.1\r\nhost: tensorvm-explorer\r\n\r\n")?;
    stream.flush()?;
    let mut text = String::new();
    stream.read_to_string(&mut text)?;
    if text.contains("200 OK") && text.contains("tensorvm_explorer_ready") {
        Ok(())
    } else {
        Err(std::io::Error::other("explorer healthcheck failed"))
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
