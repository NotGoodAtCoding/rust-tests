# Game Design & Automation Plan: Santa's Workshop Elf Simulator

## 1. Overview & Vision
In this 3D first-person / third-person simulation game built with **Bevy 0.20** and **Rapier3D**, the player embodies one of Santa's elves during peak Christmas crunch time. The player must efficiently wrap toys, maintain order in the workshop, and survive escalating hazards—most notably **Mr. Moose**, an ominous unshaded brown plastic moose figure stationed outside who demands frequent snacks or will violently smash through the workshop window.

---

## 2. Core Gameplay Principles

1. **Diegetic & In-World Communication**:
   * All player instructions, feedback, alerts, and quotas exist entirely within the physical game world.
   * Quota counters, machine readouts, and tutorial prompts are rendered on chunky CRT monitors, chalkboards, wall clipboards, illuminated gauges, and wooden warning signs mounted in the workshop—avoiding intrusive non-diegetic floating screen overlays.
   * Mr. Moose's hunger status is visually represented by a physical mechanical dial outside the window, ticking lights, or a physical scale under his feeding trough.
2. **Physics Sandbox, Soft-Body & Chaotic Dynamics**:
   * Prioritize goofy, slapstick, high-energy physics: squishy soft-body presents (spring-mass / joint lattices), elastic ribbons that snap and yank objects, ragdoll physics for stumbling elf assistants, and exaggerated impact forces.
   * Tools and machines can malfunction with extreme impulse forces (e.g. pneumatic ribbon cannons launching items across the room, bouncy rubber toys rebounding violently off walls).
   * Players are rewarded and encouraged to experiment with chaotic physics setups, dynamic stacking, and conveyor catapults.
3. **Player-Driven Agency & Emergent Gameplay**:
   * While Santa's quota provides an overarching direction, how tasks are accomplished is left entirely to the player's discretion.
   * Multiple viable pathways: carefully follow textbook wrapping procedures, construct Rube Goldberg assembly lines using conveyor belts and physics ramps, or risk dangerous back-room overclocks.
   * The player decides when to bribe, distract, or barricade against Mr. Moose, or even deliberately trigger chaos events to bypass obstacles.

---

## 3. Core Mechanics

### 3.1 Present Wrapping System (Attribute & Specification Driven)
* **Attribute-Based Architecture**:
  * Instead of a rigid linear enum stage (`RawToy -> PaperCut -> Taped -> ...`), presents are defined by an open **set of attributes / features** (`HashSet<WrappingFeature>`):
    * `PaperSized`: Cut to proper dimensions.
    * `PaperWrapped`: Wrapped in wrapping paper.
    * `TapedCorners`, `TapedCenter`: Tape applied at specific structural points.
    * `RibbonTied`, `DecorativeBow`: Decorative accents.
    * `GiftTagAttached`: Name tag labeled.
    * `PolkaDotPattern`, `StripedPattern`: Specific paper designs.
    * `PlasmaShrinkSealed`: Sealed with high-tech forbidden tools.
* **Instructions & Specifications (`WrappingSpecification`)**:
  * Each incoming toy arrives with an order instruction (or monitor readout in the workshop) specifying:
    * `required_features: HashSet<WrappingFeature>`
    * `forbidden_features: HashSet<WrappingFeature>` (e.g. "Do not use ribbons on this box", "Must not use plasma shrinker").
  * Completion is validated dynamically via `present.meets_specification(&spec)`.
* **Materials & Stations**:
  * **Unwrapped Toy Spawner / Conveyor**: Delivers assorted raw toys (wooden trains, robots, teddy bears) onto a workbench.
  * **Wrapping Paper Rolls**: Dispenses sized/patterned sheets.
  * **Tools**:
    * **Scissors**: Sizes paper or cuts ribbons.
    * **Tape Dispenser**: Secures flaps/corners.
    * **Bow Box & Ribbons**: Applies decorative accents.
    * **Gift Tag Pen**: Writes recipient tag.
    * **Forbidden Tools**: Applies volatile instant seals.

