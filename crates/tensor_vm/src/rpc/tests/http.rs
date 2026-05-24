use super::*;

fn http_status_line(response: &str) -> &str {
    response
        .lines()
        .next()
        .expect("HTTP response must include status line")
}

fn http_body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP response must include header/body separator")
}

fn http_header<'a>(response: &'a str, header: &str) -> &'a str {
    response
        .lines()
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(header).then_some(value.trim())
        })
        .unwrap_or_else(|| panic!("HTTP response must include {header} header"))
}

fn websocket_text_payload(frame: &[u8]) -> String {
    assert_eq!(frame.first().copied(), Some(0x81));
    let length_byte = frame
        .get(1)
        .copied()
        .expect("websocket frame must include length byte");
    assert_eq!(length_byte & 0x80, 0, "server websocket frame is unmasked");
    let length_indicator = length_byte & 0x7f;
    let (length, payload_start) = match length_indicator {
        0..=125 => (usize::from(length_indicator), 2),
        126 => {
            let bytes: [u8; 2] = frame[2..4]
                .try_into()
                .expect("16-bit websocket length must be present");
            (usize::from(u16::from_be_bytes(bytes)), 4)
        }
        127 => {
            let bytes: [u8; 8] = frame[2..10]
                .try_into()
                .expect("64-bit websocket length must be present");
            (
                usize::try_from(u64::from_be_bytes(bytes))
                    .expect("websocket frame length must fit usize"),
                10,
            )
        }
        _ => unreachable!("7-bit length indicator is always bounded"),
    };
    let payload_end = payload_start + length;
    assert!(frame.len() >= payload_end);
    if frame.len() > payload_end {
        assert_eq!(&frame[payload_end..], &[0x88, 0]);
    }
    String::from_utf8(frame[payload_start..payload_end].to_vec())
        .expect("websocket payload must be UTF-8 text")
}

#[test]
fn rpc_http_server_returns_bad_request_and_payload_too_large() {
    use std::io::ErrorKind;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    fn send_raw(addr: SocketAddr, raw: &[u8]) -> String {
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(raw).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        response
    }

    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let gateway = RpcGateway::new(
        RpcNode::new(Chain::new(beacon)),
        RpcPolicy {
            max_body_bytes: 2,
            ..RpcPolicy::default()
        },
    );
    let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
        Ok(server) => server,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
        Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
    };
    let addr = server.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || server.serve_n(2).unwrap());

    let bad = send_raw(addr, b"GET");
    assert_eq!(http_status_line(&bad), "HTTP/1.1 400 Bad Request");

    let too_large = send_raw(addr, b"POST /tx HTTP/1.1\r\ncontent-length: 3\r\n\r\nabc");
    assert_eq!(
        http_status_line(&too_large),
        "HTTP/1.1 413 Payload Too Large"
    );

    server_thread.join().unwrap();
}

