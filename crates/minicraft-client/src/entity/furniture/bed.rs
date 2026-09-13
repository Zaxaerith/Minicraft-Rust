//! Java Bed: sleep skip handled by World; entity only marks presence.
use super::FurnitureKind;

/// Java Bed.use: triggers sleep transition (World handles day skip).
pub fn is_bed(kind: FurnitureKind) -> bool {
    kind == FurnitureKind::Bed
}
