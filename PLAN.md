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
4. **Visual Style (Minimal & Impressionist)**:
   * Models and environment assets should be **minimal and impressionist**. They should not have extraneous detail.
   * Visuals are defined by bold geometric silhouettes, distinctive primary masses, and clean readable palettes rather than fine-grained surface noise or micro-textures.
   * Extraneous detail clutter impairs diegetic legibility during high-speed bunnyhopping and slapstick physics chaos; simplified impressionist forms maximize clarity, maintain peak physics simulation rates, and evoke a charming, chunky toy-workshop holiday feel.

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

### 2.2 Main Room Layout, Cabin Architecture & Pneumatics
* **Cabin Atmosphere & Architectural Layout**:
  * **Rustic Alpine Cabin Aesthetic**:
    * Warm timber plank and log wall construction replacing flat monochromatic walls, featuring rich cedar/pine wood tones (`perceptual_roughness: 0.85`), dark wood corner posts, and stone/brick floor trim.
  * **High Vaulted Ceiling & Structural Cross Beams**:
    * Roof geometry pitched into a high A-frame vault rising from $5.0\,\text{m}$ side walls to an $8.5\,\text{m}$ central ridge.
    * Heavy exposed timber cross-beams (tie beams and king post trusses) span across the room width at $Y = 5.2\,\text{m}$, creating structural anchor points for suspended lighting and pneumatic pipework.
  * **Dynamic Candle Chandeliers**:
    * Two large rustic wrought-iron chandeliers suspended by chains from the wooden cross-beams:
      * **North Chandelier**: Positioned at $(0.0, 5.2, -4.5)$.
      * **South Chandelier**: Positioned at $(0.0, 5.2, 4.5)$.
    * Each chandelier mounts a ring of 6–8 tallow candles with warm point light emitters (`Color::srgb(1.0, 0.76, 0.42)`, $\approx 2200\,\text{K}$).
    * Procedural flame flicker system (`intensity = base + A * sin(omega_1 * t) + B * sin(omega_2 * t)`) casting dancing dynamic shadows across the log walls, wooden cross beams, and workshop floor.

* **Conveyor Belt & Recessed Despawn Chute**:
  * Stretches along the room from $X = -8.5\,\text{m}$ to $X = 9.5\,\text{m}$ at $Z = 3.0\,\text{m}$, elevated at $Y = 0.75\,\text{m}$.
  * Carries raw toy cubes continuously from the spawner toward the dispatch end at $2.0\,\text{m/s}$.
  * **Floor-Recessed Collection Chute**:
    * Located directly below the discharge end of the conveyor belt at $(9.6, 0.0, 3.0)$.
    * Features a raised, beveled funnel lip/rim ($0.15\,\text{m}$ above floor) that catches falling presents as they tumble off the edge of the belt under gravity.
    * The chute shaft drops down into a recessed sub-floor collector pit ($Y \in [-1.5, 0.0]\,\text{m}$).
    * **No-Pop Sub-Floor Despawn Zone**: The despawn sensor volume is positioned at $Y \le -0.8\,\text{m}$ deep inside the recessed shaft. Presents completely disappear from view into the dark chute before despawning, eliminating visible popping.

