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
