use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 4225;
pub const PROTOCOL_VERSION: u32 = 1;
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// The complete 2.2.4 protocol registry retained by `MinicraftProtocol.java`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InputType {
    Invalid,
    Ping,
    Usernames,
    Login,
    Game,
    Init,
    Load,
    Tiles,
    Entities,
    Tile,
    Entity,
    Player,
    Move,
    Add,
    Remove,
    Disconnect,
    Save,
    Notify,
    Interact,
    Push,
    Pickup,
    Chestin,
    Chestout,
    Additems,
    Bed,
    Potion,
    Hurt,
    Die,
    Respawn,
    Drop,
    Stamina,
    Shirt,
    Stopfishing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    pub client_id: u64,
    pub username: String,
    pub x: i32,
    pub y: i32,
    pub level: usize,
    pub health: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Message {
    Login {
        username: String,
        game_version: String,
        protocol_version: u32,
    },
    Init {
        client_id: u64,
        protocol_version: u32,
        server_version: String,
    },
    Usernames {
        users: Vec<(u64, String)>,
    },
    Add {
        client_id: u64,
        username: String,
    },
    Remove {
        client_id: u64,
    },
    Player {
        state: PlayerState,
    },
    Move {
        x: i32,
        y: i32,
        level: usize,
        health: u8,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    Notify {
        text: String,
    },
    Disconnect,
}

#[derive(Clone)]
struct Peer {
    username: String,
    latest: Option<PlayerState>,
}

#[derive(Default)]
struct Shared {
    senders: HashMap<u64, Sender<Message>>,
    peers: HashMap<u64, Peer>,
}

pub fn run_server(address: &str) -> Result<(), String> {
    let listener = TcpListener::bind(address)
        .map_err(|error| format!("cannot bind multiplayer server at {address}: {error}"))?;
    let local = listener
        .local_addr()
        .map_err(|error| format!("cannot read multiplayer server address: {error}"))?;
    println!("Minicraft Rust multiplayer server listening on {local}");
    serve(listener, Arc::new(AtomicBool::new(false)))
}

pub fn default_server_address() -> String {
    format!("0.0.0.0:{DEFAULT_PORT}")
}

pub fn run_client_probe(address: &str, username: &str) -> Result<(), String> {
    let mut client = MultiplayerClient::connect(address, username)?;
    client.send_player(0, 0, 1, 10)?;
    let nonce = 0x224;
    client.ping(nonce)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if client
            .poll()
            .into_iter()
            .any(|message| message == Message::Pong { nonce })
        {
            println!(
                "Connected to {address} as {username} (client {})",
                client.client_id
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(2));
    }
    Err("multiplayer probe timed out waiting for PONG".to_owned())
}

fn serve(listener: TcpListener, shutdown: Arc<AtomicBool>) -> Result<(), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot configure multiplayer listener: {error}"))?;
    let shared = Arc::new(Mutex::new(Shared::default()));
    let next_id = AtomicU64::new(1);
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(error) = stream.set_nonblocking(false) {
                    return Err(format!("cannot configure multiplayer connection: {error}"));
                }
                let id = next_id.fetch_add(1, Ordering::Relaxed);
                let shared = Arc::clone(&shared);
                thread::spawn(move || handle_connection(id, stream, shared));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(format!("multiplayer accept failed: {error}")),
        }
    }
    Ok(())
}