* **Present Sending Platform & Pneumatic Vacuum System**:
  * **Present Sending Platform**:
    * A dedicated circular/hexagonal dispatch platform located adjacent to the main workbench table (e.g. at $X = 5.0, Z = -4.0$).
    * Features clearly marked hazard stripes and present alignment guides where the player deposits completed, wrapped, and inspected gifts.
    * Physical interactive **Send Button** (large red industrial plunger/button on a control pedestal beside the platform).
  * **Suction Cone & Overhead Funnel**:
    * A circular intake funnel positioned overhead at $Y = 4.0\,\text{m}$ directly above the platform center.
    * When the player presses the Send Button, a pneumatic turbine spools up with an audible acoustic rev and activates an upward conical suction field:
      $$\vec{F}_{\text{suction}}(\vec{x}) = k_{\text{lift}} \frac{Y_{\text{funnel}} - y}{H} \hat{j} + k_{\text{inward}} \frac{\vec{x}_{\text{center}} - \vec{x}_{\text{h}}}{\|\vec{x}_{\text{center}} - \vec{x}_{\text{h}}\|}$$
      where items located within the cone of opening angle $\alpha \approx 30^\circ$ below the funnel are pulled upward and focused inward toward the mouth.
  * **Physics Soft-Body Transport Tube**:
    * Continuous translucent/metallic pneumatic transport tube extending from the intake funnel ($Y = 4.0\,\text{m}$), routing up through the rafters, twisting through sweeping curved bends and a dramatic $360^\circ$ loop across the ceiling vault before terminating at the Santa Sorting Depot.
    * Modeled as a physics spline / soft-body lattice chain: each segment has radial and axial spring stiffness and damping.
  * **Dynamic Travelling "Lump" & Reaction Impulses**:
    * Upon entering the intake funnel, the physics gift is ingested and converted into an active pneumatic transit packet characterized by its physical properties:
      * Mass $m$ (from gift weight)
      * Geometric radius $r_{\text{pack}}$ (from gift bounds)
      * Transit velocity $v(s)$ along the 3D spline arc length $s$
    * **Visual Lump Deformation**: As the packet travels through the tube at high pneumatic speed ($12\text{--}18\,\text{m/s}$), adjacent tube vertices undergo local radial dilation (bulging outwards by $\Delta R \propto r_{\text{pack}}$), visually revealing the packet moving along the tube.
    * **Centrifugal & Directional Reaction Impulses**:
      * As the lump speeds through curves and the loop (with local curvature $\kappa = \frac{1}{R}$ and principal normal $\hat{N}$), it exerts an outward centrifugal force:
        $$\vec{F}_{\text{centrifugal}} = \frac{m v^2}{R} \hat{N}$$
      * Along bends and tangential transitions, the momentum change exerts reaction impulses directly onto the tube segments:
        $$\vec{J}_{\text{tube}} = -m \Delta \vec{v}$$
      * Heavy, bulky gifts (large trains, robots) cause the tube to violently jerk, sway, and oscillate on its rafters mounts, rattling hanging lamps and nearby workshop props in satisfying slapstick fashion!

* **Observation Window & Shutters**:
  * Positioned off-center on the North wall ($X = -6.5, Z = -13.0$) overlooking the snowy hill.
  * Head-sized opening ($1.8\,\text{m} \times 1.8\,\text{m}$) fitted with dual hinged wooden shutters that swing inward into the workshop as Mr. Moose pokes his head inside.

* **Pantry Storage Wing & Self-Restocking Physics Door**:
  * **Room Footprint & Shelving**: Generously sized storage room ($4.0\,\text{m} \times 4.0\,\text{m} \times 3.5\,\text{m}$) situated off the main workshop corridor, fitted with deep timber perimeter shelving units. The expansive floorplan provides plenty of capacity for current inventory as well as upcoming systems (such as cookie baking ingredients: flour sacks, sugar bags, butter blocks, and baking pans).
  * **Interactive Physics Door**: Mounted on a Rapier `RevoluteJoint` / spring hinge that physically swings open and closed with momentum, capable of being pushed, pulled, or knocked by players and ragdoll elves.
  * **Infinite Restock Mechanic**: Houses a never-ending supply of Moose Treat jars. Monitored via a door-closure sensor cycle: whenever the player completely closes the pantry door and reopens it (`Closed -> Opened` state transition), any depleted jar slots on the pantry shelves automatically restock with fresh Moose Treat jars.

* **Progression Philosophy: Task Management Escalation**:
  * **Core Gameplay Loop**: The core of Santa's Workshop is high-stakes physical **task management**. As the game progresses, tasks do not merely repeat; they become more numerous and build upon previous tasks, steadily compounding in systemic complexity:
    * Stage 1: Solo wrapping and feeding manual conveyor.
    * Stage 2: Packaging quota fulfillment + maintaining Mr. Moose's hunger threshold.
    * Stage 3: Delegating jobs to unpredictable, ragdoll-prone elf helpers, managing pantry restocking, and juggling simultaneous production and kitchen/feeding loops.
  * **Confounding Physics & Unreliable Helpers**: Crucially, completing these tasks is complicated and confounded by slapstick ragdoll physics, spring-mass soft bodies, dynamic momentum, and erratic, unreliable elf helpers with mischievous traits (lazy elves slacking, klepto elves hoarding presents, clumsy drops, and accidental physics collisions).
  * **Mr. Moose Initial Dormancy & 1st Present Activation**:
    * At game start, Mr. Moose is dormant behind the closed North window shutters. The workshop begins peacefully, allowing the player to learn movement, controls, and wrapping mechanics without time pressure.
    * Only after the player successfully wraps and submits their **first present** through the pneumatic sending vacuum platform does Mr. Moose awaken, initiating his hunger timer (+1/s), gauge ticks, and the recurring feeding loop.
  * **Easter Egg Elf Unlocks (Chandelier Sparky)**:
    * Autonomous helper elves are not all active at the start; they are discovered and unlocked as environmental easter eggs throughout the workshop.
    * **"Elf Sparky"**: Initially found inactive, perched fast asleep atop the high North or South dynamic candle chandelier ($Y = 5.2\,\text{m}$).
    * Only when the player strikes the chandelier—via a thrown present, launched tool, or high-speed aerial BHOP collision—with sufficient impact impulse to set it swinging, does Sparky get knocked loose, ragdoll-tumbles to the floor, and awakens into the roaming elf pool.

