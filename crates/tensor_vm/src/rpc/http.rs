use super::{RpcGateway, RpcNode, RpcRequest, RpcResponse};
use crate::types::hex_nibble_value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

#[derive(Debug)]
pub struct RpcHttpServer {
    listener: TcpListener,
    gateway: RpcGateway,
    read_timeout: Duration,
}

impl RpcHttpServer {
    pub fn bind(addr: &str, gateway: RpcGateway) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
            gateway,
            read_timeout: Duration::from_secs(5),
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn gateway(&self) -> &RpcGateway {
        &self.gateway
    }

    pub fn gateway_mut(&mut self) -> &mut RpcGateway {
        &mut self.gateway
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        self.listener.set_nonblocking(nonblocking)
    }

    pub fn serve_next(&mut self) -> std::io::Result<()> {
        let (mut stream, peer_addr) = self.listener.accept()?;
        stream.set_read_timeout(Some(self.read_timeout))?;
        match read_http_request(&mut stream, self.gateway.policy.max_body_bytes)? {
            ParsedHttpRequest::Request {
                request,
                auth_token,
            } => {
                let response =
                    self.gateway
                        .handle(&peer_addr.to_string(), auth_token.as_deref(), &request);
                stream.write_all(http_response_text(&response).as_bytes())?;
                stream.flush()
            }
            ParsedHttpRequest::WebSocketUpgrade {
                path,
                auth_token,
                websocket_key,
            } => {
                if path != "/explorer/ws" {
                    stream.write_all(
                        http_response_text(&RpcNode::response(404, "websocket route not found"))
                            .as_bytes(),
                    )?;
                    return stream.flush();
                }
                if let Some(response) = self
                    .gateway
                    .authorize_request(&peer_addr.to_string(), auth_token.as_deref())
                {
                    stream.write_all(http_response_text(&response).as_bytes())?;
                    return stream.flush();
                }
                super::websocket::write_websocket_handshake(&mut stream, &websocket_key)?;
                self.gateway.node.serve_explorer_websocket_once(&mut stream)
            }
            ParsedHttpRequest::BadRequest => {
                let response = RpcNode::response(400, "bad http request");
                stream.write_all(http_response_text(&response).as_bytes())?;
                stream.flush()
            }
            ParsedHttpRequest::TooLarge => {
                let response = RpcNode::response(413, "request body too large");
                stream.write_all(http_response_text(&response).as_bytes())?;
                stream.flush()
            }
        }
    }

    pub fn serve_n(&mut self, max_requests: usize) -> std::io::Result<()> {
        for _ in 0..max_requests {
            self.serve_next()?;
        }
        Ok(())
    }
}

pub fn http_response_text(response: &RpcResponse) -> String {
    let status_text = match response.status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        _ => "Unknown",
    };
    let content_type = if response.body.starts_with("<!doctype html>") {
        "text/html; charset=utf-8"
    } else {
        "application/json"
    };
    format!(
        "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\n\r\n{}",
        response.status,
        status_text,
        content_type,
        response.body.len(),
        response.body
    )
}

pub(super) enum ParsedHttpRequest {
    Request {
        request: RpcRequest,
        auth_token: Option<String>,
    },
    WebSocketUpgrade {
        path: String,
        auth_token: Option<String>,
        websocket_key: String,
    },
    BadRequest,
    TooLarge,
}

pub(super) fn read_http_request(
    stream: &mut TcpStream,
    max_body_bytes: usize,
) -> std::io::Result<ParsedHttpRequest> {
    read_http_request_from(stream, max_body_bytes)
}

pub(super) fn read_http_request_from<R: Read>(
    reader: &mut R,
    max_body_bytes: usize,
) -> std::io::Result<ParsedHttpRequest> {
    let max_request_bytes = max_body_bytes.saturating_add(8 * 1024);
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 1024];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            return Ok(ParsedHttpRequest::BadRequest);
        }
        bytes.extend_from_slice(&buf[..read]);
        if bytes.len() > max_request_bytes {
            return Ok(ParsedHttpRequest::TooLarge);
        }
        if let Some(parsed) = try_parse_http_request(&bytes, max_body_bytes) {
            return Ok(parsed);
        }
    }
}