### 2.2 Evolving Scene & Workshop Progression
* **Progression Tiers**:
  * **Tier 1 (Main Workshop)**: Basic manual workbench, paper rack, tape dispenser, delivery chute, Mr. Moose snack feeder.
  * **Tier 2 (Storage Wing Unlocked)**: Reached after $N$ successful presents. Unlocks bulk tape dispensers, motorized wrapping turntables, and elf assistants (NPCs).
  * **Tier 3 (The Forbidden Back Workshop)**:
    * Hidden behind a locked reinforced door.
    * Contains high-tech / occult machinery: "Instant Plasma Gift Shrink-Wrapper", "Hydro-pneumatic Ribbon Cannon", "Cursed Candy-Cane Overclocker".
    * **Risk vs Reward**: Wraps gifts in 1.5 seconds, but has volatility timers (heat gauges, misfire risks, explosive paper jams) that damage the workshop or summon chaotic hazards if not managed.
* **Dynamic Environment & Events**:
  * Trigger-based barriers opening new doors via Rapier kinematic rigidbodies or sensor volumes.
  * NPC Elves roaming and delivering supplies or panicking during catastrophic events.

### 2.3 Mr. Moose Mechanics
* **Visual Style**: Completely unshaded brown plastic material (`StandardMaterial { base_color: Color::srgb(0.35, 0.20, 0.10), unlit: true, ..default() }`), rigid toy geometry, low-poly or stylized antlers.
* **Hunger & Feeding Loop**:
  * Mr. Moose sits outside on a snowy hill, visible through a large glass window pane with a dedicated feeding chute / trough.
  * **Moose Hunger Meter**: Slowly depletes over time (e.g. 90-second cycle).
  * **Moose Snacks**: Spawn in the pantry; player must physically carry a snack canister and drop it into the feeding bowl.
* **Breach Event (Window Destruction)**:
  * If the hunger meter reaches zero:
    * Alarm sound / rumbling screen shake.
    * Mr. Moose leaps through the window, breaking glass (procedural fracture / physics shards).
    * Rampages through the workshop, knocking over wrapping tables, scattering presents, and causing an immediate emergency lockdown until lured back with a mega-snack.

### 2.5 Dual-Hand Player Control & Physics Interaction
* **Dual-Hand Input Architecture**:
  * The player has two independent physical hands:
    * **Left Hand**: Controlled by `KeyQ` (hold to grab, release to drop).
    * **Right Hand**: Controlled by `KeyE` (hold to grab, release to drop).
  * Raycast / sphere-cast detection at each hand's position determines nearby grabbable objects, physics handles, or tools.
* **Two-Handed Physical Gestures & Interactions**:
  * **Violent Ribbon Shaking (Bow Creation)**:
    * Grabbing the same ribbon with both hands (`Q` + `E`) enters a tension/knotting state.
    * Rapid mouse shaking or alternating hand movement generates velocity variance ($\Delta v > v_{\text{threshold}}$). Reaching the threshold instantly ties the ribbon into a ready-to-place `DecorativeBow`.
  * **Tape Pulling & Custom Lengths**:
    * Hold tape dispenser with one hand (`Q` or `E`).
    * Press and hold grab with the free opposite hand on the tape dispenser's exposed tip to start pulling a tape strip.
    * The longer the grab key is held, the longer the tape strip pulls out (e.g., $5\,\text{cm}$ to a maximum of $40\,\text{cm}$).
    * Taping too long makes the tape strip crinkle, tangle, or fold over onto itself (chaotic physics failure), requiring precision. Releasing or snapping with scissors cuts the strand for placement.
  * **Dual-Hand Object Carrying**:
    * Bulky items (giant toy boxes, heavy snack tubs, Mr. Moose snack canisters) require both hands (`Q` and `E`) to lift stably; single-hand grabbing causes items to drag, tilt, or slip out under physics gravity.

### 2.6 Player Movement Physics: Air Control & Bunny-Hopping (BHOP)
* **Airborne Physics & Air-Straining**:
  * Unlike ground movement where instantaneous or high-friction direction changes occur, airborne movement restricts direct velocity setting.
  * Ground movement has full control and deceleration; once airborne, ground friction drops to zero.
  * **Air Straining**: Holding movement keys in the air exerts a gentle steering force/acceleration ($\approx 10\%-15\%$ of ground acceleration) rather than replacing current momentum, allowing slight arc adjustments without sharp mid-air redirection.