* **Progression Tiers**:
  * **Tier 1 (Main Workshop)**: Basic manual workbench, conveyor belt with raw cube spawner, paper rack, tape dispenser, delivery chute, Mr. Moose snack feeder.
  * **Tier 2 (Storage Wing Unlocked)**: Reached after $N$ successful presents. Unlocks bulk tape dispensers, motorized wrapping turntables, and elf assistants (NPCs).
  * **Tier 3 (The Forbidden Back Workshop)**:
    * Hidden behind a locked reinforced door.
    * Contains high-tech / occult machinery: "Instant Plasma Gift Shrink-Wrapper", "Hydro-pneumatic Ribbon Cannon", "Cursed Candy-Cane Overclocker".
    * **Risk vs Reward**: Wraps gifts in 1.5 seconds, but has volatility timers (heat gauges, misfire risks, explosive paper jams) that damage the workshop or summon chaotic hazards if not managed.
* **Dynamic Environment & Events**:
  * Trigger-based barriers opening new doors via Rapier kinematic rigidbodies or sensor volumes.
  * NPC Elves roaming and delivering supplies or panicking during catastrophic events.

### 2.3 Mr. Moose Mechanics (Stages 0–60 & Rampage Path)
* **Visual Style**: Completely unshaded brown plastic material (`StandardMaterial { base_color: Color::srgb(0.35, 0.20, 0.10), unlit: true, ..default() }`), rigid toy geometry, low-poly or stylized antlers.
* **Hunger Value Progression (0 to 60, +1/sec default)**:
  * **Stage 1 (Hunger 0–20, Calm)**:
    * Mr. Moose is resting behind closed window shutters outside. The workshop is calm.
  * **Stage 2 (Hunger 20–50, Peeking)**:
    * Shutters slide open. Mr. Moose slowly pokes his plastic head through the window into the workshop, tracking inward as hunger increases.
  * **Stage 3 (Hunger 50–60, Agitated Huffing)**:
    * Mr. Moose stops advancing and waits with his head inside the room. Emits ominous audible huffs, grunts, or mechanical moose clicks alerting the player.
  * **Stage 4 (Hunger = 60, Slam & Impending Breach)**:
    * At hunger 60, Mr. Moose violently pulls back and slams the window shut with a loud acoustic thud.
    * An ominous countdown timer triggers with a randomized delay of **3 to 12 seconds**.
  * **Stage 5 (Breach & Rampage Along Set Path)**:
    * After the 3–12s delay expires, Mr. Moose smashes through the glass into the workshop!
    * Follows a deterministic waypoint path through the workshop (e.g. charging past the workbench, crashing into the conveyor belt, knocking over storage crates).
    * **Extreme Impulses**: Any physics objects (presents, tools, snack cans, elf ragdolls) interacting with Mr. Moose receive exaggerated impact impulse forces ($3\times\text{--}5\times$ normal collision impulses), sending them flying across the room in slapstick fashion.
* **Feeding & Reset Loop**:
  * Depositing Moose Snacks into the feeding trough resets the hunger timer backwards, causing Mr. Moose to retreat back behind the window shutters.

