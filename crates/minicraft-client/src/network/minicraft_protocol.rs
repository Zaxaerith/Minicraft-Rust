//! Java `network/MinicraftProtocol.java` 1:1.
//!
//! The shared crate stays the wire-format authority (server links it too);
//! this module re-exports that registry under the Java names and locks the
//! port / enum-order / set membership the Java interface defines.

pub use minicraft_common::protocol::{
    DEFAULT_PORT, GAME_VERSION, InputType, MAX_MESSAGE_BYTES, Message, PROTOCOL_VERSION,
    PlayerState, decode_line, encode_line, read_bounded_line, sanitize_username,
};

/// Java `MinicraftProtocol.PORT`.
pub const PORT: u16 = DEFAULT_PORT;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn port_is_java_4225() {
        assert_eq!(PORT, 4225);
        assert_eq!(DEFAULT_PORT, 4225);
        assert_eq!(PORT, DEFAULT_PORT);
    }

    #[test]
    fn input_type_registry_matches_java_declaration_order() {
        // Java: INVALID, PING, USERNAMES, LOGIN, GAME, INIT, LOAD, TILES,
        // ENTITIES, TILE, ENTITY, PLAYER, MOVE, ADD, REMOVE, DISCONNECT,
        // SAVE, NOTIFY, INTERACT, PUSH, PICKUP, CHESTIN, CHESTOUT, ADDITEMS,
        // BED, POTION, HURT, DIE, RESPAWN, DROP, STAMINA, SHIRT, STOPFISHING.
        let expected = [
            "INVALID",
            "PING",
            "USERNAMES",
            "LOGIN",
            "GAME",
            "INIT",
            "LOAD",
            "TILES",
            "ENTITIES",
            "TILE",
            "ENTITY",
            "PLAYER",
            "MOVE",
            "ADD",
            "REMOVE",
            "DISCONNECT",
            "SAVE",
            "NOTIFY",
            "INTERACT",
            "PUSH",
            "PICKUP",
            "CHESTIN",
            "CHESTOUT",
            "ADDITEMS",
            "BED",
            "POTION",
            "HURT",
            "DIE",
            "RESPAWN",
            "DROP",
            "STAMINA",
            "SHIRT",
            "STOPFISHING",
        ];
        assert_eq!(InputType::VALUES.len(), expected.len());
        for (index, name) in expected.iter().enumerate() {
            assert_eq!(InputType::VALUES[index] as usize, index);
            assert_eq!(InputType::VALUES[index].wire_name(), *name);
        }
    }

    #[test]
    fn java_role_sets_are_preserved() {
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
    fn login_and_heartbeat_wire_fields_roundtrip() {
        let login = Message::Login {
            username: "Alice".to_owned(),
            game_version: GAME_VERSION.to_owned(),
            protocol_version: PROTOCOL_VERSION,
        };
        let init = Message::Init {
            client_id: 1,
            protocol_version: PROTOCOL_VERSION,
            server_version: GAME_VERSION.to_owned(),
        };
        let ping = Message::Ping { nonce: 0x224 };
        let pong = Message::Pong { nonce: 0x224 };
        for sample in [login, init, ping, pong] {
            let frame = encode_line(&sample).unwrap();
            assert!(frame.ends_with(b"\n"));
            let decoded = decode_line(std::str::from_utf8(&frame).unwrap()).unwrap();
            assert_eq!(decoded, sample);
        }
    }

    #[test]
    fn version_compare_matches_java_ordering_rules() {
        let release = crate::saveload::Version::parse("2.2.4");
        let newer = crate::saveload::Version::parse("2.3.0");
        let older = crate::saveload::Version::parse("2.2.3");
        assert_eq!(newer.compare(&release), Ordering::Greater);
        assert_eq!(older.compare(&release), Ordering::Less);
        assert_eq!(
            release.compare(&crate::saveload::Version::parse("2.2.4")),
            Ordering::Equal
        );
    }
}
