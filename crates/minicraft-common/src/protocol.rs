//! Shared multiplayer protocol matching MinicraftProtocol.java registry order.
use serde::{Deserialize, Serialize};
use std::io::BufRead;

/// Java MinicraftProtocol.PORT.
pub const DEFAULT_PORT: u16 = 4225;
pub const PROTOCOL_VERSION: u32 = 1;
pub const GAME_VERSION: &str = "2.2.4-rust";
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// Java MinicraftProtocol.InputType order (INVALID=0 … STOPFISHING=32).
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

impl InputType {
    pub const VALUES: [Self; 33] = [
        Self::Invalid,
        Self::Ping,
        Self::Usernames,
        Self::Login,
        Self::Game,
        Self::Init,
        Self::Load,
        Self::Tiles,
        Self::Entities,
        Self::Tile,
        Self::Entity,
        Self::Player,
        Self::Move,
        Self::Add,
        Self::Remove,
        Self::Disconnect,
        Self::Save,
        Self::Notify,
        Self::Interact,
        Self::Push,
        Self::Pickup,
        Self::Chestin,
        Self::Chestout,
        Self::Additems,
        Self::Bed,
        Self::Potion,
        Self::Hurt,
        Self::Die,
        Self::Respawn,
        Self::Drop,
        Self::Stamina,
        Self::Shirt,
        Self::Stopfishing,
    ];

    pub const SERVER_ONLY: [Self; 10] = [
        Self::Init,
        Self::Tiles,
        Self::Entities,
        Self::Add,
        Self::Remove,
        Self::Hurt,
        Self::Game,
        Self::Additems,
        Self::Stamina,
        Self::Stopfishing,
    ];

    pub const ENTITY_UPDATES: [Self; 3] = [Self::Entity, Self::Add, Self::Remove];

    pub const TILE_UPDATES: [Self; 1] = [Self::Tile];

    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Invalid => "INVALID",
            Self::Ping => "PING",
            Self::Usernames => "USERNAMES",
            Self::Login => "LOGIN",
            Self::Game => "GAME",
            Self::Init => "INIT",
            Self::Load => "LOAD",
            Self::Tiles => "TILES",
            Self::Entities => "ENTITIES",
            Self::Tile => "TILE",
            Self::Entity => "ENTITY",
            Self::Player => "PLAYER",
            Self::Move => "MOVE",
            Self::Add => "ADD",
            Self::Remove => "REMOVE",
            Self::Disconnect => "DISCONNECT",
            Self::Save => "SAVE",
            Self::Notify => "NOTIFY",
            Self::Interact => "INTERACT",
            Self::Push => "PUSH",
            Self::Pickup => "PICKUP",
            Self::Chestin => "CHESTIN",
            Self::Chestout => "CHESTOUT",
            Self::Additems => "ADDITEMS",
            Self::Bed => "BED",
            Self::Potion => "POTION",
            Self::Hurt => "HURT",
            Self::Die => "DIE",
            Self::Respawn => "RESPAWN",
            Self::Drop => "DROP",
            Self::Stamina => "STAMINA",
            Self::Shirt => "SHIRT",
            Self::Stopfishing => "STOPFISHING",
        }
    }
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

/// Line-delimited JSON messages used by both client and server.
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

pub fn sanitize_username(username: &str) -> String {
    username
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .take(24)
        .collect()
}

/// Encode one JSON message as a single newline-terminated wire frame.
pub fn encode_line(message: &Message) -> std::io::Result<Vec<u8>> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    encoded.push(b'\n');
    Ok(encoded)
}

/// Decode one already-framed line (trailing newline optional).
pub fn decode_line(line: &str) -> Result<Message, serde_json::Error> {
    serde_json::from_str(line.trim())
}

