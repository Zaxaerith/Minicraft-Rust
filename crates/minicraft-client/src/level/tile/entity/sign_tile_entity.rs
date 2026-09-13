//! Java `entity.SignTileEntity` — marker entity spawned when a sign is placed.

/// Display/kind name used by the client entity registry.
pub const KIND_NAME: &str = "sign";

/// Java `SignTileEntity` has no tick logic of its own; text lives in the sign map.
pub fn is_marker_only() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_entity_is_a_marker() {
        assert!(is_marker_only());
        assert_eq!(KIND_NAME, "sign");
    }
}
