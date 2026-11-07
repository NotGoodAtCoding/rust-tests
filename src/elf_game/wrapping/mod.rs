use std::collections::HashSet;
use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader};

/// Individual visual and physical features a wrapped present can possess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrappingFeature {
    PaperSized,
    PaperWrapped,
    TapedCorners,
    TapedCenter,
    RibbonTied,
    DecorativeBow,
    GiftTagAttached,
    PolkaDotPattern,
    StripedPattern,
    PlasmaShrinkSealed,
}

/// Instructions / requirements specifying the required set of features.
#[derive(Component, Debug, Clone)]
pub struct WrappingSpecification {
    pub required_features: HashSet<WrappingFeature>,
    pub forbidden_features: HashSet<WrappingFeature>,
}

impl Default for WrappingSpecification {
    fn default() -> Self {
        let mut required = HashSet::new();
        required.insert(WrappingFeature::PaperSized);
        required.insert(WrappingFeature::PaperWrapped);
        required.insert(WrappingFeature::TapedCorners);
        required.insert(WrappingFeature::DecorativeBow);
        Self {
            required_features: required,
            forbidden_features: HashSet::new(),
        }
    }
}

/// A present entity in the workshop, holding its current set of applied features and attributes.
#[derive(Component, Debug, Clone)]
pub struct Present {
    pub toy_name: String,
    pub features: HashSet<WrappingFeature>,
    pub quality_score: f32,
    pub is_completed: bool,
}

impl Default for Present {
    fn default() -> Self {
        Self {
            toy_name: "Wooden Train".to_string(),
            features: HashSet::new(),
            quality_score: 1.0,
            is_completed: false,
        }
    }
}

impl Present {
    /// Returns true if all required features are met and no forbidden features are present.
    pub fn meets_specification(&self, spec: &WrappingSpecification) -> bool {
        let has_all_required = spec.required_features.iter().all(|req| self.features.contains(req));
        let has_no_forbidden = spec.forbidden_features.iter().all(|forb| !self.features.contains(forb));
        has_all_required && has_no_forbidden
    }

    /// Evaluates satisfaction percentage (0.0 to 1.0).
    pub fn specification_satisfaction(&self, spec: &WrappingSpecification) -> f32 {
        if spec.required_features.is_empty() {
            return 1.0;
        }
        let met_count = spec.required_features.iter().filter(|req| self.features.contains(req)).count();
        let penalty = spec.forbidden_features.iter().filter(|forb| self.features.contains(forb)).count();
        ((met_count as f32 / spec.required_features.len() as f32) - (penalty as f32 * 0.3)).clamp(0.0, 1.0)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrappingTool {
    Scissors,
    WrappingPaperRoll,
    TapeDispenser,
    RibbonSpool,
    DecorativeBowBox,
    GiftTagPen,
    ForbiddenPlasmaShrink,
}

#[derive(Message, Debug, Clone)]
pub struct UseToolOnPresentMessage {
    pub tool: WrappingTool,
    pub present_entity: Entity,
}

#[derive(Message, Debug, Clone)]
pub struct PresentDeliveredMessage {
    pub present_entity: Entity,
    pub quality: f32,
}

pub fn handle_present_wrapping(
    mut messages: MessageReader<UseToolOnPresentMessage>,
    mut present_query: Query<(&mut Present, Option<&WrappingSpecification>)>,
) {
    for msg in messages.read() {
        if let Ok((mut present, spec_opt)) = present_query.get_mut(msg.present_entity) {
            match msg.tool {
                WrappingTool::Scissors => {
                    present.features.insert(WrappingFeature::PaperSized);
                }
                WrappingTool::WrappingPaperRoll => {
                    present.features.insert(WrappingFeature::PaperWrapped);
                }
                WrappingTool::TapeDispenser => {
                    if !present.features.contains(&WrappingFeature::TapedCorners) {
                        present.features.insert(WrappingFeature::TapedCorners);
                    } else {
                        present.features.insert(WrappingFeature::TapedCenter);
                    }
                }
                WrappingTool::RibbonSpool => {
                    present.features.insert(WrappingFeature::RibbonTied);
                }
                WrappingTool::DecorativeBowBox => {
                    present.features.insert(WrappingFeature::DecorativeBow);
                }
                WrappingTool::GiftTagPen => {
                    present.features.insert(WrappingFeature::GiftTagAttached);
                }
                WrappingTool::ForbiddenPlasmaShrink => {
                    // Instant high-tech application of all fundamental features
                    present.features.insert(WrappingFeature::PaperSized);
                    present.features.insert(WrappingFeature::PaperWrapped);
                    present.features.insert(WrappingFeature::TapedCorners);
                    present.features.insert(WrappingFeature::TapedCenter);
                    present.features.insert(WrappingFeature::DecorativeBow);
                    present.features.insert(WrappingFeature::PlasmaShrinkSealed);
                    present.quality_score *= 1.5;
                }
            }

            // If a specification is present, check if all requirements are fulfilled
            if let Some(spec) = spec_opt {
                if present.meets_specification(spec) {
                    present.is_completed = true;
                }
            }
        }
    }
}

pub fn check_present_specifications(
    mut present_query: Query<(&mut Present, &WrappingSpecification)>,
) {
    for (mut present, spec) in &mut present_query {
        if present.meets_specification(spec) {
            present.is_completed = true;
        }
    }
}

pub struct WrappingPlugin;

impl Plugin for WrappingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UseToolOnPresentMessage>()
            .add_message::<PresentDeliveredMessage>()
            .add_systems(Update, (handle_present_wrapping, check_present_specifications).chain());
    }
}
