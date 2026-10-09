//! End-to-end protocol differential coverage: client transport against the
//! standalone server over loopback TCP.
use minicraft_client::network::MultiplayerClient;
use minicraft_common::protocol::{Message, PlayerState};
use minicraft_server::serve;
use std::{
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

fn wait_for(client: &mut MultiplayerClient, predicate: impl Fn(&Message) -> bool) -> Message {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        for message in client.poll() {
            if predicate(&message) {
                return message;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "protocol wait timed out"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn two_clients_exchange_presence_state_and_soak_pings() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = Arc::new(AtomicBool::new(false));
    let server_shutdown = Arc::clone(&shutdown);
    let server = thread::spawn(move || serve(listener, server_shutdown).unwrap());

    let mut alice = MultiplayerClient::connect(&address.to_string(), "Alice").unwrap();
    let mut bob = MultiplayerClient::connect(&address.to_string(), "Bob").unwrap();
    assert_ne!(alice.client_id, bob.client_id);
    wait_for(
        &mut alice,
        |message| matches!(message, Message::Add { username, .. } if username == "Bob"),
    );

    alice.send_player(80, 96, 1, 9).unwrap();
    let state = wait_for(&mut bob, |message| {
        matches!(message, Message::Player { .. })
    });
    assert!(matches!(
        state,
        Message::Player {
            state: PlayerState {
                x: 80,
                y: 96,
                health: 9,
                ..
            }
        }
    ));

    for nonce in 0..512 {
        bob.ping(nonce).unwrap();
        let pong = wait_for(
            &mut bob,
            |message| matches!(message, Message::Pong { nonce: seen } if *seen == nonce),
        );
        assert_eq!(pong, Message::Pong { nonce });
    }

    drop(alice);
    wait_for(&mut bob, |message| {
        matches!(message, Message::Remove { .. })
    });
    drop(bob);
    shutdown.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

#[test]
fn login_rejects_version_mismatch_and_empty_username() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = Arc::new(AtomicBool::new(false));
    let server_shutdown = Arc::clone(&shutdown);
    let server = thread::spawn(move || serve(listener, server_shutdown).unwrap());

    let error = match MultiplayerClient::connect(&address.to_string(), "..") {
        Err(error) => error,
        Ok(_) => panic!("empty username must be rejected"),
    };
    assert!(error.contains("username must contain"), "{error}");

    // Force an incompatible login by speaking raw protocol.
    use minicraft_common::protocol::{Message, decode_line, encode_line, read_bounded_line};
    use std::io::{BufReader, Write};
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    let frame = encode_line(&Message::Login {
        username: "Alice".to_owned(),
        game_version: "0.0.0".to_owned(),
        protocol_version: 1,
    })
    .unwrap();
    stream.write_all(&frame).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let line = read_bounded_line(&mut reader)
        .unwrap()
        .expect("notify line");
    let message = decode_line(&line).unwrap();
    assert!(
        matches!(&message, Message::Notify { text } if text.contains("incompatible client")),
        "{message:?}"
    );

    shutdown.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

struct TestServer {
    address: std::net::SocketAddr,
    shutdown: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn start(limits: minicraft_server::server::ServerLimits) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&shutdown);
        let worker = thread::spawn(move || {
            minicraft_server::server::serve_with_limits(listener, flag, limits).unwrap()
        });
        Self {
            address,
            shutdown,
            worker: Some(worker),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn idle_logged_in_peer_is_removed_while_active_peer_keeps_heartbeats() {
    let server = TestServer::start(minicraft_server::server::ServerLimits {
        idle_timeout: Duration::from_millis(500),
        ..Default::default()
    });
    let idle = MultiplayerClient::connect(&server.address.to_string(), "Idle").unwrap();
    let mut active = MultiplayerClient::connect(&server.address.to_string(), "Active").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        active.ping(1).unwrap();
        if active.poll().iter().any(|message| {
            *message
                == Message::Remove {
                    client_id: idle.client_id,
                }
        }) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "idle peer was not removed"
        );
        thread::sleep(Duration::from_millis(20));
    }
    active.ping(2).unwrap();
    wait_for(&mut active, |message| {
        *message == Message::Pong { nonce: 2 }
    });
}

#[test]
fn invalid_move_disconnects_only_offender_and_name_can_be_reused() {
    let server = TestServer::start(Default::default());
    let mut bob = MultiplayerClient::connect(&server.address.to_string(), "Bob").unwrap();
    let mut alice = MultiplayerClient::connect(&server.address.to_string(), "Alice").unwrap();
    assert!(MultiplayerClient::connect(&server.address.to_string(), "Alice!").is_err());
    alice.send_player(i32::MAX, 0, usize::MAX, 255).unwrap();
    let removed = wait_for(
        &mut bob,
        |message| matches!(message, Message::Remove { client_id } if *client_id == alice.client_id),
    );
    assert_eq!(
        removed,
        Message::Remove {
            client_id: alice.client_id
        }
    );
    bob.ping(42).unwrap();
    wait_for(&mut bob, |message| *message == Message::Pong { nonce: 42 });
    let replacement = MultiplayerClient::connect(&server.address.to_string(), "Alice").unwrap();
    assert_ne!(replacement.client_id, alice.client_id);
}

#[test]
fn partial_login_times_out_connection_capacity_recovers_and_shutdown_closes_live_client() {
    use std::io::{Read, Write};
    let server = TestServer::start(minicraft_server::server::ServerLimits {
        max_connections: 1,
        login_timeout: Duration::from_millis(250),
        ..Default::default()
    });
    let mut partial = std::net::TcpStream::connect(server.address).unwrap();
    partial
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    partial.write_all(b"{").unwrap();
    thread::sleep(Duration::from_millis(50));
    let mut excess = std::net::TcpStream::connect(server.address).unwrap();
    excess
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut byte = [0];
    assert!(matches!(excess.read(&mut byte), Ok(0) | Err(_)));
    assert!(matches!(partial.read(&mut byte), Ok(0) | Err(_)));
    thread::sleep(Duration::from_millis(30));
    let mut live = MultiplayerClient::connect(&server.address.to_string(), "Live").unwrap();
    drop(server); // Must return while `live` still owns its socket.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if live.ping(1).is_err() {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn flooding_peer_is_removed_without_blocking_second_client() {
    let server = TestServer::start(minicraft_server::server::ServerLimits {
        messages_per_second: 32,
        ..Default::default()
    });
    let mut healthy = MultiplayerClient::connect(&server.address.to_string(), "Healthy").unwrap();
    let mut flood = MultiplayerClient::connect(&server.address.to_string(), "Flood").unwrap();
    for nonce in 0..100 {
        if flood.ping(nonce).is_err() {
            break;
        }
    }
    wait_for(
        &mut healthy,
        |message| matches!(message, Message::Remove { client_id } if *client_id == flood.client_id),
    );
    healthy.ping(99).unwrap();
    wait_for(&mut healthy, |message| {
        *message == Message::Pong { nonce: 99 }
    });
}

#[test]
fn fragmented_login_survives_socket_timeouts_and_server_only_input_is_rejected() {
    use minicraft_common::protocol::{
        GAME_VERSION, PROTOCOL_VERSION, decode_line, encode_line, read_bounded_line,
    };
    use std::io::{BufReader, Write};
    let server = TestServer::start(Default::default());
    let mut healthy = MultiplayerClient::connect(&server.address.to_string(), "Healthy").unwrap();
    let mut raw = std::net::TcpStream::connect(server.address).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let frame = encode_line(&Message::Login {
        username: "Raw".to_owned(),
        game_version: GAME_VERSION.to_owned(),
        protocol_version: PROTOCOL_VERSION,
    })
    .unwrap();
    raw.write_all(&frame[..10]).unwrap();
    thread::sleep(Duration::from_millis(150));
    raw.write_all(&frame[10..]).unwrap();
    let mut reader = BufReader::new(raw.try_clone().unwrap());
    let message = decode_line(&read_bounded_line(&mut reader).unwrap().unwrap()).unwrap();
    let Message::Init { client_id, .. } = message else {
        panic!("INIT expected")
    };
    read_bounded_line(&mut reader).unwrap().unwrap(); // USERNAMES
    raw.write_all(
        &encode_line(&Message::Remove {
            client_id: healthy.client_id,
        })
        .unwrap(),
    )
    .unwrap();
    let notify = decode_line(&read_bounded_line(&mut reader).unwrap().unwrap()).unwrap();
    assert!(matches!(notify, Message::Notify { .. }));
    wait_for(&mut healthy, |message| {
        *message == Message::Remove { client_id }
    });
    healthy.ping(3).unwrap();
    wait_for(&mut healthy, |message| {
        *message == Message::Pong { nonce: 3 }
    });
}
