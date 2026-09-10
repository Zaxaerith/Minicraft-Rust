//! Per-client login, presence, state, and heartbeat lifecycle.
use minicraft_common::protocol::{GAME_VERSION, Message, PlayerState, PROTOCOL_VERSION};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        mpsc::Sender,
    },
};

#[derive(Clone)]
pub struct Peer {
    pub username: String,
    pub latest: Option<PlayerState>,
}

#[derive(Default)]
pub struct Shared {
    pub senders: HashMap<u64, Sender<Message>>,
    pub peers: HashMap<u64, Peer>,
}

/// Result of accepting a LOGIN as the first message on a fresh connection.
#[derive(Debug)]
pub struct LoginAccepted {
    pub username: String,
    pub existing_users: Vec<(u64, String)>,
    pub replay_states: Vec<PlayerState>,
}

/// Validate a LOGIN frame and insert the peer. On success the caller must
/// send INIT/USERNAMES/replay and broadcast ADD to the other peers.
pub fn accept_login(
    shared: &Mutex<Shared>,
    client_id: u64,
    username: &str,
    game_version: &str,
    protocol_version: u32,
) -> Result<LoginAccepted, String> {
    if protocol_version != PROTOCOL_VERSION || game_version != GAME_VERSION {
        return Err(format!(
            "incompatible client {game_version}/protocol {protocol_version}"
        ));
    }
    let username = minicraft_common::protocol::sanitize_username(username);
    if username.is_empty() {
        return Err("username must contain a letter or number".to_owned());
    }
    let mut state = shared.lock().unwrap();
    let existing_users = state
        .peers
        .iter()
        .map(|(&peer_id, peer)| (peer_id, peer.username.clone()))
        .collect();
    let replay_states = state
        .peers
        .values()
        .filter_map(|peer| peer.latest.clone())
        .collect::<Vec<_>>();
    state.peers.insert(
        client_id,
        Peer {
            username: username.clone(),
            latest: None,
        },
    );
    Ok(LoginAccepted {
        username,
        existing_users,
        replay_states,
    })
}

pub fn init_message(client_id: u64) -> Message {
    Message::Init {
        client_id,
        protocol_version: PROTOCOL_VERSION,
        server_version: GAME_VERSION.to_owned(),
    }
}

/// Store the latest MOVE snapshot for a logged-in peer and build the PLAYER
/// broadcast sent to everyone else.
pub fn apply_move(
    shared: &Mutex<Shared>,
    client_id: u64,
    username: &str,
    x: i32,
    y: i32,
    level: usize,
    health: u8,
) -> Message {
    let state = PlayerState {
        client_id,
        username: username.to_owned(),
        x,
        y,
        level,
        health,
    };
    if let Some(peer) = shared.lock().unwrap().peers.get_mut(&client_id) {
        peer.latest = Some(state.clone());
    }
    Message::Player { state }
}

/// Drop the peer and report whether a logged-in presence should be announced.
pub fn end_session(shared: &Mutex<Shared>, client_id: u64) -> bool {
    let mut state = shared.lock().unwrap();
    let was_logged_in = state.peers.remove(&client_id).is_some();
    state.senders.remove(&client_id);
    was_logged_in
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn accept_login_rejects_version_mismatch_and_empty_username() {
        let shared = Mutex::new(Shared::default());
        let error = accept_login(&shared, 1, "Alice", "0.0.0", PROTOCOL_VERSION).unwrap_err();
        assert!(error.contains("incompatible client"));
        let error = accept_login(&shared, 1, "Alice", GAME_VERSION, 999).unwrap_err();
        assert!(error.contains("incompatible client"));
        let error = accept_login(&shared, 1, "..", GAME_VERSION, PROTOCOL_VERSION).unwrap_err();
        assert!(error.contains("username must contain"));
    }

    #[test]
    fn accept_login_sanitizes_and_replays_presence() {
        let shared = Mutex::new(Shared::default());
        let first = accept_login(&shared, 1, "Alice!", GAME_VERSION, PROTOCOL_VERSION).unwrap();
        assert_eq!(first.username, "Alice");
        assert!(first.existing_users.is_empty());
        apply_move(&shared, 1, &first.username, 80, 96, 1, 9);
        let second = accept_login(&shared, 2, "Bob", GAME_VERSION, PROTOCOL_VERSION).unwrap();
        assert_eq!(second.existing_users, vec![(1, "Alice".to_owned())]);
        assert_eq!(second.replay_states.len(), 1);
        assert_eq!(second.replay_states[0].x, 80);
        assert_eq!(second.replay_states[0].health, 9);
        assert!(end_session(&shared, 1));
        assert!(!end_session(&shared, 1));
    }

    #[test]
    fn apply_move_updates_latest_snapshot() {
        let (sender, _receiver) = mpsc::channel();
        let mut shared = Shared::default();
        shared.senders.insert(9, sender);
        shared.peers.insert(
            9,
            Peer {
                username: "Zoe".to_owned(),
                latest: None,
            },
        );
        let shared = Mutex::new(shared);
        let message = apply_move(&shared, 9, "Zoe", 1, 2, 0, 10);
        let Message::Player { state } = message else {
            panic!("expected PLAYER");
        };
        assert_eq!(shared.lock().unwrap().peers[&9].latest.as_ref(), Some(&state));
        assert!(end_session(&shared, 9));
        assert!(shared.lock().unwrap().senders.is_empty());
    }
}
