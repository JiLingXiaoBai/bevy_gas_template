/// Samples a nonlooping clip from a task interval without a render clock.
pub fn phase_sample(tick: u32, duration: u32, frames: usize) -> usize {
    if frames == 0 || duration == 0 {
        return 0;
    }
    ((u128::from(tick.min(duration - 1)) * frames as u128) / u128::from(duration))
        .min((frames - 1) as u128) as usize
}