* **Bunny-Hopping Mechanic**:
  * **Timing Window**: When landing on the ground, the player must press jump (`just_pressed(KeyCode::Space)`) within a tight tolerance window of hitting the surface.
  * **Single-Press Rule**: Holding the space bar (`pressed(KeyCode::Space)`) will **never** trigger a bunny hop—the jump input must be fresh on impact.
  * **Speed Scaling**:
    * Consecutive successful bunny hops build a combo streak (`consecutive_bhops`).
    * After 3+ consecutive successful hops, the maximum horizontal speed cap increases up to $\sqrt{2} \approx 1.414\times$ the base movement speed.
    * Failing a hop (landing without jumping in time, jumping too early, or touching ground friction for longer than the window) resets the combo streak and smoothly decays velocity back to base cap.
* **Debug Menu Options**:
  * Located under the Options / Debug menu (toggled via `Tab`):
  * **BHOP Timing Window Setting**: Cycleable / selectable options:
    * `1/60 sec` ($\approx 16.6\,\text{ms}$, hard / competitive precision)
    * `1/30 sec` ($\approx 33.3\,\text{ms}$, medium)
    * `1/10 sec` ($\approx 100\,\text{ms}$, forgiving / debug)

---

## 4. Architecture & Modular Structure

To ensure clean decoupling, maintainability, and rapid automated testing, the codebase will be partitioned into clean Bevy plugins:

```
src/
├── elf_game/
│   ├── mod.rs               # ElfGamePlugin orchestrating sub-plugins
│   ├── core/                # Core states (GameState, Pause, Quota)
│   │   ├── mod.rs
│   │   └── state.rs
│   ├── player/              # Player movement, inventory, hand holding, tool raycasts, BHOP
│   │   ├── mod.rs
│   │   ├── controller.rs
│   │   └── inventory.rs
│   ├── wrapping/            # Present entity state machine, tool mechanics
│   │   ├── mod.rs
│   │   ├── present.rs
│   │   ├── tools.rs
│   │   └── scoring.rs
│   ├── physics_sandbox/     # Ragdolls, spring-mass soft bodies, impulse forces
│   │   ├── mod.rs
│   │   ├── soft_body.rs
│   │   └── ragdoll.rs
│   ├── workshop/            # Scene layout, doors, forbidden room, progression triggers
│   │   ├── mod.rs
│   │   ├── room.rs
│   │   └── progression.rs
│   ├── moose/               # Mr. Moose hunger timer, window breach, physics rampage
│   │   ├── mod.rs
│   │   ├── hunger.rs
│   │   └── breach.rs
│   ├── assets/              # Procedural mesh generation, shaders, AI asset loader
│   │   └── mod.rs
│   └── diegetic_ui/         # In-world CRT monitors, signs, meters, clipboard displays
│       └── mod.rs
└── tests/                   # Integration and headless Bevy simulation tests
```

---

## 5. Testing & Verification Strategy

Heavy emphasis on unit tests to enable fast, zero-manual-friction testing:
1. **Headless Bevy App Unit Tests**:
   * Wrap gameplay logic inside small headless `App::new()` instances without rendering or window plugins.
   * `test_wrapping_state_progression()`: Verifies that applying Paper -> Tape -> Ribbon in sequence transitions state from `Unwrapped` to `Completed`.
   * `test_moose_hunger_depletion_and_snack_reset()`: Steps simulated time by 60s, verifies hunger decreases, simulates snack deposit, checks meter restored.
   * `test_progression_unlock_conditions()`: Delivers target quota of presents, asserts forbidden door entity receives unlock event.
   * `test_forbidden_tools_volatility()`: Overheats plasma wrapper, verifies failure / explosion event fires.
   * `test_soft_body_spring_dynamics()`: Simulates impulse force on soft-body lattice, verifying restorative spring oscillations and bounding sanity.
2. **Deterministic Physics Tests**:
   * Rapier step tests for present placement on tables and snack chute gravity detection.
