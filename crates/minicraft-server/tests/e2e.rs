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