pub(super) fn try_parse_http_request(
    bytes: &[u8],
    max_body_bytes: usize,
) -> Option<ParsedHttpRequest> {
    let header_end = find_header_end(bytes)?;
    let header_text = match std::str::from_utf8(&bytes[..header_end]) {
        Ok(text) => text,
        Err(_) => return Some(ParsedHttpRequest::BadRequest),
    };
    let mut lines = header_text.split("\r\n");
    let first_line = lines.next().unwrap_or_default();
    let Ok(request_line) = parse_http_request_line(first_line) else {
        return Some(ParsedHttpRequest::BadRequest);
    };
    let method = request_line.method.to_owned();
    let (path, query_auth_token) = split_path_and_auth_token(request_line.path);

    let mut content_length = 0_usize;
    let mut auth_token = query_auth_token;
    let mut websocket_key = None;
    let mut websocket_upgrade = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = match value.parse() {
                Ok(content_length) => content_length,
                Err(_) => return Some(ParsedHttpRequest::BadRequest),
            };
        } else if name.eq_ignore_ascii_case("authorization") {
            auth_token = Some(
                value
                    .strip_prefix("Bearer ")
                    .unwrap_or(value)
                    .trim()
                    .to_owned(),
            );
        } else if name.eq_ignore_ascii_case("x-tensorchain-auth") {
            auth_token = Some(value.to_owned());
        } else if name.eq_ignore_ascii_case("sec-websocket-key") {
            websocket_key = Some(value.to_owned());
        } else if name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket") {
            websocket_upgrade = true;
        }
    }
    if content_length > max_body_bytes {
        return Some(ParsedHttpRequest::TooLarge);
    }

    if websocket_upgrade {
        let Some(websocket_key) = websocket_key else {
            return Some(ParsedHttpRequest::BadRequest);
        };
        return Some(ParsedHttpRequest::WebSocketUpgrade {
            path,
            auth_token,
            websocket_key,
        });
    }

    let body_start = header_end + 4;
    let body_end = body_start.checked_add(content_length)?;
    if bytes.len() < body_end {
        return None;
    }

    Some(ParsedHttpRequest::Request {
        request: RpcRequest {
            method,
            path,
            body: bytes[body_start..body_end].to_vec(),
        },
        auth_token,
    })
}

pub(super) struct HttpRequestLine<'a> {
    pub(super) method: &'a str,
    pub(super) path: &'a str,
}

pub(super) enum HttpRequestLineError {
    MissingMethod,
    MissingPath,
}

pub(super) fn parse_http_request_line(
    line: &str,
) -> Result<HttpRequestLine<'_>, HttpRequestLineError> {
    let mut parts = line.split_whitespace();
    let Some(method) = parts.next() else {
        return Err(HttpRequestLineError::MissingMethod);
    };
    let Some(path) = parts.next() else {
        return Err(HttpRequestLineError::MissingPath);
    };
    Ok(HttpRequestLine { method, path })
}

pub(super) fn split_path_and_auth_token(path: &str) -> (String, Option<String>) {
    let Some((path_only, query)) = path.split_once('?') else {
        return (path.to_owned(), None);
    };
    let token = query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == "token").then(|| percent_decode(value))
    });
    (path_only.to_owned(), token)
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn percent_decode(value: &str) -> String {
    let mut out = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                hex_nibble_value(bytes[index + 1]),
                hex_nibble_value(bytes[index + 2]),
            )
        {
            out.push((high << 4) | low);
            index += 3;
        } else if bytes[index] == b'+' {
            out.push(b' ');
            index += 1;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
