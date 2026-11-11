use std::collections::HashSet;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy::ecs::message::{Message, MessageReader};

pub mod auto_wrapper;
pub use auto_wrapper::{
    AutoWrapperTool, WrappedElf, WrappedInPaper, calculate_wrap_duration,
    spawn_auto_wrapper_tool, auto_wrapper_usage_system, wrapped_elf_system,
};

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
    SnowflakePattern,
    PlasmaShrinkSealed,
}

/// Paper pattern design variants available on wrapping paper rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaperPattern {
    #[default]
    Plain,
    PolkaDots,
    Stripes,
    Snowflakes,
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
            PaperPattern::Stripes => PaperPattern::Snowflakes,
            PaperPattern::Snowflakes => PaperPattern::Plain,
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

/// Assorted raw toy types that arrive via the conveyor belt.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ToyType {
    #[default]
    WoodenTrain,
    TeddyBear,
    TinRobot,
}

impl ToyType {
    pub fn name(&self) -> &'static str {
        match self {
            ToyType::WoodenTrain => "Wooden Train",
            ToyType::TeddyBear => "Teddy Bear",
            ToyType::TinRobot => "Tin Robot",
        }
    }

    pub fn dimensions(&self) -> Vec3 {
        match self {
            ToyType::WoodenTrain => Vec3::new(0.65, 0.45, 0.45),
            ToyType::TeddyBear => Vec3::new(0.50, 0.55, 0.50),
            ToyType::TinRobot => Vec3::new(0.45, 0.65, 0.40),
        }
    }

    pub fn mass(&self) -> f32 {
        match self {
            ToyType::WoodenTrain => 1.2,
            ToyType::TeddyBear => 0.6,
            ToyType::TinRobot => 2.0,
        }
    }

    pub fn lattice_params(&self) -> (f32, f32) {
        match self {
            ToyType::WoodenTrain => (60.0, 6.0),
            ToyType::TeddyBear => (30.0, 3.5),
            ToyType::TinRobot => (120.0, 10.0),
        }
    }

    pub fn default_specification(&self) -> WrappingSpecification {
        let mut required = HashSet::new();
        let mut forbidden = HashSet::new();
        match self {
            ToyType::WoodenTrain => {
                required.insert(WrappingFeature::PaperSized);
                required.insert(WrappingFeature::PaperWrapped);
                required.insert(WrappingFeature::TapedCorners);
                required.insert(WrappingFeature::DecorativeBow);
                required.insert(WrappingFeature::GiftTagAttached);
            }
            ToyType::TeddyBear => {
                required.insert(WrappingFeature::PaperWrapped);
                required.insert(WrappingFeature::SnowflakePattern);
                required.insert(WrappingFeature::RibbonTied);
                forbidden.insert(WrappingFeature::PlasmaShrinkSealed);
            }
            ToyType::TinRobot => {
                required.insert(WrappingFeature::PaperWrapped);
                required.insert(WrappingFeature::StripedPattern);
                required.insert(WrappingFeature::TapedCorners);
                required.insert(WrappingFeature::TapedCenter);
                forbidden.insert(WrappingFeature::RibbonTied);
            }
        }
        WrappingSpecification {
            required_features: required,
            forbidden_features: forbidden,
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

impl WrappingSpecification {
    pub fn for_toy(toy_type: ToyType) -> Self {
        toy_type.default_specification()
    }
}

/// A present entity in the workshop, holding its current set of applied features and attributes.
#[derive(Component, Debug, Clone)]
pub struct Present {
    pub toy_type: ToyType,
    pub toy_name: String,
    pub features: HashSet<WrappingFeature>,
    pub quality_score: f32,
    pub is_completed: bool,
    pub recipient_tag: Option<String>,
}

impl Default for Present {
    fn default() -> Self {
        Self::new(ToyType::WoodenTrain)
    }
}

impl Present {
    pub fn new(toy_type: ToyType) -> Self {
        Self {
            toy_type,
            toy_name: toy_type.name().to_string(),
            features: HashSet::new(),
            quality_score: 1.0,
            is_completed: false,
            recipient_tag: None,
        }
    }
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
    AutoWrapper,
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
                            present.features.remove(&WrappingFeature::SnowflakePattern);
                        }
                        Some(PaperPattern::Stripes) => {
                            present.features.insert(WrappingFeature::StripedPattern);
                            present.features.remove(&WrappingFeature::PolkaDotPattern);
                            present.features.remove(&WrappingFeature::SnowflakePattern);
                        }
                        Some(PaperPattern::Snowflakes) => {
                            present.features.insert(WrappingFeature::SnowflakePattern);
                            present.features.remove(&WrappingFeature::PolkaDotPattern);
                            present.features.remove(&WrappingFeature::StripedPattern);
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
                WrappingTool::AutoWrapper => {
                    present.features.insert(WrappingFeature::PaperWrapped);
                    present.features.insert(WrappingFeature::StripedPattern);
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
            .add_systems(
                Update,
                (
                    handle_present_wrapping,
                    check_present_specifications,
                    auto_wrapper_usage_system,
                    wrapped_elf_system,
                ).chain(),
            );
    }
}

/// Generates a procedural 64x64 festive snowflake wrapping paper texture
pub fn generate_snowflake_pattern_image() -> Image {
    const W: u32 = 64;
    const H: u32 = 64;
    let mut data = vec![0u8; (W * H * 4) as usize];

    // Deep festive emerald pine green background
    let bg_r = 18;
    let bg_g = 68;
    let bg_b = 32;

    for i in (0..data.len()).step_by(4) {
        data[i] = bg_r;
        data[i + 1] = bg_g;
        data[i + 2] = bg_b;
        data[i + 3] = 255;
    }

    let set_pixel = |x: i32, y: i32, r: u8, g: u8, b: u8, data: &mut [u8]| {
        let x = x.rem_euclid(W as i32) as u32;
        let y = y.rem_euclid(H as i32) as u32;
        let idx = ((y * W + x) * 4) as usize;
        data[idx] = r;
        data[idx + 1] = g;
        data[idx + 2] = b;
        data[idx + 3] = 255;
    };

    let draw_snowflake = |cx: i32, cy: i32, radius: i32, data: &mut [u8]| {
        // Crisp snow white
        let (sr, sg, sb) = (248, 250, 255);
        // Central hub
        for dy in -1..=1 {
            for dx in -1..=1 {
                set_pixel(cx + dx, cy + dy, sr, sg, sb, data);
            }
        }
        // 6-fold radial symmetry arms
        for step in 1..=radius {
            // Horizontal and vertical crosses
            set_pixel(cx + step, cy, sr, sg, sb, data);
            set_pixel(cx - step, cy, sr, sg, sb, data);
            set_pixel(cx, cy + step, sr, sg, sb, data);
            set_pixel(cx, cy - step, sr, sg, sb, data);

            // Diagonal arms
            set_pixel(cx + step, cy + step, sr, sg, sb, data);
            set_pixel(cx - step, cy - step, sr, sg, sb, data);
            set_pixel(cx + step, cy - step, sr, sg, sb, data);
            set_pixel(cx - step, cy + step, sr, sg, sb, data);

            // Branching crystal barbs
            if step == radius / 2 {
                set_pixel(cx + step - 1, cy + step + 1, sr, sg, sb, data);
                set_pixel(cx + step + 1, cy + step - 1, sr, sg, sb, data);
                set_pixel(cx - step + 1, cy - step - 1, sr, sg, sb, data);
                set_pixel(cx - step - 1, cy - step + 1, sr, sg, sb, data);
            }
        }
    };

    // Draw main snowflake in center and offset snowflakes around the tile
    draw_snowflake(32, 32, 12, &mut data);
    draw_snowflake(0, 0, 8, &mut data);
    draw_snowflake(64, 0, 8, &mut data);
    draw_snowflake(0, 64, 8, &mut data);
    draw_snowflake(64, 64, 8, &mut data);

    Image::new(
        Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapping_attribute_specification_fulfillment() {
        let mut app = App::new();
        app.add_plugins(WrappingPlugin);

        // Specification requires PaperSized, PaperWrapped, TapedCorners, and DecorativeBow
        let mut required = HashSet::new();
        required.insert(WrappingFeature::PaperSized);
        required.insert(WrappingFeature::PaperWrapped);
        required.insert(WrappingFeature::TapedCorners);
        required.insert(WrappingFeature::DecorativeBow);

        let present_entity = app.world_mut().spawn((
            Present {
                toy_name: "Wooden Train".to_string(),
                ..default()
            },
            WrappingSpecification {
                required_features: required,
                forbidden_features: HashSet::new(),
            },
        )).id();

        // 1. Cut Paper (PaperSized)
        app.world_mut().write_message(UseToolOnPresentMessage::new(
            WrappingTool::Scissors,
            present_entity,
        ));
        app.update();
        let p = app.world().get::<Present>(present_entity).unwrap();
        assert!(p.features.contains(&WrappingFeature::PaperSized));
        assert!(!p.is_completed);

        // 2. Wrap Paper (PaperWrapped)
        app.world_mut().write_message(UseToolOnPresentMessage::new(
            WrappingTool::WrappingPaperRoll,
            present_entity,
        ));
        app.update();
        let p = app.world().get::<Present>(present_entity).unwrap();
        assert!(p.features.contains(&WrappingFeature::PaperWrapped));
        assert!(!p.is_completed);

        // 3. Tape Corners (TapedCorners)
        app.world_mut().write_message(UseToolOnPresentMessage::new(
            WrappingTool::TapeDispenser,
            present_entity,
        ));
        app.update();
        let p = app.world().get::<Present>(present_entity).unwrap();
        assert!(p.features.contains(&WrappingFeature::TapedCorners));
        assert!(!p.is_completed);

        // 4. Attach Decorative Bow (DecorativeBow) -> All required attributes present!
        app.world_mut().write_message(UseToolOnPresentMessage::new(
            WrappingTool::DecorativeBowBox,
            present_entity,
        ));
        app.update();
        let p = app.world().get::<Present>(present_entity).unwrap();
        assert!(p.features.contains(&WrappingFeature::DecorativeBow));
        assert!(p.is_completed, "Present should be completed when all specified attributes are met");
    }

    #[test]
    fn test_forbidden_plasma_instant_wrap() {
        let mut app = App::new();
        app.add_plugins(WrappingPlugin);

        let present_entity = app.world_mut().spawn((
            Present {
                toy_name: "Giant Teddy".to_string(),
                quality_score: 1.0,
                ..default()
            },
            WrappingSpecification::default(),
        )).id();

        // Instantly seal with Forbidden Plasma Shrink
        app.world_mut().write_message(UseToolOnPresentMessage::new(
            WrappingTool::ForbiddenPlasmaShrink,
            present_entity,
        ));
        app.update();

        let p = app.world().get::<Present>(present_entity).unwrap();
        assert!(p.is_completed, "Forbidden plasma wrap fulfills required specifications");
        assert!(p.features.contains(&WrappingFeature::PlasmaShrinkSealed));
        assert_eq!(p.quality_score, 1.5, "Forbidden tool gives instant high quality boost");
    }

    #[test]
    fn test_gift_tag_pen_writing_and_paper_pattern_variants() {
        let mut app = App::new();
        app.add_plugins(WrappingPlugin);

        // 1. Verify PaperPatternRoll cycling
        let mut roll = PaperPatternRoll::default();
        assert_eq!(roll.pattern, PaperPattern::Plain);
        assert_eq!(roll.cycle(), PaperPattern::PolkaDots);
        assert_eq!(roll.cycle(), PaperPattern::Stripes);
        assert_eq!(roll.cycle(), PaperPattern::Snowflakes);
        assert_eq!(roll.cycle(), PaperPattern::Plain);

        // 2. Present requiring PolkaDotPattern and GiftTagAttached
        let mut req = HashSet::new();
        req.insert(WrappingFeature::PaperWrapped);
        req.insert(WrappingFeature::PolkaDotPattern);
        req.insert(WrappingFeature::GiftTagAttached);

        let mut forb = HashSet::new();
        forb.insert(WrappingFeature::StripedPattern);

        let spec = WrappingSpecification {
            required_features: req,
            forbidden_features: forb,
        };

        let present_e = app.world_mut().spawn((
            Present {
                toy_name: "Rocket Ship".to_string(),
                ..default()
            },
            spec.clone(),
        )).id();

        // Wrap with stripes first - should have stripes and not satisfy spec (forbidden feature)
        app.world_mut().write_message(
            UseToolOnPresentMessage::new(WrappingTool::WrappingPaperRoll, present_e)
                .with_pattern(PaperPattern::Stripes),
        );
        app.update();

        let p = app.world().get::<Present>(present_e).unwrap();
        assert!(p.features.contains(&WrappingFeature::PaperWrapped));
        assert!(p.features.contains(&WrappingFeature::StripedPattern));
        assert!(!p.meets_specification(&spec));
        assert!(!p.is_completed);

        // Now re-wrap with polka dots - should replace stripes with polka dots
        app.world_mut().write_message(
            UseToolOnPresentMessage::new(WrappingTool::WrappingPaperRoll, present_e)
                .with_pattern(PaperPattern::PolkaDots),
        );
        app.update();

        let p = app.world().get::<Present>(present_e).unwrap();
        assert!(p.features.contains(&WrappingFeature::PolkaDotPattern));
        assert!(!p.features.contains(&WrappingFeature::StripedPattern));
        assert!(!p.is_completed, "Still missing GiftTagAttached");

        // Write with GiftTagPen
        app.world_mut().write_message(
            UseToolOnPresentMessage::new(WrappingTool::GiftTagPen, present_e)
                .with_inscription("For: Clara - Merry Christmas!"),
        );
        app.update();

        let p = app.world().get::<Present>(present_e).unwrap();
        assert!(p.features.contains(&WrappingFeature::GiftTagAttached));
        assert_eq!(p.recipient_tag.as_deref(), Some("For: Clara - Merry Christmas!"));
        assert!(p.meets_specification(&spec));
        assert!(p.is_completed, "Present should be completed once all requirements including tag and pattern are met");
    }

    #[test]
    fn test_visual_polish_snowflakes_and_pattern_cycling() {
        // 1. Test Snowflake Image Generation
        let snow_img = generate_snowflake_pattern_image();
        assert_eq!(snow_img.texture_descriptor.size.width, 64);
        assert_eq!(snow_img.texture_descriptor.size.height, 64);
        let raw = snow_img.data.as_ref().expect("Snowflake image data must exist");
        assert!(!raw.is_empty());

        // 2. Test Present Wrapping with Snowflake Pattern
        let mut wrap_app = App::new();
        wrap_app.add_message::<UseToolOnPresentMessage>();
        wrap_app.add_systems(Update, handle_present_wrapping);

        let present_e = wrap_app.world_mut().spawn(Present::default()).id();

        // Apply wrapping paper with Snowflake pattern
        {
            let mut msgs = wrap_app.world_mut().resource_mut::<bevy::ecs::message::Messages<UseToolOnPresentMessage>>();
            msgs.write(UseToolOnPresentMessage {
                present_entity: present_e,
                tool: WrappingTool::WrappingPaperRoll,
                pattern: Some(PaperPattern::Snowflakes),
                inscription: None,
            });
        }
        wrap_app.update();

        let present = wrap_app.world().get::<Present>(present_e).unwrap();
        assert!(
            present.features.contains(&WrappingFeature::SnowflakePattern),
            "Present wrapped with Snowflake paper must have SnowflakePattern feature"
        );
        assert!(
            present.features.contains(&WrappingFeature::PaperWrapped),
            "Present wrapped with paper must have PaperWrapped feature"
        );
    }

    #[test]
    fn test_toy_types_dimensions_mass_and_specifications() {
        let train = ToyType::WoodenTrain;
        assert_eq!(train.name(), "Wooden Train");
        assert_eq!(train.dimensions(), Vec3::new(0.65, 0.45, 0.45));
        assert_eq!(train.mass(), 1.2);
        let train_spec = train.default_specification();
        assert!(train_spec.required_features.contains(&WrappingFeature::PaperSized));
        assert!(train_spec.required_features.contains(&WrappingFeature::PaperWrapped));
        assert!(train_spec.required_features.contains(&WrappingFeature::TapedCorners));
        assert!(train_spec.required_features.contains(&WrappingFeature::DecorativeBow));
        assert!(train_spec.required_features.contains(&WrappingFeature::GiftTagAttached));
        assert!(train_spec.forbidden_features.is_empty());

        let bear = ToyType::TeddyBear;
        assert_eq!(bear.name(), "Teddy Bear");
        assert_eq!(bear.dimensions(), Vec3::new(0.50, 0.55, 0.50));
        assert_eq!(bear.mass(), 0.6);
        let bear_spec = bear.default_specification();
        assert!(bear_spec.required_features.contains(&WrappingFeature::PaperWrapped));
        assert!(bear_spec.required_features.contains(&WrappingFeature::SnowflakePattern));
        assert!(bear_spec.required_features.contains(&WrappingFeature::RibbonTied));
        assert!(bear_spec.forbidden_features.contains(&WrappingFeature::PlasmaShrinkSealed));

        let robot = ToyType::TinRobot;
        assert_eq!(robot.name(), "Tin Robot");
        assert_eq!(robot.dimensions(), Vec3::new(0.45, 0.65, 0.40));
        assert_eq!(robot.mass(), 2.0);
        let robot_spec = robot.default_specification();
        assert!(robot_spec.required_features.contains(&WrappingFeature::PaperWrapped));
        assert!(robot_spec.required_features.contains(&WrappingFeature::StripedPattern));
        assert!(robot_spec.required_features.contains(&WrappingFeature::TapedCorners));
        assert!(robot_spec.required_features.contains(&WrappingFeature::TapedCenter));
        assert!(robot_spec.forbidden_features.contains(&WrappingFeature::RibbonTied));
    }

    #[test]
    fn test_toy_wrapping_specification_fulfillment_and_forbidden_violations() {
        let mut app = App::new();
        app.add_plugins(WrappingPlugin);

        // 1. Teddy Bear order: requires PaperWrapped, SnowflakePattern, RibbonTied; forbids PlasmaShrinkSealed
        let bear_spec = WrappingSpecification::for_toy(ToyType::TeddyBear);
        let bear_entity = app.world_mut().spawn((
            Present::new(ToyType::TeddyBear),
            bear_spec.clone(),
        )).id();

        // Wrap paper with Snowflake pattern
        app.world_mut().write_message(
            UseToolOnPresentMessage::new(WrappingTool::WrappingPaperRoll, bear_entity)
                .with_pattern(PaperPattern::Snowflakes),
        );
        app.update();

        let p = app.world().get::<Present>(bear_entity).unwrap();
        assert!(!p.is_completed, "Needs ribbon tied");

        // Tie ribbon
        app.world_mut().write_message(
            UseToolOnPresentMessage::new(WrappingTool::RibbonSpool, bear_entity),
        );
        app.update();

        let p = app.world().get::<Present>(bear_entity).unwrap();
        assert!(p.is_completed, "Teddy bear completed with snowflake paper and ribbon!");
        assert_eq!(p.specification_satisfaction(&bear_spec), 1.0);

        // Violate with forbidden plasma shrink wrap
        app.world_mut().write_message(
            UseToolOnPresentMessage::new(WrappingTool::ForbiddenPlasmaShrink, bear_entity),
        );
        app.update();

        let p = app.world().get::<Present>(bear_entity).unwrap();
        assert!(!p.meets_specification(&bear_spec), "Plasma wrap violates teddy bear forbidden specification!");
        assert!(p.specification_satisfaction(&bear_spec) < 1.0);
    }
}