#[test]
fn rpc_http_parser_rejects_bad_headers_and_waits_for_complete_bodies() {
    let uppercase_hash = hex(&hash_bytes(b"test", &[b"rpc-uppercase"])).to_uppercase();
    assert!(parse_hash(&uppercase_hash).is_ok());
    assert!(parse_hash(&"g".repeat(64)).is_err());
    assert!(parse_hash(&format!("0x{uppercase_hash}")).is_err());

    assert!(matches!(
        try_parse_http_request(b"GET /chain/head HTTP/1.1\r\n\r\n", 16),
        Some(ParsedHttpRequest::Request {
            auth_token: None,
            ..
        })
    ));
    assert!(matches!(
        try_parse_http_request(
            b"POST /tx HTTP/1.1\r\nauthorization: Bearer secret\r\ncontent-length: 4\r\n\r\nbody",
            16,
        ),
        Some(ParsedHttpRequest::Request {
            auth_token: Some(token),
            ..
        }) if token == "secret"
    ));
    assert!(matches!(
        try_parse_http_request(
            b"POST /tx HTTP/1.1\r\nx-tensorchain-auth: local\r\ncontent-length: 4\r\n\r\nbody",
            16,
        ),
        Some(ParsedHttpRequest::Request {
            auth_token: Some(token),
            ..
        }) if token == "local"
    ));
    assert!(matches!(
        try_parse_http_request(
            b"GET /explorer/ws?token=local HTTP/1.1\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
            16,
        ),
        Some(ParsedHttpRequest::WebSocketUpgrade {
            path,
            auth_token: Some(token),
            websocket_key,
        }) if path == "/explorer/ws" && token == "local" && websocket_key == "dGhlIHNhbXBsZSBub25jZQ=="
    ));
    assert_eq!(
        websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"ws-miner");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(miner, 10_000).unwrap();
    chain.produce_block(miner, 1).unwrap();
    let rpc = RpcNode::new(chain);
    let overview = rpc.explorer_websocket_response(
        "{\"type\":\"overview\",\"block_limit\":1,\"receipt_limit\":1,\"job_limit\":1}",
    );
    let overview = json_text(&overview);
    assert_eq!(overview["type"].as_str(), Some("overview"));
    assert_eq!(overview["summary"]["block_count"].as_u64(), Some(1));
    assert_eq!(
        overview["blocks"]
            .as_array()
            .expect("overview must include blocks")
            .len(),
        1
    );
    let account = rpc.explorer_websocket_response(&format!(
        "{{\"type\":\"account\",\"address\":\"{}\"}}",
        hex(&miner)
    ));
    let account = json_text(&account);
    assert_eq!(account["type"].as_str(), Some("account"));
    assert_eq!(
        account["account"]["address"].as_str(),
        Some(hex(&miner).as_str())
    );
    assert_eq!(account["account"]["is_miner"].as_bool(), Some(true));
    assert_eq!(account["account"]["is_validator"].as_bool(), Some(true));
    assert!(matches!(
        try_parse_http_request(b"GET /\xff HTTP/1.1\r\n\r\n", 16),
        Some(ParsedHttpRequest::BadRequest)
    ));
    assert!(matches!(
        try_parse_http_request(b"GET\r\n\r\n", 16),
        Some(ParsedHttpRequest::BadRequest)
    ));
    assert!(matches!(
        try_parse_http_request(b"\r\n\r\n", 16),
        Some(ParsedHttpRequest::BadRequest)
    ));
    assert!(matches!(
        try_parse_http_request(b"GET /chain/head HTTP/1.1\r\nhost\r\n\r\n", 16),
        Some(ParsedHttpRequest::Request { .. })
    ));
    assert!(matches!(
        try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: nope\r\n\r\n", 16),
        Some(ParsedHttpRequest::BadRequest)
    ));
    assert!(matches!(
        try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: 17\r\n\r\n", 16),
        Some(ParsedHttpRequest::TooLarge)
    ));
    assert!(
        try_parse_http_request(b"POST /tx HTTP/1.1\r\ncontent-length: 4\r\n\r\nbo", 16,).is_none()
    );
    assert!(try_parse_http_request(b"GET /chain/head HTTP/1.1\r\n", 16).is_none());
    assert!(matches!(
        try_parse_http_request(
            b"GET /explorer/ws HTTP/1.1\r\nupgrade: websocket\r\n\r\n",
            16,
        ),
        Some(ParsedHttpRequest::BadRequest)
    ));
}

#[test]
fn rpc_http_reader_handles_in_memory_requests_and_limits() {
    let mut get = std::io::Cursor::new(b"GET /chain/head HTTP/1.1\r\n\r\n");
    assert!(matches!(
        read_http_request_from(&mut get, 16).unwrap(),
        ParsedHttpRequest::Request {
            request,
            auth_token: None,
        } if request.method == "GET" && request.path == "/chain/head" && request.body.is_empty()
    ));

    let mut post = std::io::Cursor::new(b"POST /tx HTTP/1.1\r\ncontent-length: 4\r\n\r\nbody");
    assert!(matches!(
        read_http_request_from(&mut post, 16).unwrap(),
        ParsedHttpRequest::Request { request, .. } if request.body == b"body"
    ));

    let mut empty = std::io::Cursor::new(Vec::<u8>::new());
    assert!(matches!(
        read_http_request_from(&mut empty, 16).unwrap(),
        ParsedHttpRequest::BadRequest
    ));

    let too_large = vec![b'x'; 8 * 1024 + 17];
    let mut too_large = std::io::Cursor::new(too_large);
    assert!(matches!(
        read_http_request_from(&mut too_large, 16).unwrap(),
        ParsedHttpRequest::TooLarge
    ));
}

#[test]
fn rpc_gateway_enforces_auth_body_limits_and_rate_limits() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut gateway = RpcGateway::new(
        RpcNode::new(Chain::new(beacon)),
        RpcPolicy {
            auth_token: Some("secret".to_owned()),
            max_body_bytes: 8,
            max_requests_per_client: 1,
        },
    );
    let request = RpcRequest {
        method: "GET".to_owned(),
        path: "/chain/head".to_owned(),
        body: Vec::new(),
    };

    assert_eq!(gateway.handle("client", None, &request).status, 401);
    assert_eq!(gateway.request_count("client"), 0);

    let oversized = RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: b"too many bytes".to_vec(),
    };
    assert_eq!(
        gateway.handle("client", Some("secret"), &oversized).status,
        413
    );
    assert_eq!(gateway.request_count("client"), 0);

    assert_eq!(
        gateway.handle("client", Some("secret"), &request).status,
        200
    );
    assert_eq!(
        gateway.handle("client", Some("secret"), &request).status,
        429
    );
}