/// Read one bounded UTF-8 line. `Ok(None)` means clean EOF before any bytes.
pub fn read_bounded_line(reader: &mut impl BufRead) -> std::io::Result<Option<String>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_registry_matches_java_2_2_4_order_and_port() {
        assert_eq!(DEFAULT_PORT, 4225);
        assert_eq!(PROTOCOL_VERSION, 1);
        assert_eq!(GAME_VERSION, "2.2.4-rust");
        assert_eq!(InputType::Invalid as usize, 0);
        assert_eq!(InputType::Stopfishing as usize, 32);
        assert_eq!(InputType::VALUES.len(), 33);
        assert_eq!(
            serde_json::to_string(&InputType::Stopfishing).unwrap(),
            r#""STOPFISHING""#
        );
        assert_eq!(
            serde_json::to_string(&InputType::Usernames).unwrap(),
            r#""USERNAMES""#
        );
        for (index, input) in InputType::VALUES.iter().enumerate() {
            assert_eq!(*input as usize, index);
            assert_eq!(input.wire_name(), serde_json::to_string(input).unwrap().trim_matches('"'));
        }
    }

    #[test]
    fn server_only_and_update_sets_match_java() {
        assert_eq!(
            InputType::SERVER_ONLY,
            [
                InputType::Init,
                InputType::Tiles,
                InputType::Entities,
                InputType::Add,
                InputType::Remove,
                InputType::Hurt,
                InputType::Game,
                InputType::Additems,
                InputType::Stamina,
                InputType::Stopfishing,
            ]
        );
        assert_eq!(
            InputType::ENTITY_UPDATES,
            [InputType::Entity, InputType::Add, InputType::Remove]
        );
        assert_eq!(InputType::TILE_UPDATES, [InputType::Tile]);
    }

    #[test]
    fn usernames_are_sanitized_like_java() {
        assert_eq!(sanitize_username("  A!lice_世界-123  "), "Alice_-123");
        assert_eq!(sanitize_username(&"x".repeat(100)).len(), 24);
        assert_eq!(sanitize_username(".."), "");
    }

    #[test]
    fn message_wire_roundtrip_covers_login_presence_and_heartbeat() {
        let samples = [
            Message::Login {
                username: "Alice".to_owned(),
                game_version: GAME_VERSION.to_owned(),
                protocol_version: PROTOCOL_VERSION,
            },
            Message::Init {
                client_id: 7,
                protocol_version: PROTOCOL_VERSION,
                server_version: GAME_VERSION.to_owned(),
            },
            Message::Usernames {
                users: vec![(1, "Alice".to_owned()), (2, "Bob".to_owned())],
            },
            Message::Add {
                client_id: 2,
                username: "Bob".to_owned(),
            },
            Message::Remove { client_id: 2 },
            Message::Player {
                state: PlayerState {
                    client_id: 1,
                    username: "Alice".to_owned(),
                    x: 80,
                    y: 96,
                    level: 1,
                    health: 9,
                },
            },
            Message::Move {
                x: -3,
                y: 4,
                level: 0,
                health: 10,
            },
            Message::Ping { nonce: 0x224 },
            Message::Pong { nonce: 0x224 },
            Message::Notify {
                text: "ready".to_owned(),
            },
            Message::Disconnect,
        ];
        for sample in samples {
            let frame = encode_line(&sample).unwrap();
            assert!(frame.ends_with(b"\n"));
            let decoded = decode_line(std::str::from_utf8(&frame).unwrap()).unwrap();
            assert_eq!(decoded, sample);
        }
    }

    #[test]
    fn protocol_lines_are_bounded() {
        let mut acceptable = std::io::Cursor::new(vec![b'a'; MAX_MESSAGE_BYTES - 1]);
        assert_eq!(
            read_bounded_line(&mut acceptable).unwrap().unwrap().len(),
            MAX_MESSAGE_BYTES - 1
        );
        let mut oversized = std::io::Cursor::new(vec![b'a'; MAX_MESSAGE_BYTES + 1]);
        let error = read_bounded_line(&mut oversized).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
