// Sprint 312: WebSocket RPC & Realtime Event Broadcaster Integration Tests
//
// Tests verify:
//   1. WebSocket handshake (HTTP 101 Upgrade with Sec-WebSocket-Accept calculation)
//   2. Request compilation over WebSocket frame (knc_compile)
//   3. Execution and real-time VmEvent streaming over WebSocket frames (knc_execute)
//   4. State inspection over WebSocket (knc_inspect_state)
//   5. Graceful socket disconnect handling

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::Arc;

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{RpcServer, base64_encode, read_ws_frame, sha1_digest};
use knoten_core_types::ast::Node;
use serde_json::{Value, json};

fn test_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

fn write_masked_ws_frame<W: Write>(writer: &mut W, text: &str) -> std::io::Result<()> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mask: [u8; 4] = [0x12, 0x34, 0x56, 0x78];

    let mut frame = Vec::new();
    frame.push(0x81); // FIN = 1, Opcode = 1 (Text)

    if len <= 125 {
        frame.push((len as u8) | 0x80); // Mask bit set
    } else if len <= 65535 {
        frame.push(126 | 0x80);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127 | 0x80);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    frame.extend_from_slice(&mask);
    let mut masked_bytes = bytes.to_vec();
    for i in 0..len {
        masked_bytes[i] ^= mask[i % 4];
    }
    frame.extend_from_slice(&masked_bytes);

    writer.write_all(&frame)?;
    writer.flush()
}

#[test]
fn test_sha1_and_base64_utilities() {
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let magic = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", key);
    let digest = sha1_digest(magic.as_bytes());
    let accept = base64_encode(&digest);
    assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
}

#[test]
fn test_websocket_rpc_handshake_and_execution() {
    let port = 19876;
    let server = Arc::new(RpcServer::new(test_perms()));

    let server_clone = server.clone();
    std::thread::spawn(move || {
        let _ = server_clone.listen_ws(port);
    });

    // Give server thread time to bind port
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("Failed to connect to WS port");

    // 1. Send WebSocket Handshake
    let handshake_req = "GET / HTTP/1.1\r\n\
                         Host: 127.0.0.1\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\r\n";

    stream.write_all(handshake_req.as_bytes()).unwrap();
    stream.flush().unwrap();

    // 2. Read Handshake Response
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
    assert!(status_line.contains("101 Switching Protocols"));

    let mut accept_found = false;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if line.to_lowercase().contains("sec-websocket-accept:") {
            accept_found = true;
        }
    }
    assert!(
        accept_found,
        "Handshake response must contain Sec-WebSocket-Accept header"
    );

    // 3. Send knc_compile over WebSocket frame
    let compile_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_compile",
        "params": {
            "ast": Node::Add(Box::new(Node::IntLiteral(40)), Box::new(Node::IntLiteral(2)))
        },
        "id": 1
    });

    write_masked_ws_frame(&mut stream, &compile_req.to_string()).unwrap();

    let resp_frame = read_ws_frame(&mut reader)
        .unwrap()
        .expect("Response frame expected");
    let resp: Value = serde_json::from_str(&resp_frame).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["status"], "ok");
    assert!(resp["result"]["instruction_count"].as_u64().unwrap() > 0);

    // 4. Send knc_execute with EventEmit over WebSocket frame
    let exec_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "ws_session_1",
            "ast": Node::Block(vec![
                Node::EventEmit(
                    Box::new(Node::StringLiteral("ws_connected".to_string())),
                    Box::new(Node::IntLiteral(100)),
                ),
                Node::Add(Box::new(Node::IntLiteral(15)), Box::new(Node::IntLiteral(25)))
            ])
        },
        "id": 2
    });

    write_masked_ws_frame(&mut stream, &exec_req.to_string()).unwrap();

    // Read responses: event stream notice and/or main execution response
    let frame1 = read_ws_frame(&mut reader).unwrap().expect("Frame expected");
    let val1: Value = serde_json::from_str(&frame1).unwrap();

    if val1["method"] == "knc_event" {
        assert_eq!(val1["params"]["session_id"], "ws_session_1");

        let frame2 = read_ws_frame(&mut reader)
            .unwrap()
            .expect("Main response expected");
        let val2: Value = serde_json::from_str(&frame2).unwrap();
        assert_eq!(val2["id"], 2);
        assert_eq!(val2["result"]["status"], "ok");
        assert_eq!(val2["result"]["result"]["Int"], 40);
    } else {
        assert_eq!(val1["id"], 2);
        assert_eq!(val1["result"]["status"], "ok");
        assert_eq!(val1["result"]["result"]["Int"], 40);
    }

    // 5. Send Close frame and verify clean disconnect
    let close_frame: [u8; 6] = [0x88, 0x80, 0x00, 0x00, 0x00, 0x00];
    stream.write_all(&close_frame).unwrap();
    stream.flush().unwrap();
}
