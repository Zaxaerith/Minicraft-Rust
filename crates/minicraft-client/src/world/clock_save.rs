//! Preserve the existing flat Rust save fields without coupling core to serde.
use crate::core::clock::{Clock, SharedClock};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct SavedClock {
    #[serde(default)]
    game_time: i32,
    day_tick: u32,
}

pub(super) fn serialize<S: Serializer>(
    clock: &SharedClock,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let clock = clock.get();
    SavedClock {
        game_time: clock.game_time,
        day_tick: clock.day_tick,
    }
    .serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SharedClock, D::Error> {
    let saved = SavedClock::deserialize(deserializer)?;
    Ok(SharedClock::new(Clock {
        game_time: saved.game_time,
        day_tick: saved.day_tick,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize, Deserialize)]
    struct Envelope {
        tick: u64,
        #[serde(flatten, with = "super")]
        clock: SharedClock,
    }
    #[test]
    fn flat_clock_save_keeps_legacy_fields_and_signed_time() {
        let old: Envelope = serde_json::from_str(r#"{"tick":9,"day_tick":123}"#).unwrap();
        assert_eq!(old.clock.get().game_time, 0);
        let saved = Envelope {
            tick: 9,
            clock: SharedClock::new(Clock {
                game_time: i32::MIN,
                day_tick: 123,
            }),
        };
        assert_eq!(
            serde_json::to_value(saved).unwrap(),
            serde_json::json!({"tick":9,"game_time":i32::MIN,"day_tick":123})
        );
        assert!(serde_json::from_str::<Envelope>(r#"{"tick":9}"#).is_err());
        assert!(
            serde_json::from_str::<Envelope>(r#"{"tick":9,"day_tick":123,"game_time":2147483648}"#)
                .is_err()
        );
    }
}