### 2.5 Dual-Hand Player Control & Physics Interaction
* **Dual-Hand Input Architecture**:
  * The player has two independent physical hands:
    * **Left Hand**: Controlled by `KeyQ` (toggle to grab or set down).
    * **Right Hand**: Controlled by `KeyE` (toggle to grab or set down).
  * Raycast / proximity detection at each hand's position determines nearby grabbable objects, physics handles, or tools.
  * **Toggle Grab & Gentle Set Down**:
    * Pressing `KeyQ` or `KeyE` (`just_pressed`) toggles grab state.
    * If a hand is empty, it grabs the nearest grabbable object within reach ($1.8\,\text{m}$).
    * If already holding an object, pressing the same grab key again releases the object gently at its current hand position with zero initial launch speed, setting it down stably on a workbench, sill, or floor.
  * **Mouse 1 Throwing**:
    * Clicking **Mouse 1** (`MouseButton::Left`) throws the currently held object forward along the camera's sightline with a lively upward arc:
      $$\vec{v}_{\text{throw}} = 10.0 \cdot \frac{\vec{u}_{\text{camera}} + 0.15 \hat{j}}{\|\vec{u}_{\text{camera}} + 0.15 \hat{j}\|} + 0.5 \vec{v}_{\text{player}}$$
    * Imparts randomized angular tumble velocity ($\vec{\omega} \sim \mathcal{U}(-3, 3)\,\text{rad/s}$) so thrown presents and tools spin and tumble satisfyingly in flight.
    * Throws the Right Hand item first; if empty, throws the Left Hand item.
* **Floaty Hands & Physics Momentum**:
  * Hands are modeled with dynamic physical inertia and a damped spring follow model in viewmodel space:
    $$\vec{F}_{\text{spring}} = -k_{\text{spring}} (\vec{x} - \vec{x}_{\text{rest}})$$
    $$\vec{F}_{\text{damping}} = -c_{\text{damping}} \vec{v}_{\text{hand}}$$
    $$\vec{F}_{\text{inertia}} = -m_{\text{hand}} (\vec{a}_{\text{camera\_sway}} + \vec{a}_{\text{player\_accel}})$$
  * When turning the camera quickly, hands sway and lag slightly behind the view direction before smoothly catching up.
  * Walking, jumping, and landing introduce vertical and horizontal inertial displacement (compression on landing, dipping on jump).
  * Breathing and idle micro-motions provide a floaty, organic, alive feel to the gloves.
* **2D Sprite Viewmodel Hands & Grab Poses**:
  * Replace the primitive 3D sphere gloves with expressive 2D sprite viewmodel hands rendered on a dedicated viewmodel/UI canvas layer or camera-anchored billboarding quad mesh.
  * State machine driving sprite textures and poses based on player locomotion and grab state:
    * **`Idle`**: Relaxed, slightly open glove resting in low-ready pose with gentle breathing micro-bobbing.
    * **`Walking`**: Rhythmic procedural sway and alternating stride bobbing synced to footstep timing.
    * **`Sprinting`** (or bunny-hop acceleration): Dynamic high-speed pumping / air-straining pose with angled forward lean and energetic stride motion.
    * **`Grabbing`**: Reaching / anticipation pinch pose triggered when within interaction proximity of an item or holding grab key before latching.
    * **`Holding`**: Clamped, firm grip pose wrapped around the held object (present, tool, tape dispenser, snack jar), locking fingers into a grasping clasp.
  * Both hands operate independently: Left Hand can be `Holding` while Right Hand is `Grabbing` or `Idle`.
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
* **Debug Menu Options & Dedicated DEBUG Submenu**:
  * Accessible via the in-game pause modal (`Tab`, `Escape`, `P`, or `Pause`), featuring dedicated `OPTIONS` and `DEBUG` tabs and navigation buttons:
  * **1. BHOP Timing Window Setting**: Cycleable / selectable timing windows:
    * `1/60 sec` ($\approx 16.6\,\text{ms}$, hard / competitive precision)
    * `1/30 sec` ($\approx 33.3\,\text{ms}$, medium)
    * `1/10 sec` ($\approx 100\,\text{ms}$, forgiving / debug)
  * **2. Noclip Toggle (Free 3D Flying)**:
    * Bypasses character controller collision resolution and gravity (`velocity = 0`).
    * Inserts Rapier `ColliderDisabled` on the player entity, allowing free traversal through any wall, ceiling, table, conveyor, or out into the exterior snowy rafters.
    * 3D locomotion along sightline: `W`/`S` along look direction (including pitch angle), `A`/`D` strafe, `Space` ascend (+Y), `Left Ctrl` / `C` descend (-Y), and `Shift` fast boost ($2\times$, $24\,\text{m/s}$).
    * Can also be toggled directly in-game via keyboard shortcut `[V]`.
  * **3. Wireframe Toggle**:
    * Toggles global wireframe rendering mode across the workshop.
    * Synchronizes with both Rapier `DebugRenderContext.enabled` (physics colliders, sensors, joints) and Bevy PBR `WireframeConfig.global` (3D mesh polygon wireframes).

