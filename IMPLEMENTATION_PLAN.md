# Lightweight Implementation Plan: Santa's Workshop Elf Simulator

This document tracks the short-form task breakdown for each section of [`PLAN.md`](file:///home/stef/repo/rust-tests/PLAN.md) with implementation status.

---

## 1. Core Gameplay Principles

| Section | Task Description | Status | Reference Files |
|---|---|:---:|---|
| **2.1 Diegetic Communication** | In-world CRT task monitor rendering Santa OS stats, live quota, and moose status via `Text2d` | **DONE** | [`monitor.rs`](file:///home/stef/repo/rust-tests/src/elf_game/diegetic_ui/monitor.rs) |
| **2.1 Diegetic Communication** | Wooden warning sign next to Mr. Moose window ("WARNING: KEEP MR. MOOSE FED OR ELSE") | **DONE** | [`monitor.rs`](file:///home/stef/repo/rust-tests/src/elf_game/diegetic_ui/monitor.rs) |
| **2.1 Diegetic Communication** | Physical mechanical dial / scale outside window representing hunger | TODO | Planned |
| **2.2 Physics & Chaos** | Spring-mass `SoftBodyLattice` simulation with Hooke's Law and velocity damping | **DONE** | [`physics_sandbox/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/physics_sandbox/mod.rs) |
| **2.2 Physics & Chaos** | Exaggerated slapstick impulses on dynamic bodies during moose rampage ($15\,\text{N}\cdot\text{s}$ force $+ 8\,\text{N}\cdot\text{s}$ lift) | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| **2.2 Physics & Chaos** | Player launch impulse on rampage contact (random upward launch $v_y \ge 10\,\text{m/s}$) | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| **2.2 Physics & Chaos** | Soft-body mesh deformation renderer and ragdoll NPC assistants | TODO | Planned |
| **2.3 Player Agency** | Flexible attribute-based present wrapping (choose tools, ordering, and methods) | **DONE** | [`wrapping/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/wrapping/mod.rs) |
| **2.3 Player Agency** | Physical conveyor belt interactions, manual pickup, and feeding options | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs), [`hands.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/hands.rs) |
| **2.3 Player Agency** | Alternate automated assembly lines and dynamic conveyor catapults | TODO | Planned |

---

## 2. Core Mechanics

### 2.1 Present Wrapping System (Attribute & Specification Driven)
| Task Description | Status | Reference Files |
|---|:---:|---|
| Attribute set architecture (`WrappingFeature` flags) replacing rigid stage enums | **DONE** | [`wrapping/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/wrapping/mod.rs) |
| `WrappingSpecification` with `required_features` and `forbidden_features` | **DONE** | [`wrapping/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/wrapping/mod.rs) |
| Dynamic quality scoring & order satisfaction check (`meets_specification`) | **DONE** | [`wrapping/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/wrapping/mod.rs) |
| In-world wrapping tools on workbench: Scissors, Tape Dispenser, Paper Roll, Bow Box | **DONE** | [`hands.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/hands.rs) |
| Forbidden tool logic: `ForbiddenPlasmaShrink` instant-wrap and high quality | **DONE** | [`wrapping/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/wrapping/mod.rs) |
| Gift tag pen writing and customizable paper pattern variants (polka dots, stripes) | TODO | Planned |

### 2.2 Main Room Layout, Cabin Architecture & Pneumatics
| Task Description | Status | Reference Files |
|---|:---:|---|
| Enclosed 3D workshop geometry with floor, ceiling, and lighting framework | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Scaled room dimensions to 60% ($26\,\text{m} \times 26\,\text{m}$) | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **Cabin Aesthetic**: Warm timber log/plank walls, exposed rustic post framing, and stone/wood accents replacing stark flat walls | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **High Vaulted Ceiling & Wooden Cross Beams**: Vaulted pitched roof ridge ($8.5\,\text{m}$ peak) with structural timber cross-beams and rafters spanning the workshop | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **Dual Chandelier Dynamic Candle Lighting**: Two rustic wrought-iron chandeliers suspended from cross-beams with flickering candle point lights | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Main conveyor belt ($18\,\text{m}$, speed $2.0\,\text{m/s}$) carrying toy cubes forward | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Periodic conveyor cube spawner continuously providing raw cubes | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Player riding physics: conveyor moves the player when standing on the belt | **DONE** | [`controller.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/controller.rs) |
| **Floor-Level Recessed Despawn Chute**: Floor-mounted chute with beveled funnel rim; presents tumble off belt and fall down into recessed sub-floor shaft before despawning out of sight | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Central wrapping workbench ($4.0\,\text{m} \times 1.8\,\text{m}$) and diegetic tool layout | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **Present Sending Platform & Suction Cone**: Dedicated designated loading platform with interactive physical send button and overhead circular intake funnel ($4\,\text{m}$ up) exerting upward/inward suction cone forces on physics objects | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs), [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| **Physics Tube Present Vacuum**: Continuous curved pneumatic tube with loops linking the $4\,\text{m}$ intake funnel across the rafters to the dispatch chute | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **Dynamic Lump Progression & Centrifugal Tube Impulses**: Visible soft-body/deformable travelling lump moving along the 3D tube spline, exerting directional and centrifugal reaction impulses ($F_c = m v^2 / R$) proportional to original present size/mass | TODO | [`physics_sandbox/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/physics_sandbox/mod.rs), [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| Outdoor winter scene with snow landscape and scenic pine trees | **DONE** | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Workshop progression tiers (`WorkshopQuota`, `check_workshop_progression`, `UnlockDoorMessage`) | **DONE** | [`workshop/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/mod.rs) |
| Physical door barrier entities opening to Tier 2 (Storage Wing) and Tier 3 (Forbidden Room) | TODO | Planned |
| Roaming NPC assistant elves delivering supplies or panicking | TODO | Planned |

### 2.3 Mr. Moose Mechanics (Stages 0–60 & Rampage)
| Task Description | Status | Reference Files |
|---|:---:|---|
| Stylized solid brown unshaded plastic toy model (`unlit: true`, antlers, stilt legs) | **DONE** | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs) |
| Moose window positioned off to the side ($X = -6.5$, not behind monitor) | **DONE** | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs), [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| Hunger progression 0–60 (+1/sec default) with feeding trough snack consumption | **DONE** | [`moose/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/mod.rs) |
| Stage 1 (0–20): Moose rests outside behind closed shutters | **DONE** | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs) |
| Stage 2 (20–50): Shutters open; Moose moves forward and slowly pokes head into window | **DONE** | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs) |
| Stage 3 (50–60): Agitated huffing state with head poked inside room | **DONE** | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs) |
| Stage 4 (60): Window slammed shut with 3–12s random countdown delay before breach | **DONE** | [`moose/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/mod.rs) |
| Stage 5: Breakable window shatters and Moose rampages along set waypoints | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| Rampage orientation fix: Moose faces forward (+Z head) along direction of motion | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| Player launch on rampage contact: launched in random upwards direction | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| Rampage retreat & reset: Moose returns outside, shutters close, window restores, hunger resets | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| Audio cues: Moose huffing/grunting audio playback and glass shatter sound effects | TODO | Planned |

### 2.4 Mr. Moose Enhanced Interactions & Detailed Visuals
| Task Description | Status | Reference Files |
|---|:---:|---|
| **Task 1: Head-Sized Window & Inward-Rotating Shutters**: Window cutout sized just large enough for Moose head ($1.8\,\text{m} \times 1.8\,\text{m}$); dual shutters hinge and swing/rotate inward as Mr. Moose pokes head in during hunger 20–50 | **DONE** | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs), [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **Task 2: Realistic Mr. Moose Model**: Upgrade stylized solid unshaded brown toy block model to a realistic anatomical moose mesh (sculpted snout, nostrils, ears, branching palmated antlers, eyes, fur shader) | TODO | [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs) |
| **Task 3: Post-Rampage Hunger Timer Reset**: Reset hunger timer back to 0 and restore initial shutter state after rampage waypoint retreat completes | **DONE** | [`events.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/events.rs) |
| **Task 4: Windowsill Physics Decorations**: Miniature decorative physics props on windowsill (little Christmas tree, mug of hot cocoa) that can be knocked over or picked up | TODO | [`room.rs`](file:///home/stef/repo/rust-tests/src/elf_game/workshop/room.rs) |
| **Task 5: Main Table Moose Snacks Jar & Feeding Stage (50–60)**: Glass jar filled with small Moose Snacks on main workbench table; snack brought to food box waits until hunger 50–60 (waiting/feeding stage), then despawns and resets hunger to 0 | TODO | [`hands.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/hands.rs), [`moose/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/mod.rs) |
| **Task 6: Red-and-White Striped Soft-Body Physics Scarf**: Soft-body spline/chain physics scarf wrapped around Mr. Moose's neck that flows dynamically with movement | TODO | [`physics_sandbox/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/physics_sandbox/mod.rs), [`toy_model.rs`](file:///home/stef/repo/rust-tests/src/elf_game/moose/toy_model.rs) |

### 2.5 Dual-Hand Player Control & Physics Interaction
| Task Description | Status | Reference Files |
|---|:---:|---|
| Independent dual-hand grab controls: Left Hand (`KeyQ`) and Right Hand (`KeyE`) | **DONE** | [`hands.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/hands.rs) |
| Kinematic object holding and follow for grabbed tools and presents | **DONE** | [`hands.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/hands.rs) |
| Viewmodel glove visuals parented to camera pivot with vertical pitch look | **DONE** | [`controller.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/controller.rs) |
| Procedural viewmodel hand bobbing while walking/grounded | **DONE** | [`controller.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/controller.rs) |
| Two-handed tape pulling up to length limit ($0.40\,\text{m}$) with tangling penalty | **DONE** | [`player/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/mod.rs) |
| Two-handed ribbon shaking: rapid shaking ($\Delta v$) ties ribbon into decorative bow | **DONE** | [`player/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/mod.rs) |
| Two-handed heavy item lifting penalty (single-hand dragging/slipping for bulky items) | TODO | Planned |

### 2.6 Player Movement Physics: Air Control & Bunny-Hopping (BHOP)
| Task Description | Status | Reference Files |
|---|:---:|---|
| Air-straining physics: restricted steering acceleration in air ($\approx 12.5\%$ ground accel) | **DONE** | [`player/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/mod.rs) |
| Bunny-hopping landing timing window (`1/60s`, `1/30s`, `1/10s`) | **DONE** | [`player/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/mod.rs) |
| Anti-spacebar-holding guard (must press Space freshly on impact to hop) | **DONE** | [`player/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/mod.rs) |
| Speed combo scaling up to $\sqrt{2} \approx 1.414\times$ base speed after 3+ consecutive hops | **DONE** | [`player/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/player/mod.rs) |
| Options & Debug menu (`[TAB]`): BHOP window duration toggle button | **DONE** | [`ui.rs`](file:///home/stef/repo/rust-tests/src/elf_game/ui.rs) |
| In-game HUD: FPS meter, Physics stats, Controls guide, Crosshair, Sensitivity slider | **DONE** | [`ui.rs`](file:///home/stef/repo/rust-tests/src/elf_game/ui.rs) |

---

## 3. Architecture & Testing

| Task Description | Status | Reference Files |
|---|:---:|---|
| Dedicated `src/elf_game/` module and `src/bin/elf_game.rs` executable binary | **DONE** | [`src/bin/elf_game.rs`](file:///home/stef/repo/rust-tests/src/bin/elf_game.rs) |
| Modular decoupled plugins (`CorePlugin`, `PlayerPlugin`, `WrappingPlugin`, etc.) | **DONE** | [`src/elf_game/mod.rs`](file:///home/stef/repo/rust-tests/src/elf_game/mod.rs) |
| Headless unit test suite with 15 passing deterministic tests running in 0.02s | **DONE** | [`tests/elf_game_tests.rs`](file:///home/stef/repo/rust-tests/tests/elf_game_tests.rs) |
| Independent `cargo run --bin rust-tests` physics demo preserved | **DONE** | [`src/main.rs`](file:///home/stef/repo/rust-tests/src/main.rs) |

---

## 4. Roadmap Milestone Status

- [x] **M1: Core Game Loop & Framework**: Architecture, attribute wrapping, and headless test suite.
- [x] **M2: Advanced Movement & BHOP**: Air straining, landing timing windows, speed combo scaling, debug menu.
- [x] **M3: Mr. Moose & Window Breach**: Hunger timer stages, off-center window, breach rampage, facing fix, retreat loop.
- [x] **M4 (Core): Chaotic Physics Sandbox**: Soft-body lattice simulation, extreme slapstick rampage impulses, player launch.
- [x] **M5 (Core): In-World Diegetic Displays & Conveyor Belt**: In-world CRT monitor, warning sign, conveyor belt with player riding & despawn chute.
- [ ] **M5.1: Cabin Architecture, Vaulted Ceiling & Pneumatic Vacuum Tube**: Timber cabin aesthetic, $8.5\,\text{m}$ vaulted ceiling with cross beams, dual candle chandeliers, floor-recessed chute, and present sending platform with soft-body looped vacuum tube.
- [ ] **M6: Progression Expansion & Forbidden Workshop**: Physical doors opening to Storage Wing & Forbidden Back Workshop with volatile machines.
- [ ] **M7: Visual & Audio Polish**: AI-generated festive wrapping textures, particle effects (glass shards, wrapping paper confetti), sound effects.
