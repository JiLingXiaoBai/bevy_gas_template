//! Linear level formulas shared by runtime effects and authoring tools.

use bevy_gas::{ModifierEvaluationContext, ModifierMagnitudeCalculation};

pub(crate) fn evaluate_linear(base: f32, per_level: f32, level: u32) -> f64 {
    f64::from(base) + f64::from(per_level) * f64::from(level.saturating_sub(1))
}

pub(super) struct LinearLevelMagnitude {
    pub(super) base: f32,
    pub(super) per_level: f32,
}

impl ModifierMagnitudeCalculation for LinearLevelMagnitude {
    fn calculate(&self, context: &dyn ModifierEvaluationContext) -> f32 {
        // Preserve overflow as non-finite so GAS rejects the effect before applying modifiers.
        evaluate_linear(self.base, self.per_level, context.level()) as f32
    }
}
