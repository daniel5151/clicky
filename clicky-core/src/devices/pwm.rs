pub trait PwmSink: std::fmt::Debug + Send + Sync {
    /// Notify PWM channel update.
    ///
    /// - `frequency` is in Hz
    /// - `duty` is within `[0.0, 1.0)`
    fn update(&mut self, enabled: bool, frequency: f32, duty: f32);
}