### 2.7 Autonomous NPC Elf Assistants: Navigation Graph, Mischief Traits & Job Hats
* **Topological Navigation Graph & Clearance Paths (`ElfNavGraph`)**:
  * 15-node topological pathfinding graph (`ElfNavGraph`) that routes elves smoothly through workshop aisles while strictly providing geometric clearance around obstacles:
    * Central Workbench ($X \in [-2.0, 2.0], Z \in [-4.9, -3.1]$)
    * Main Conveyor Belt ($X \in [-8.5, 9.5], Z = 3.0$)
    * Recessed Despawner & Vacuum Platform
  * **Shortest Path Planning & Free Movement**:
    * Breadth-first search / Dijkstra shortest path calculation (`find_path(start, goal) -> Option<Vec<usize>>`).
    * When arriving at a node, elves can move freely within that node's designated radius (`radius: 0.8--1.2m`) before selecting a subsequent destination node.
  * **Diegetic Debug Visualization**:
    * Visible via Bevy `Gizmos` showing node center spheres, floor free-movement radius circles, and active edge connecting lines when wireframes/debug mode is enabled.
* **Impressionist Grounded Visuals & Translation**:
  * Capsule physics collider: `Collider::capsule_y(0.25, 0.20)` with base at local $Y = -0.45\,\text{m}$.
  * Child articulated limb rest translations shifted down by $0.48\,\text{m}$ (Legs: $Y = -0.32\,\text{m}$, Torso: $Y = -0.10\,\text{m}$, Belt: $Y = -0.20\,\text{m}$, Head: $Y = 0.20\,\text{m}$, Hat: $Y = 0.32\,\text{m}$).
  * Boot soles align directly with collider bottom ($-0.45\,\text{m}$), ensuring elves walk flush with the floor planks at $Y = 0.0\,\text{m}$ when standing at $Y = 0.45\,\text{m}$.
* **Mischievous Behavioral Traits (`ElfMischiefTrait`)**:
  * Elves spawn with distinct personality traits:
    * **Lazy (`ElfMischiefTrait::Lazy`, e.g. "Elf Sparky")**:
      * Periodically enters `ElfNpcState::Slacking` with slouching posture, drooping head, and limp arms.
      * Refuses to perform work or follow patrol routes.
      * Remains stubborn and motionless until knocked down into `ElfNpcState::RagdollTumble` by a dynamic physics object impact (thrown presents, sliding tools) or high-speed player BHOP tackle.
      * Upon recovering balance, gets back to work with a refreshed slacking cooldown.
    * **Klepto (`ElfMischiefTrait::Klepto`, e.g. "Elf Tinsel")**:
      * Scours the workshop for unwrapped or wrapped presents.
      * Snatches available presents (`carried_present`), transitions into `ElfNpcState::StashingPresent`, and navigates along the graph to the secluded Stash Node (Node 13 at $(-10.5, 0.45, 10.5)$) to hide them.
    * **Pyro (`ElfMischiefTrait::Pyro`, e.g. "Elf Jingles")**:
      * Tagged for future fiery expansion (will seek heat/spark sources to ignite flammable workshop items; currently passive/pathfinding).
* **Hat Closet & Job Assignment (`ElfHatCloset`, `ElfJobHat`, `ElfJob`)**:
  * **Diegetic Hat Closet**:
    * Impressionist pine/cedar wardrobe located on the back wall ($X = -4.5, Y = 1.3, Z = 12.3$) with a countertop hat shelf and sign plaque.
    * Interacting with the closet (<kbd>E</kbd>/<kbd>Q</kbd>/<kbd>F</kbd>/<kbd>Left Click</kbd>) dispenses three dynamic job hats simultaneously:
      1. `Moose Feeder` Hat (hunter brown cap with accent band)
      2. `Present Wrapper` Hat (festive red cone with gold ribbon ring)
      3. `Workshop Maintenance` Hat (industrial blue cap with sheet metal visor)
  * **Dynamic Interaction & Equipping**:
    * Hats are dynamic `Grabbable` physics objects that the player can pick up in either hand.
    * Approaching an elf while holding a hat and pressing interact assigns that hat to the elf:
      * Removes hat from player's grasp.
      * Disables hat dynamic physics and parents it to the elf root at head position ($Y = 0.32\,\text{m}$).
      * Updates `elf.assigned_job = Some(job)`.

