//! Logical-clock animation state from Java SpriteAnimation.render.
//! Sampling precedes advancement: repeated draws in one tick see updated state.
#[derive(Clone)]
pub struct SpriteMeta {
    pub frames: i32,
    pub frame_time: i32,
    pub border: Option<String>,
    pub corner: Option<String>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            frames: 1,
            frame_time: 0,
            border: None,
            corner: None,
        }
    }
}

pub struct SpriteAnimation {
    frame: i32,
    frame_tick: i32,
    last_tick: i32,
    last_millis: i64,
    frames: i32,
    frame_time: i32,
}

impl SpriteAnimation {
    pub fn new(frames: i32, frame_time: i32, tick: i32, millis: i64) -> Self {
        Self {
            frame: 0,
            frame_tick: 0,
            last_tick: tick,
            last_millis: millis,
            frames: frames.max(1),
            frame_time,
        }
    }

    pub fn frame(&self) -> usize {
        self.frame.max(0) as usize
    }

    /// Called after drawing, never once per screen frame or simulation update.
    pub fn after_render(&mut self, tick: i32, millis: i64, world_enter_millis: i64) {
        if self.frames <= 1 {
            return;
        }
        if self.last_millis < world_enter_millis {
            self.last_tick = tick;
            self.frame = 0;
            self.frame_tick = 0;
        }
        self.frame_tick = self
            .frame_tick
            .wrapping_add(tick.wrapping_sub(self.last_tick));
        self.last_tick = tick;
        self.last_millis = millis;
        // Invalid animation metadata is rejected by the resource adapter;
        // this guard also prevents divide-by-zero for direct callers.
        if self.frame_time > 0 && self.frame_tick >= self.frame_time {
            self.frame = self.frame.wrapping_add(self.frame_tick / self.frame_time);
            self.frame_tick %= self.frame_time;
            if self.frame >= self.frames - 1 {
                self.frame %= self.frames;
            }
        }
    }

    /// Java refreshAnimation resets frame counters but retains clock history.
    pub fn refresh(&mut self, frames: i32, frame_time: i32) {
        self.frame = 0;
        self.frame_tick = 0;
        self.frames = frames.max(1);
        self.frame_time = frame_time;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_frame_animation_never_advances() {
        let mut anim = SpriteAnimation::new(1, 4, 0, 100);
        anim.after_render(20, 200, 0);
        assert_eq!(anim.frame(), 0);
    }

    #[test]
    fn frame_advances_by_frametime_ticks() {
        let mut anim = SpriteAnimation::new(4, 3, 0, 100);
        anim.after_render(2, 101, 0);
        assert_eq!(anim.frame(), 0, "2 < frametime");
        anim.after_render(3, 102, 0);
        // frametick = 3 → frame += 1
        assert_eq!(anim.frame(), 1);
        anim.after_render(6, 103, 0);
        // frametick += 3 → frame += 1
        assert_eq!(anim.frame(), 2);
    }

    #[test]
    fn frame_wraps_using_java_modulo_when_at_or_past_last() {
        let mut anim = SpriteAnimation::new(4, 1, 0, 100);
        // Each tick advances one frame; at length-1 Java mods (keeps last), then wraps.
        anim.after_render(1, 101, 0);
        assert_eq!(anim.frame(), 1);
        anim.after_render(2, 102, 0);
        assert_eq!(anim.frame(), 2);
        anim.after_render(3, 103, 0);
        // frame becomes 3 == length-1 → 3 % 4 = 3
        assert_eq!(anim.frame(), 3);
        anim.after_render(4, 104, 0);
        // frame becomes 4 → 4 % 4 = 0
        assert_eq!(anim.frame(), 0);
    }

    #[test]
    fn world_enter_resets_frame_clock() {
        let mut anim = SpriteAnimation::new(4, 1, 0, 100);
        anim.after_render(5, 150, 0);
        assert_ne!(anim.frame(), 0);
        // New world entered after lastMillis → reset.
        anim.after_render(50, 200, 250);
        assert_eq!(anim.frame(), 0);
    }

    #[test]
    fn refresh_resets_frames_but_keeps_clock() {
        let mut anim = SpriteAnimation::new(4, 1, 10, 100);
        anim.after_render(15, 150, 0);
        assert_ne!(anim.frame(), 0);
        anim.refresh(2, 2);
        assert_eq!(anim.frame(), 0);
        // last_tick still 15; a tick of 17 with frametime 2 advances one frame.
        anim.after_render(17, 160, 0);
        assert_eq!(anim.frame(), 1);
    }

    #[test]
    fn zero_frame_time_does_not_divide_by_zero() {
        let mut anim = SpriteAnimation::new(4, 0, 0, 100);
        anim.after_render(50, 150, 0);
        assert_eq!(anim.frame(), 0);
    }

    #[test]
    fn meta_defaults_match_java_sprite_meta() {
        let meta = SpriteMeta::default();
        assert_eq!(meta.frames, 1);
        assert_eq!(meta.frame_time, 0);
        assert!(meta.border.is_none());
        assert!(meta.corner.is_none());
    }
}