3. **Execution**:
   * `cargo test` runs all headless tests in under 2 seconds.

---

## 6. Asset Generation Strategy (Procedural & AI-Assisted)

* **Phased Asset Pipeline**:
  * **Phase 1 (Procedural Primitives & Math Meshes)**:
    * Mr. Moose constructed as a stylized composite mesh (capsule body, cylinder legs, custom planar antlers) rendered with an unshaded matte brown material.
    * Workshop built from modular extruded walls, windows with breakable segment grid, and festive procedural textures (checkerboard linoleum, wood planks, candy cane stripes).
    * Wrapping paper with procedural wrapping patterns (stripes, polka dots, snowflakes generated via compute shaders or CPU image buffers).
  * **Phase 2 (AI-Assisted Assets & glTF Generation)**:
    * Use AI image/mesh generator pipelines (e.g. text-to-3D glTF models, local Blender script generation via Python, AI-generated texture maps for wrapping paper and Christmas posters).
    * Fallback to lightweight GLTF/GLB formats loaded dynamically into Bevy's `AssetServer`.

---

## 7. Automation Routine & Quota Management (AFK Agent Workflow)

To keep the project continuously moving forward without exceeding your 5-hour quota windows while you are AFK:

### 7.1 Architecture of the Automation
* **Autonomous Milestone Engine**:
  * A scheduled sidecar automation or multi-step goal plan that executes iterative sub-tasks:
    1. **Pull & Status Check**: Inspect Git log and test suite.
    2. **Pick Next Milestone Task from Roadmap**: Focus on a single bite-sized feature (e.g., "Implement Moose Hunger UI meter" or "Add scissors cutting logic").
    3. **Implementation & Refactoring**: Write Rust code and components.
    4. **Unit Test Coverage**: Write headless Bevy tests exercising the new system.
    5. **Verification**: Run `cargo check` and `cargo test`.
    6. **Status Artifact Update**: Update `PROJECT_STATUS.md` with progress, open issues, and next tasks.
    7. **Commit & Checkpoint**: Git commit with clear semantic message.
* **Quota & Rate-Limiting Guardrails**:
  * **Cadence**: Run once every 3 to 4 hours (or at scheduled intervals), strictly limiting each session to a focused single-feature cycle (approx. 5-10 agent turns).
  * **Exit Early on Green**: Once the single milestone builds cleanly and all tests pass, the agent commits and shuts down, saving budget for the next cycle.
  * **Fail-Safe / Self-Recovery**: If `cargo test` fails after 2 self-repair attempts, the agent stashes or reverts changes, logs the failure in `PROJECT_STATUS.md`, and alerts the user rather than burning quota in infinite retry loops.

---

## 8. Iterative Roadmap

| Milestone | Scope | Deliverables |
|---|---|---|
| **M1: Core Game Loop & Framework** | Architecture & Present State Machine | App scaffolding, Present wrapping state machine, headless unit tests. |
| **M2: Advanced Movement & BHOP** | Air straining, Bunny-hopping & Debug Menu | Realistic air momentum, jump timing window (1/60, 1/30, 1/10s), 1.41x speed scaling, headless physics unit tests. |
| **M3: Mr. Moose & Window Breach** | Moose Hunger & Physics Rampage | Hunger timer, feeding chute, unshaded toy moose model, window glass shatter event. |
| **M4: Chaotic Physics & Soft-Body Sandbox** | Soft bodies, ragdolls & extreme forces | Spring-mass squishy presents, bouncy toys, ragdoll elves, chaotic impulse machines. |
| **M5: In-World Diegetic Displays & Player Agency** | In-world monitors, signs & player choices | In-world CRT monitors, warning signs, physical scales, alternate wrapping pipelines. |
| **M6: Progression & The Forbidden Workshop** | Multi-tier workshop expansion | Door unlocks, quota tracker, forbidden overpowered volatile tools, NPC elves. |
| **M7: Visual Polish & Custom Assets** | Audio, AI textures & procedural models | AI/procedural festive textures, particles (wrapping paper confetti, glass shards), sound effects. |
