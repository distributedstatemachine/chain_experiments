use super::*;

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
    assert!(bad.starts_with("HTTP/1.1 400 Bad Request"));

    let too_large = send_raw(addr, b"POST /tx HTTP/1.1\r\ncontent-length: 3\r\n\r\nabc");
    assert!(too_large.starts_with("HTTP/1.1 413 Payload Too Large"));

    server_thread.join().unwrap();
}

#[test]
fn rpc_http_parser_rejects_bad_headers_and_waits_for_complete_bodies() {
    let uppercase_hash = hex(&hash_bytes(b"test", &[b"rpc-uppercase"])).to_uppercase();
    assert!(parse_hash(&uppercase_hash).is_ok());
    assert!(parse_hash(&"g".repeat(64)).is_err());

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
    assert!(overview.contains("\"type\":\"overview\""));
    assert!(overview.contains("\"block_count\":1"));
    let account = rpc.explorer_websocket_response(&format!(
        "{{\"type\":\"account\",\"address\":\"{}\"}}",
        hex(&miner)
    ));
    assert!(account.contains("\"type\":\"account\""));
    assert!(account.contains("\"is_miner\":true"));
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
    assert!(text.starts_with("HTTP/1.1 202 Accepted"));
    assert!(text.ends_with("{\"accepted\":true}"));
    let conflict = http_response_text(&RpcResponse {
        status: 409,
        body: "{\"error\":\"duplicate transaction\"}".to_owned(),
    });
    assert!(conflict.starts_with("HTTP/1.1 409 Conflict"));
    let limited = http_response_text(&RpcResponse {
        status: 429,
        body: "{\"error\":\"rate limit exceeded\"}".to_owned(),
    });
    assert!(limited.starts_with("HTTP/1.1 429 Too Many Requests"));
    let html = http_response_text(&RpcResponse {
        status: 200,
        body: "<!doctype html><html></html>".to_owned(),
    });
    assert!(html.contains("content-type: text/html; charset=utf-8"));
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

    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("\"height\":0"));
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
    let mut full_response = handshake;
    full_response.extend_from_slice(&response);
    let response = String::from_utf8_lossy(&full_response);

    assert!(response.contains("101 Switching Protocols"));
    assert!(response.contains("sec-websocket-accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
    assert!(response.contains("\"type\":\"overview\""));
    assert!(response.contains("\"block_count\":1"));
}

#[test]
fn rpc_http_server_rejects_bad_websocket_routes_and_auth() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let bad_route = serve_one_http_request(
            RpcGateway::new(RpcNode::new(Chain::new(beacon)), RpcPolicy::default()),
            b"GET /wrong/ws HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
        );
    assert!(bad_route.starts_with("HTTP/1.1 404 Not Found"));

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
    assert!(unauthorized.starts_with("HTTP/1.1 401 Unauthorized"));
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