* **Identifiable Navigation Nodes & Routine Job Paths (`ElfNodePurpose`, `ElfJob::MooseFeeder`)**:
  * **Semantic Node Roles (`ElfNodePurpose`)**:
    * Graph waypoints in `ElfNavGraph` carry semantic identifiers so elves can navigate contextually to perform assigned work:
      * `ElfNodePurpose::Workbench`: Central present assembly station.
      * `ElfNodePurpose::MainTableTreatJar`: Main workbench snack jar area.
      * `ElfNodePurpose::MooseFeedingStation`: Trough active feeding pad outside observation window.
      * `ElfNodePurpose::PantryShelves`: Deep pantry storage shelves for fetching supplies.
      * `ElfNodePurpose::HatCloset`: Job hat dispensing wardrobe.
      * `ElfNodePurpose::KleptoStash`: Mischievous stash node behind crates.
  * **Moose Feeder Job Routine**:
    * When equipped with the `Moose Feeder` hat, an elf executes a continuous autonomous feeding loop:
      1. **Navigate to Source**: Computes path and walks to `ElfNodePurpose::MainTableTreatJar`.
      2. **Inspect Node Area**: Scans within the node radius for an available `MooseTreatJar` containing at least one snack.
      3. **Retrieve Treat**: Extracts a treat from the jar, holds it in hand, and sets destination to `ElfNodePurpose::MooseFeedingStation`.
      4. **Deliver to Trough**: Pathfinds to the feeding station and deposits the treat directly onto the active feeding trigger area, reducing Mr. Moose's hunger meter.
      5. **Repeat**: Loops back to retrieve the next treat.
    * **Panic State Fallback**:
      * If the Moose Feeder arrives at `MainTableTreatJar` and cannot find a Moose Treat jar containing treats (e.g. jar is empty, knocked away, or missing), the routine cannot proceed.
      * The elf immediately transitions into **`ElfNpcState::Panicking`**: waves arms frantically overhead, utters high-pitched diegetic alarm squeaks, and paces erratically between nearby nodes.
      * Remains in the Panic state until the player brings a fresh Moose Treat jar from the **Pantry** and places it within the node area.

---

### 2.8 Game State Persistence & Lightweight Save System
* **Implementation Schedule**: Slated near the conclusion of the implementation roadmap (Milestone 8), ensuring all core systems are stabilized before finalizing schema.
* **Selective Persistence Scope**:
  * The system deliberately persists only major milestones, player stats, and key persistent items, intentionally omitting ephemeral dynamic physics clutter:
    * **Major Progression Points**: Presents submitted count, unlocked doors and wings (Tier 2 Storage Wing, Tier 3 Forbidden Workshop), Mr. Moose awakened state, and unlocked easter egg elves (e.g. Sparky chandelier knock-down flag).
    * **Player Stats & Records**: Total gifts wrapped, current quota score, best consecutive BHOP combo streak, play session elapsed time.
    * **Key Item Transforms & State**: Coordinates, rotations, and contents of vital props (active Moose Treat jars on tables and pantry shelves, spawned job hats, tool dispenser placements, closet restock state).
    * **Excluded Ephemeral Clutter**: Transient conveyor cubes, loose wrapping paper scraps, dynamic smoke/spark particles, and flying impact debris are regenerated deterministically at startup rather than serialized.
* **Storage Architecture**:
  * Serialized using lightweight, human-readable RON (Rusty Object Notation) or JSON (`workshop_save.ron`).
  * Auto-saves on major progression milestones and provides quick-save / quick-load functionality from the in-game pause menu.

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

