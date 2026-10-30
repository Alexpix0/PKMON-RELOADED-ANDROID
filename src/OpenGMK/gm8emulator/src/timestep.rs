//! Deterministic frame pacing and temporal accumulation ("Fix Your Timestep").

#[derive(Clone, Debug)]
pub struct TimeStepController {
    pub fixed_dt: f64,       // 1.0 / 60.0 (16.666 ms)
    pub accumulator: f64,
    pub current_time: f64,
}

impl TimeStepController {
    pub fn new(target_logic_fps: f64) -> Self {
        Self {
            fixed_dt: 1.0 / target_logic_fps.max(1.0),
            accumulator: 0.0,
            current_time: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }

    pub fn set_target_fps(&mut self, target_logic_fps: f64) {
        self.fixed_dt = 1.0 / target_logic_fps.max(1.0);
    }

    pub fn advance_frame<F, R>(&mut self, frame_time_seconds: f64, mut tick_step: F, mut render_lerp: R)
    where
        F: FnMut(),
        R: FnMut(f32),
    {
        let frame_time = frame_time_seconds.min(0.25); // Protección contra espiral de la muerte
        self.accumulator += frame_time;

        while self.accumulator >= self.fixed_dt {
            tick_step(); // Eventos Step, Alarms y Collisions discretos de GM8
            self.accumulator -= self.fixed_dt;
        }

        let alpha = ((self.accumulator / self.fixed_dt) as f32).clamp(0.0, 1.0);
        render_lerp(alpha); // Interpolación LERP continua para 90/120/144 Hz
    }
}
