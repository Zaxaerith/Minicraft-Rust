//! Java `util/Vector2.java`.
//!
//! Note: Java `normalized()` divides by `Math.max(x, y)`, not by vector length.
//! That quirk is preserved 1:1.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Java `Vector2.normalized()` — divides by `Math.max(x, y)`.
    pub fn normalized(&self) -> Self {
        let max = self.x.max(self.y);
        Self {
            x: self.x / max,
            y: self.y / max,
        }
    }

    /// Java `Vector2.normalize(vec)`.
    pub fn normalize(vec: &Self) -> Self {
        vec.normalized()
    }

    /// Java `Vector2.distance(x, y)` — Euclidean length of `(x, y)`.
    pub fn distance(x: f64, y: f64) -> f64 {
        (x * x + y * y).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::Vector2;

    #[test]
    fn normalized_uses_max_axis_like_java() {
        let v = Vector2::new(3.0, 6.0);
        let n = v.normalized();
        assert_eq!(n.x, 0.5);
        assert_eq!(n.y, 1.0);
        assert_eq!(Vector2::normalize(&v), n);
    }

    #[test]
    fn distance_is_euclidean_length() {
        assert_eq!(Vector2::distance(3.0, 4.0), 5.0);
    }
}