fn handle_connection(id: u64, stream: TcpStream, shared: Arc<Mutex<Shared>>) {
    let Ok(writer_stream) = stream.try_clone() else {
        return;
    };
    let (sender, receiver) = mpsc::channel();
    shared.lock().unwrap().senders.insert(id, sender.clone());
    let writer = thread::spawn(move || writer_loop(writer_stream, receiver));
    let mut reader = BufReader::new(stream);
    let mut username = None;
    loop {
        let line = match read_bounded_line(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                let _ = sender.send(Message::Notify {
                    text: error.to_string(),
                });
                break;
            }
            Err(_) => break,
        };
        let Ok(message) = serde_json::from_str::<Message>(line.trim()) else {
            let _ = sender.send(Message::Notify {
                text: "invalid protocol message".to_owned(),
            });
            continue;
        };
        if username.is_none() {
            let Message::Login {
                username: requested,
                game_version,
                protocol_version,
            } = message
            else {
                let _ = sender.send(Message::Notify {
                    text: "LOGIN must be the first message".to_owned(),
                });
                break;
            };
            if protocol_version != PROTOCOL_VERSION || game_version != "2.2.4-rust" {
                let _ = sender.send(Message::Notify {
                    text: format!("incompatible client {game_version}/protocol {protocol_version}"),
                });
                break;
            }
            let requested = sanitize_username(&requested);
            if requested.is_empty() {
                let _ = sender.send(Message::Notify {
                    text: "username must contain a letter or number".to_owned(),
                });
                break;
            }
            username = Some(requested.clone());
            let (users, latest) = {
                let mut state = shared.lock().unwrap();
                let users = state
                    .peers
                    .iter()
                    .map(|(&peer_id, peer)| (peer_id, peer.username.clone()))
                    .collect();
                let latest = state
                    .peers
                    .values()
                    .filter_map(|peer| peer.latest.clone())
                    .collect::<Vec<_>>();
                state.peers.insert(
                    id,
                    Peer {
                        username: requested.clone(),
                        latest: None,
                    },
                );
                (users, latest)
            };
            let _ = sender.send(Message::Init {
                client_id: id,
                protocol_version: PROTOCOL_VERSION,
                server_version: "2.2.4-rust".to_owned(),
            });
            let _ = sender.send(Message::Usernames { users });
            for state in latest {
                let _ = sender.send(Message::Player { state });
            }
            broadcast(
                &shared,
                Some(id),
                Message::Add {
                    client_id: id,
                    username: requested,
                },
            );
            continue;
        }
        match message {
            Message::Move {
                x,
                y,
                level,
                health,
            } => {
                let state = PlayerState {
                    client_id: id,
                    username: username.clone().unwrap_or_default(),
                    x,
                    y,
                    level,
                    health,
                };
                if let Some(peer) = shared.lock().unwrap().peers.get_mut(&id) {
                    peer.latest = Some(state.clone());
                }
                broadcast(&shared, Some(id), Message::Player { state });
            }
            Message::Ping { nonce } => {
                let _ = sender.send(Message::Pong { nonce });
            }
            Message::Disconnect => break,
            Message::Login { .. }
            | Message::Init { .. }
            | Message::Usernames { .. }
            | Message::Add { .. }
            | Message::Remove { .. }
            | Message::Player { .. }
            | Message::Pong { .. }
            | Message::Notify { .. } => {
                let _ = sender.send(Message::Notify {
                    text: "message is server-only or invalid in this state".to_owned(),
                });
            }
        }
    }
    let was_logged_in = shared.lock().unwrap().peers.remove(&id).is_some();
    shared.lock().unwrap().senders.remove(&id);
    if was_logged_in {
        broadcast(&shared, None, Message::Remove { client_id: id });
    }
    drop(sender);
    let _ = writer.join();
}

fn writer_loop(mut stream: TcpStream, receiver: Receiver<Message>) {
    for message in receiver {
        let Ok(mut encoded) = serde_json::to_vec(&message) else {
            break;
        };
        encoded.push(b'\n');
        if stream.write_all(&encoded).is_err() {
            break;
        }
    }
}

fn broadcast(shared: &Mutex<Shared>, except: Option<u64>, message: Message) {
    let senders = shared
        .lock()
        .unwrap()
        .senders
        .iter()
        .filter(|(id, _)| Some(**id) != except)
        .map(|(_, sender)| sender.clone())
        .collect::<Vec<_>>();
    for sender in senders {
        let _ = sender.send(message.clone());
    }
}

fn sanitize_username(username: &str) -> String {
    username
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .take(24)
        .collect()
}

pub struct MultiplayerClient {
    pub client_id: u64,
    writer: TcpStream,
    receiver: Receiver<Message>,
    backlog: VecDeque<Message>,
}

