//! Bounded JSON-lines relay lifecycle, without authoritative game simulation.
use crate::session::{self, Shared};
use minicraft_common::protocol::{Message, decode_line, encode_line, read_bounded_line_until};
use std::{
    io::{BufReader, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};
const QUEUE_CAPACITY: usize = 256;
#[derive(Clone, Copy)]
pub struct ServerLimits {
    pub max_connections: usize,
    pub login_timeout: Duration,
    pub idle_timeout: Duration,
    pub messages_per_second: usize,
}
impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            max_connections: 64,
            login_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(30),
            messages_per_second: 2048,
        }
    }
}
pub fn default_server_address() -> String {
    format!("0.0.0.0:{}", minicraft_common::protocol::DEFAULT_PORT)
}
pub fn run_server(address: &str) -> Result<(), String> {
    let listener = TcpListener::bind(address)
        .map_err(|error| format!("cannot bind multiplayer server at {address}: {error}"))?;
    let local = listener.local_addr().map_err(|error| error.to_string())?;
    println!("Minicraft Rust multiplayer server listening on {local}");
    serve(listener, Arc::new(AtomicBool::new(false)))
}
pub fn serve(listener: TcpListener, shutdown: Arc<AtomicBool>) -> Result<(), String> {
    serve_with_limits(listener, shutdown, ServerLimits::default())
}
pub fn serve_with_limits(
    listener: TcpListener,
    shutdown: Arc<AtomicBool>,
    limits: ServerLimits,
) -> Result<(), String> {
    // Ensure USERNAMES/replay frames fit the bounded protocol and queue.
    if limits.max_connections == 0
        || limits.max_connections > 64
        || limits.login_timeout.is_zero()
        || limits.idle_timeout.is_zero()
        || limits.messages_per_second == 0
    {
        return Err("invalid relay limits".to_owned());
    }
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let shared = Arc::new(Mutex::new(Shared::default()));
    let next_id = AtomicU64::new(1);
    let mut connections: Vec<(TcpStream, thread::JoinHandle<()>)> = Vec::new();
    let result = loop {
        let mut index = 0;
        while index < connections.len() {
            if connections[index].1.is_finished() {
                let (_, worker) = connections.swap_remove(index);
                let _ = worker.join();
            } else {
                index += 1;
            }
        }
        if shutdown.load(Ordering::Relaxed) {
            break Ok(());
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if connections.len() >= limits.max_connections {
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                if stream.set_nonblocking(false).is_err()
                    || stream
                        .set_read_timeout(Some(Duration::from_millis(100)))
                        .is_err()
                    || stream
                        .set_write_timeout(Some(Duration::from_secs(2)))
                        .is_err()
                    || stream.set_nodelay(true).is_err()
                {
                    continue;
                }
                let Ok(control) = stream.try_clone() else {
                    continue;
                };
                let id = next_id.fetch_add(1, Ordering::Relaxed);
                let shared = Arc::clone(&shared);
                match thread::Builder::new()
                    .name(format!("relay-{id}"))
                    .spawn(move || handle_connection(id, stream, shared, limits))
                {
                    Ok(worker) => connections.push((control, worker)),
                    Err(_) => {
                        let _ = control.shutdown(Shutdown::Both);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => break Err(format!("multiplayer accept failed: {error}")),
        }
    };
    // Interrupt and join all owners; closing just the listener leaks readers.
    for (stream, _) in &connections {
        let _ = stream.shutdown(Shutdown::Both);
    }
    for (_, worker) in connections {
        let _ = worker.join();
    }
    result
}
fn enqueue(
    shared: &Mutex<Shared>,
    id: u64,
    sender: &SyncSender<Message>,
    message: Message,
) -> bool {
    if sender.try_send(message).is_ok() {
        return true;
    }
    if let Some(socket) = shared
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .sockets
        .get(&id)
    {
        let _ = socket.shutdown(Shutdown::Both);
    }
    false
}
fn handle_connection(id: u64, stream: TcpStream, shared: Arc<Mutex<Shared>>, limits: ServerLimits) {
    let Ok(writer_stream) = stream.try_clone() else {
        return;
    };
    let Ok(control) = stream.try_clone() else {
        return;
    };
    let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
    let Ok(writer) = thread::Builder::new()
        .name(format!("relay-writer-{id}"))
        .spawn(move || writer_loop(writer_stream, receiver))
    else {
        return;
    };
    {
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        state.senders.insert(id, sender.clone());
        state.sockets.insert(id, control);
    }
    let send = |message| enqueue(&shared, id, &sender, message);
    let mut reader = BufReader::new(stream);
    let mut username = None;
    let mut window = Instant::now();
    let mut messages = 0usize;
    loop {
        let timeout = if username.is_none() {
            limits.login_timeout
        } else {
            limits.idle_timeout
        };
        let line = match read_bounded_line_until(&mut reader, Some(Instant::now() + timeout)) {
            Ok(Some(line)) => line,
            _ => break,
        };
        if window.elapsed() >= Duration::from_secs(1) {
            window = Instant::now();
            messages = 0;
        }
        messages += 1;
        if messages > limits.messages_per_second {
            break;
        }
        let Ok(message) = decode_line(&line) else {
            send(Message::Notify {
                text: "invalid protocol message".to_owned(),
            });
            break;
        };
        if username.is_none() {
            let Message::Login {
                username: requested,
                game_version,
                protocol_version,
            } = message
            else {
                send(Message::Notify {
                    text: "LOGIN must be the first message".to_owned(),
                });
                break;
            };
            match session::accept_login(&shared, id, &requested, &game_version, protocol_version) {
                Ok(accepted) => {
                    username = Some(accepted.username.clone());
                    if !send(session::init_message(id)) || !finish_login(&shared, id, &sender) {
                        break;
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
                    send(Message::Notify { text });
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
            } => match session::apply_move(
                &shared,
                id,
                username.as_deref().unwrap_or_default(),
                x,
                y,
                level,
                health,
            ) {
                Ok(player) => broadcast(&shared, Some(id), player),
                Err(text) => {
                    send(Message::Notify { text });
                    break;
                }
            },
            Message::Ping { nonce } => {
                if !send(Message::Pong { nonce }) {
                    break;
                }
            }
            Message::Disconnect => break,
            _ => {
                send(Message::Notify {
                    text: "message is server-only or invalid in this state".to_owned(),
                });
                break;
            }
        }
    }
    if session::end_session(&shared, id) {
        broadcast(&shared, None, Message::Remove { client_id: id });
    }
    drop(sender);
    let _ = writer.join();
}
// Snapshot presence and enable broadcasts in one lock acquisition. A peer that
// logs in or moves between accept_login and INIT must not vanish from replay.
fn finish_login(shared: &Mutex<Shared>, id: u64, sender: &SyncSender<Message>) -> bool {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    let users = state
        .peers
        .iter()
        .filter(|(peer_id, _)| **peer_id != id)
        .map(|(&peer_id, peer)| (peer_id, peer.username.clone()))
        .collect();
    if sender.try_send(Message::Usernames { users }).is_err() {
        return false;
    }
    for peer in state
        .peers
        .iter()
        .filter(|(peer_id, _)| **peer_id != id)
        .map(|(_, peer)| peer)
    {
        if let Some(latest) = &peer.latest
            && sender
                .try_send(Message::Player {
                    state: latest.clone(),
                })
                .is_err()
        {
            return false;
        }
    }
    if let Some(peer) = state.peers.get_mut(&id) {
        peer.ready = true;
        true
    } else {
        false
    }
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
    let _ = stream.shutdown(Shutdown::Both);
}
fn broadcast(shared: &Mutex<Shared>, except: Option<u64>, message: Message) {
    let senders = {
        let state = shared.lock().unwrap_or_else(|error| error.into_inner());
        state
            .peers
            .iter()
            .filter(|(id, peer)| Some(**id) != except && peer.ready)
            .map(|(id, _)| id)
            .filter_map(|id| state.senders.get(id).map(|sender| (*id, sender.clone())))
            .collect::<Vec<_>>()
    };
    for (id, sender) in senders {
        enqueue(shared, id, &sender, message.clone());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_replay_includes_peer_joining_after_initial_acceptance() {
        use minicraft_common::protocol::{GAME_VERSION, PROTOCOL_VERSION};
        let shared = Mutex::new(Shared::default());
        let accepted =
            session::accept_login(&shared, 1, "Alice", GAME_VERSION, PROTOCOL_VERSION).unwrap();
        assert!(accepted.existing_users.is_empty());
        session::accept_login(&shared, 2, "Bob", GAME_VERSION, PROTOCOL_VERSION).unwrap();
        let moved = session::apply_move(&shared, 2, "Bob", 32, 48, 1, 10).unwrap();
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        sender.try_send(session::init_message(1)).unwrap();
        assert!(finish_login(&shared, 1, &sender));
        assert_eq!(receiver.try_recv().unwrap(), session::init_message(1));
        assert_eq!(
            receiver.try_recv().unwrap(),
            Message::Usernames {
                users: vec![(2, "Bob".to_owned())]
            }
        );
        assert_eq!(receiver.try_recv().unwrap(), moved);
        assert!(shared.lock().unwrap().peers[&1].ready);
    }

    #[test]
    fn slow_peer_queue_does_not_block_healthy_peer() {
        let (slow, slow_receiver) = mpsc::sync_channel(1);
        let (healthy, healthy_receiver) = mpsc::sync_channel(1);
        let mut state = Shared::default();
        for (id, sender) in [(1, slow), (2, healthy)] {
            state.senders.insert(id, sender);
            state.peers.insert(
                id,
                session::Peer {
                    username: id.to_string(),
                    latest: None,
                    ready: true,
                },
            );
        }
        state.senders[&1]
            .try_send(Message::Ping { nonce: 0 })
            .unwrap();
        let shared = Mutex::new(state);
        broadcast(&shared, None, Message::Remove { client_id: 3 });
        assert_eq!(
            healthy_receiver.try_recv().unwrap(),
            Message::Remove { client_id: 3 }
        );
        assert_eq!(
            slow_receiver.try_recv().unwrap(),
            Message::Ping { nonce: 0 }
        );
    }
}
