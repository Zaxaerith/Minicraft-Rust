//! Java `level/tile/entity/*` — tile-attached entity state.
pub mod sign_tile_entity;

#[cfg(test)]
mod tests {
    #[test]
    fn sign_entity_module_is_registered() {
        assert!(super::sign_tile_entity::KIND_NAME.eq_ignore_ascii_case("sign"));
    }
}
