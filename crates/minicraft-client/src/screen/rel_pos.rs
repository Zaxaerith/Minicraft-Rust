//! Java RelPos geometry. Keep divisions separate to preserve odd-size rounding.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RelPos {
    x: i32,
    y: i32,
}

impl RelPos {
    pub const CENTER: Self = Self { x: 1, y: 1 };
    pub const BOTTOM: Self = Self { x: 1, y: 2 };
    pub const TOP: Self = Self { x: 1, y: 0 };
    pub const LEFT: Self = Self { x: 0, y: 1 };
    pub const RIGHT: Self = Self { x: 2, y: 1 };
    pub const TOP_LEFT: Self = Self { x: 0, y: 0 };
    pub const TOP_RIGHT: Self = Self { x: 2, y: 0 };
    pub const BOTTOM_LEFT: Self = Self { x: 0, y: 2 };
    pub const BOTTOM_RIGHT: Self = Self { x: 2, y: 2 };

    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x: x.clamp(0, 2),
            y: y.clamp(0, 2),
        }
    }

    pub fn x_index(self) -> i32 {
        self.x
    }

    pub fn y_index(self) -> i32 {
        self.y
    }

    pub fn opposite(self) -> Self {
        Self {
            x: 2 - self.x,
            y: 2 - self.y,
        }
    }

    pub fn in_rect(self, size: (i32, i32), rect: (i32, i32, i32, i32)) -> (i32, i32) {
        let (x, y, w, h) = rect;
        (
            x + w / 2 + (self.x - 1) * w / 2 - self.x * size.0 / 2,
            y + h / 2 + (self.y - 1) * h / 2 - self.y * size.1 / 2,
        )
    }

    pub fn around(self, size: (i32, i32), anchor: (i32, i32)) -> (i32, i32) {
        self.in_rect(
            size,
            (anchor.0 - size.0, anchor.1 - size.1, size.0 * 2, size.1 * 2),
        )
    }

    pub fn point(self, rect: (i32, i32, i32, i32)) -> (i32, i32) {
        (rect.0 + self.x * rect.2 / 2, rect.1 + self.y * rect.3 / 2)
    }
}
