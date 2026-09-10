//! Listener lifecycle and connection ownership.
use crate::session::{self, Shared};
use minicraft_common::protocol::{
    Message, decode_line, encode_line, read_bounded_line,
};
use std::{
    io::{BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::Duration,
};

pub fn default_server_address() -> String {
    format!("0.0.0.0:{}", minicraft_common::protocol::DEFAULT_PORT)
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

pub fn serve(listener: TcpListener, shutdown: Arc<AtomicBool>) -> Result<(), String> {
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
        let Ok(message) = decode_line(&line) else {
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
            match session::accept_login(&shared, id, &requested, &game_version, protocol_version) {
                Ok(accepted) => {
                    username = Some(accepted.username.clone());
                    let _ = sender.send(session::init_message(id));
                    let _ = sender.send(Message::Usernames {
                        users: accepted.existing_users,
                    });
                    for state in accepted.replay_states {
                        let _ = sender.send(Message::Player { state });
                    }
                    broadcast(
                        &shared,
                        Some(id),
                        Message::Add {
                            client_id: id,
                            username: accepted.username,
                        },
                    );
                }
                Err(text) => {
                    let _ = sender.send(Message::Notify { text });
                    break;
                }
            }
            continue;
        }
        match message {
            Message::Move {
                x,
                y,
                level,
                health,
            } => {
                let player = session::apply_move(
                    &shared,
                    id,
                    username.as_deref().unwrap_or_default(),
                    x,
                    y,
                    level,
                    health,
                );
                broadcast(&shared, Some(id), player);
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
    if session::end_session(&shared, id) {
        broadcast(&shared, None, Message::Remove { client_id: id });
    }
    drop(sender);
    let _ = writer.join();
}

fn writer_loop(mut stream: TcpStream, receiver: Receiver<Message>) {
    for message in receiver {
        let Ok(frame) = encode_line(&message) else {
            break;
        };
        if stream.write_all(&frame).is_err() {
            break;
        }
    }
}

fn broadcast(shared: &Mutex<Shared>, except: Option<u64>, message: Message) {
    let senders = {
        let state = shared.lock().unwrap();
        state
            .senders
            .iter()
            .filter(|(id, _)| Some(**id) != except)
            .map(|(_, sender)| sender.clone())
            .collect::<Vec<_>>()
    };
    for sender in senders {
        let _ = sender.send(message.clone());
    }
}
