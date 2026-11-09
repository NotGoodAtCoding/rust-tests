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

/// Paper pattern design variants available on wrapping paper rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaperPattern {
    #[default]
    Plain,
    PolkaDots,
    Stripes,
}

/// Component attached to wrapping paper rolls specifying their pattern variant.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaperPatternRoll {
    pub pattern: PaperPattern,
}

impl Default for PaperPatternRoll {
    fn default() -> Self {
        Self {
            pattern: PaperPattern::Plain,
        }
    }
}

impl PaperPatternRoll {
    pub fn new(pattern: PaperPattern) -> Self {
        Self { pattern }
    }

    pub fn cycle(&mut self) -> PaperPattern {
        self.pattern = match self.pattern {
            PaperPattern::Plain => PaperPattern::PolkaDots,
            PaperPattern::PolkaDots => PaperPattern::Stripes,
            PaperPattern::Stripes => PaperPattern::Plain,
        };
        self.pattern
    }
}

/// Component attached to gift tag pen tools with an inscription.
#[derive(Component, Debug, Clone)]
pub struct GiftTagPenTool {
    pub inscription: String,
}

impl Default for GiftTagPenTool {
    fn default() -> Self {
        Self {
            inscription: "To: Santa's Favorite".to_string(),
        }
    }
}

impl GiftTagPenTool {
    pub fn new(inscription: impl Into<String>) -> Self {
        Self {
            inscription: inscription.into(),
        }
    }
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
    pub recipient_tag: Option<String>,
}

impl Default for Present {
    fn default() -> Self {
        Self {
            toy_name: "Wooden Train".to_string(),
            features: HashSet::new(),
            quality_score: 1.0,
            is_completed: false,
            recipient_tag: None,
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
    pub pattern: Option<PaperPattern>,
    pub inscription: Option<String>,
}

impl Default for UseToolOnPresentMessage {
    fn default() -> Self {
        Self {
            tool: WrappingTool::Scissors,
            present_entity: Entity::PLACEHOLDER,
            pattern: None,
            inscription: None,
        }
    }
}

impl UseToolOnPresentMessage {
    pub fn new(tool: WrappingTool, present_entity: Entity) -> Self {
        Self {
            tool,
            present_entity,
            pattern: None,
            inscription: None,
        }
    }

    pub fn with_pattern(mut self, pattern: PaperPattern) -> Self {
        self.pattern = Some(pattern);
        self
    }

    pub fn with_inscription(mut self, inscription: impl Into<String>) -> Self {
        self.inscription = Some(inscription.into());
        self
    }
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
                    match msg.pattern {
                        Some(PaperPattern::PolkaDots) => {
                            present.features.insert(WrappingFeature::PolkaDotPattern);
                            present.features.remove(&WrappingFeature::StripedPattern);
                        }
                        Some(PaperPattern::Stripes) => {
                            present.features.insert(WrappingFeature::StripedPattern);
                            present.features.remove(&WrappingFeature::PolkaDotPattern);
                        }
                        _ => {}
                    }
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
                    if let Some(ref text) = msg.inscription {
                        present.recipient_tag = Some(text.clone());
                    } else if present.recipient_tag.is_none() {
                        present.recipient_tag = Some("To: Good Child".to_string());
                    }
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