#[test]
fn rpc_formats_http_response() {
    let response = RpcResponse {
        status: 202,
        body: "{\"accepted\":true}".to_owned(),
    };
    let text = http_response_text(&response);
    assert_eq!(http_status_line(&text), "HTTP/1.1 202 Accepted");
    assert_eq!(http_body(&text), "{\"accepted\":true}");
    let conflict = http_response_text(&RpcResponse {
        status: 409,
        body: "{\"error\":\"duplicate transaction\"}".to_owned(),
    });
    assert_eq!(http_status_line(&conflict), "HTTP/1.1 409 Conflict");
    let limited = http_response_text(&RpcResponse {
        status: 429,
        body: "{\"error\":\"rate limit exceeded\"}".to_owned(),
    });
    assert_eq!(http_status_line(&limited), "HTTP/1.1 429 Too Many Requests");
    let html = http_response_text(&RpcResponse {
        status: 200,
        body: "<!doctype html><html></html>".to_owned(),
    });
    assert_eq!(
        http_header(&html, "content-type"),
        "text/html; charset=utf-8"
    );
}

#[test]
fn rpc_http_server_serves_socket_request() {
    use std::io::ErrorKind;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let gateway = RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default());
    let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
        Ok(server) => server,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
        Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
    };
    assert_eq!(server.gateway().request_count("unseen-client"), 0);
    server.set_nonblocking(true).unwrap();
    server.set_nonblocking(false).unwrap();
    server.gateway_mut().policy.max_body_bytes = 32;
    assert_eq!(server.gateway().policy.max_body_bytes, 32);
    let addr = server.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || server.serve_next().unwrap());

    let mut client = TcpStream::connect(addr).unwrap();
    client
        .write_all(b"GET /chain/head HTTP/1.1\r\nhost: localhost\r\n\r\n")
        .unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    server_thread.join().unwrap();

    assert_eq!(http_status_line(&response), "HTTP/1.1 200 OK");
    let body = json_text(http_body(&response));
    assert_eq!(body["height"].as_u64(), Some(0));
    assert_eq!(body["block_count"].as_u64(), Some(0));
    json_hex_field(&body, "state_root");
}

#[test]
fn rpc_http_server_serves_explorer_websocket_poll() {
    use std::io::ErrorKind;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"ws-http-miner");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(miner, 10_000).unwrap();
    chain.produce_block(miner, 1).unwrap();
    let gateway = RpcGateway::new(RpcNode::new(chain), RpcPolicy::default());
    let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
        Ok(server) => server,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
        Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
    };
    let addr = server.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || server.serve_next().unwrap());

    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(b"GET /explorer/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nconnection: Upgrade\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
    let mut handshake = Vec::new();
    let mut byte = [0_u8; 1];
    while !handshake.ends_with(b"\r\n\r\n") {
        client.read_exact(&mut byte).unwrap();
        handshake.push(byte[0]);
    }
    client
        .write_all(&masked_websocket_text_frame(
            "{\"type\":\"overview\",\"block_limit\":1}",
        ))
        .unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    server_thread.join().unwrap();
    let handshake = std::str::from_utf8(&handshake).expect("websocket handshake must be UTF-8");
    assert_eq!(
        http_status_line(handshake),
        "HTTP/1.1 101 Switching Protocols"
    );
    assert_eq!(
        http_header(handshake, "sec-websocket-accept"),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
    let response = json_text(&websocket_text_payload(&response));
    assert_eq!(response["type"].as_str(), Some("overview"));
    assert_eq!(response["summary"]["block_count"].as_u64(), Some(1));
}

#[test]
fn rpc_http_server_rejects_bad_websocket_routes_and_auth() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let bad_route = serve_one_http_request(
            RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default()),
            b"GET /wrong/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
    assert_eq!(http_status_line(&bad_route), "HTTP/1.1 404 Not Found");

    let unauthorized = serve_one_http_request(
            RpcGateway::new(
                RpcNode::new(Chain::new(beacon)),
                RpcPolicy {
                    auth_token: Some("secret".to_owned()),
                    max_body_bytes: 1024,
                    max_requests_per_client: 10,
                },
            ),
            b"GET /explorer/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
    assert_eq!(http_status_line(&unauthorized), "HTTP/1.1 401 Unauthorized");
}

fn serve_one_http_request(gateway: RpcGateway, request: &[u8]) -> String {
    use std::io::ErrorKind;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let mut server = match RpcHttpServer::bind("127.0.0.1:0", gateway) {
        Ok(server) => server,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return String::new();
        }
        Err(error) => panic!("failed to bind RPC HTTP server: {error}"),
    };
    let addr = server.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || server.serve_next().unwrap());
    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(request).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    server_thread.join().unwrap();
    response
}

fn masked_websocket_text_frame(text: &str) -> Vec<u8> {
    let mask = [1_u8, 2, 3, 4];
    let bytes = text.as_bytes();
    assert!(bytes.len() < 126);
    let mut frame = vec![0x81, 0x80 | bytes.len() as u8];
    frame.extend_from_slice(&mask);
    for (index, byte) in bytes.iter().enumerate() {
        frame.push(byte ^ mask[index % 4]);
    }
    frame
}