| Milestone | Scope | Deliverables | Status |
|---|---|---|:---:|
| **M1: Core Game Loop & Framework** | Architecture & Present State Machine | App scaffolding, Present wrapping state machine, headless unit tests. | **DONE** |
| **M2: Advanced Movement & BHOP** | Air straining, Bunny-hopping & Debug Menu | Realistic air momentum, jump timing window (1/60, 1/30, 1/10s), 1.41x speed scaling, headless physics unit tests. | **DONE** |
| **M3: Mr. Moose & Window Breach** | Moose Hunger & Physics Rampage | Hunger timer, feeding chute, unshaded toy moose model, window glass shatter event. | **DONE** |
| **M4: Chaotic Physics & Soft-Body Sandbox** | Soft bodies, ragdolls & extreme forces | Spring-mass squishy presents, bouncy toys, ragdoll elves, chaotic impulse machines. | **DONE** |
| **M5: In-World Diegetic Displays & Player Agency** | In-world monitors, signs & player choices | In-world CRT monitors, warning signs, physical scales, alternate wrapping pipelines. | **DONE** |
| **M6: Progression & The Forbidden Workshop** | Multi-tier workshop expansion | Door unlocks, quota tracker, forbidden overpowered volatile tools (Instant Plasma Wrapper, Ribbon Cannon, Cursed Candy-Cane Overclocker), Storage Wing turntable, NPC elves. | **DONE** |
| **M6.1: Pantry Storage Wing & Autonomous Restock Loop** | Storage room, infinite treat restock, initial moose dormancy | Generously sized $4\,\text{m} \times 4\,\text{m} \times 3.5\,\text{m}$ Pantry Storage Wing off west corridor, deep perimeter timber shelving (North 3-tier shelves, West supply shelves, South baking staging counter), interactive swinging physics door on Rapier `RevoluteJoint` / spring hinge with momentum, automatic restock cycle on door `Closed -> Opened` transition, Mr. Moose initial dormancy behind shutters with 1st present vacuum dispatch wakeup, and semantic navigation role `ElfNodePurpose::PantryShelves` (Node 15). | **DONE** |
| **M6.2: Autonomous Moose Feeder Routine & Panic Fallback** | Autonomous elf job loop & pantry treat reliance | Autonomous Moose Feeder routine with `ElfNpcState::FetchingMooseTreat` and `DeliveringMooseTreat`, retrieving treats from `MainTableTreatJar` (Node 1) with jar count decrement, pathfinding and delivering to `MooseFeedingStation` (Node 4) to reduce hunger by 60, carrying treat hand animations, and `ElfNpcState::Panicking` fallback when jars are empty/missing with recovery when fresh pantry jars are placed. | **DONE** |
| **M7: Visual Polish & Custom Assets** | Audio, AI textures & procedural models | AI/procedural festive textures, particles (wrapping paper confetti, glass shards), sound effects. | Planned |
| **M8: Game State Persistence & Progression Saving** | Lightweight save/load system | Save file serialization (`workshop_save.ron`), major milestone flags, player stats, key item positions, restore on startup. | Planned |

---

## 9. Idea Section & Future Concepts

### 9.1 Cookie Baking, Outdoor Gas Infrastructure & Pyro Hazard Chain
* **Cookie Baking Task Loop**:
  * **Ingredient Retrieval**: The player retrieves recipe ingredients (flour sacks, sugar bags, butter blocks, and chocolate chip tins) from the newly expanded, spacious **Pantry**.
  * **Kitchen Prep & Workshop Stove**: Ingredients are carried to the kitchen prep area, combined into cookie dough on a baking sheet, and baked using the cast-iron **Gas Furnace Stove**.
* **Outdoor Propane Gas Valve**:
  * The stove runs on external bottled gas.
  * To fuel the stove burners, the player must open the exterior cabin door, run outside into the freezing snowy blizzard to turn on the heavy outdoor brass gas valve on the exterior wall, and sprint back inside before freezing or losing interior cabin heat.
* **Elf Mischief & Pyro Hazard Chain**:
  * **Unattended Stove Risk**: If the player leaves the stove unattended with the outdoor gas valve open for too long:
    1. A **Mischievous elf** creeps over to the stove and turns on the burner knobs, allowing unignited flammable gas to steadily fill the workshop air.
    2. After a **15-second delay** of unignited gas flow, a **Pyro elf** (`ElfMischiefTrait::Pyro`) detects the gas concentration.
    3. The Pyro elf scouts the room for an active flame source (such as dynamic sparks drifting from the candle chandeliers, a workbench match, or a lighter).
    4. Upon securing an ignition source, the Pyro elf gleefully rushes over to the stove.
    5. **Slapstick Catastrophic Explosion**: Ignites a massive physics blast that blows open doors, shatters windows, and launches presents, snack jars, and ragdoll elves flying into the high ceiling rafters!