impl MultiplayerClient {
    pub fn connect(address: &str, username: &str) -> Result<Self, String> {
        let mut writer = TcpStream::connect(address)
            .map_err(|error| format!("cannot connect to multiplayer server {address}: {error}"))?;
        writer
            .set_nodelay(true)
            .map_err(|error| format!("cannot configure multiplayer client: {error}"))?;
        let reader = writer
            .try_clone()
            .map_err(|error| format!("cannot clone multiplayer connection: {error}"))?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || client_reader(reader, sender));
        send_message(
            &mut writer,
            &Message::Login {
                username: username.to_owned(),
                game_version: "2.2.4-rust".to_owned(),
                protocol_version: PROTOCOL_VERSION,
            },
        )?;
        let mut backlog = VecDeque::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let client_id = loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let message = receiver
                .recv_timeout(remaining)
                .map_err(|_| "multiplayer login timed out".to_owned())?;
            match message {
                Message::Init {
                    client_id,
                    protocol_version: PROTOCOL_VERSION,
                    ..
                } => break client_id,
                Message::Notify { text } => return Err(text),
                other => backlog.push_back(other),
            }
        };
        Ok(Self {
            client_id,
            writer,
            receiver,
            backlog,
        })
    }

    pub fn send_player(&mut self, x: i32, y: i32, level: usize, health: u8) -> Result<(), String> {
        send_message(
            &mut self.writer,
            &Message::Move {
                x,
                y,
                level,
                health,
            },
        )
    }

    pub fn ping(&mut self, nonce: u64) -> Result<(), String> {
        send_message(&mut self.writer, &Message::Ping { nonce })
    }

    pub fn poll(&mut self) -> Vec<Message> {
        self.backlog
            .drain(..)
            .chain(self.receiver.try_iter())
            .collect()
    }
}

impl Drop for MultiplayerClient {
    fn drop(&mut self) {
        let _ = send_message(&mut self.writer, &Message::Disconnect);
    }
}

fn client_reader(stream: TcpStream, sender: Sender<Message>) {
    let mut reader = BufReader::new(stream);
    while let Ok(Some(line)) = read_bounded_line(&mut reader) {
        if let Ok(message) = serde_json::from_str(line.trim())
            && sender.send(message).is_err()
        {
            break;
        }
    }
}

fn read_bounded_line(reader: &mut impl BufRead) -> std::io::Result<Option<String>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            break;
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(consumed) > MAX_MESSAGE_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "message exceeds 64 KiB",
            ));
        }
        line.extend_from_slice(&available[..consumed]);
        let complete = available[..consumed].ends_with(b"\n");
        reader.consume(consumed);
        if complete {
            break;
        }
    }
    String::from_utf8(line).map(Some).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "message is not valid UTF-8",
        )
    })
}

fn send_message(stream: &mut TcpStream, message: &Message) -> Result<(), String> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|error| format!("cannot encode multiplayer message: {error}"))?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .map_err(|error| format!("cannot send multiplayer message: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

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
        let address: SocketAddr = listener.local_addr().unwrap();
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
    fn protocol_registry_matches_java_2_2_4_order_and_port() {
        assert_eq!(DEFAULT_PORT, 4225);
        let encoded = serde_json::to_string(&InputType::Stopfishing).unwrap();
        assert_eq!(encoded, r#""STOPFISHING""#);
        assert_eq!(InputType::Invalid as usize, 0);
        assert_eq!(InputType::Stopfishing as usize, 32);
    }

    #[test]
    fn protocol_lines_are_bounded_and_usernames_are_sanitized() {
        let mut acceptable = std::io::Cursor::new(vec![b'a'; MAX_MESSAGE_BYTES - 1]);
        assert_eq!(
            read_bounded_line(&mut acceptable).unwrap().unwrap().len(),
            MAX_MESSAGE_BYTES - 1
        );
        let mut oversized = std::io::Cursor::new(vec![b'a'; MAX_MESSAGE_BYTES + 1]);
        let error = read_bounded_line(&mut oversized).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(sanitize_username("  A!lice_世界-123  "), "Alice_-123");
        assert_eq!(sanitize_username(&"x".repeat(100)).len(), 24);
    }
}
