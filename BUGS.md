HOW TO USE: 
Bug descriptions will be added to the ACTIVE section. Fix the bug and move them to the DONE section.

ACTIVE:

DONE:

Change the key to initiate dialog with elves to X instead of E.

Refactor: Tests should be co-located with the file containing the logic so that they can be run on update to the file itself rather than running the entire test file for every change. If the tests get too big for the file, then they can be separated out into a different file in the same directory and mod. label unit tests with `#[cfg(test)]` in a mod tests and label each test with `#[test]`.

In debug overlay mode, display the XYZ coordinates of the player's current position and the raycast collision point of the crosshair.

The moose will attempt to target the player even when the player is on the mezzanine above. Change the Moose to have waypoint based navigation and a randomized path across waypoints. The moose rampage should choose 5-7 waypoints. Add waypoints to the top mezzanine, and reorganize the existing waypoints so they don't go through the new rooms in the back of the cabin

Feature: Let's make small modular physics objects in assets/objects that are self-contained, well-tested, and able to be reused. e.g. Doors, Ropes, attachment points

The Pantry door is glitching and freaking out, likely due to it clipping into the wall. Fix the pantry door.

All parts of the Moose should launch the player, and after contact with the moose have a short time of non-collision with the player.

When the player is within interation distance of an interactive object, display a text name of the object. e.g. Oven, Present

The Oven gas hookup is not held by the player properly, it gets stuck. Implement a cooldown from grabbing that prevents it from reconnecting. Disable collision on the gas hose nozzle end. Make the Gas Hose have a max distance per chain including for the end nozzle. If the player gets further than the hose length, it should pull it out of their hand and they should drop it.

Feature: Propane tank fixes: Make the outside propane receptacles a platform with the AnchorPoint function with an activation area that locks the propane tank in a specific fixed place until the player interacts with it. The platform near the house supplies the stove. The platform near the outside larger propane main tank refills the smaller propane tanks. The Stove inside should be able to be turned off and on separately from the gas supply. While off AND connected, propane is not consumed. If disconnected, then propane is consumed since it's leaking. 

The lighting from the candles is flickering. This may be just due to debug fastbuild in cargo rather than a full binary build.

Move the Pantry to be inside the main cabin room as a subroom, and keep the door. The back wall of the cabin should have different rooms for the Pantry, and at least 2 more unused rooms. There should be a staircase to get to a mezzanine area above these rooms. Make the staircase have a smooth collision ramp for the player so that they ascend smoothly, but keep the stairs as the collision for objects that are not the player or elves. The mezzanine should have a wood railing and wood plank floors. Move the Hat Closet to be up in the mezzanine and turn it into a Hat Rack, which should spawn one random hat on it. Add a Mirror by the Hat Stand. If the hat has been taken off the rack and the player is not looking and not holding a hat, spawn a new random hat on the rack.

Cleanup: Colors and materials should be defined in assets/materials instead of in the file where behavior is defined so that the files do not get too cluttered. This follows the limited color palette Principle under Artistic Direction. Move material definitions from stove.rs to the assets/materials.rs file, and audit other files for extra material definitions.

Idea: Elf Mischief behavior: Philosopher. If an elf gains this Mischief trait, it will go to the Mirror Node and stay there. In order to get the elf back to work, the player needs to navigate through a deep dialogue tree. If left alone, eventually it will start recruiting other elves which will make them navigate to the Philosopher elf and then stay there with a "thinking" pose. This can be granted by putting on the Philosopher Pipe, which is a hat that makes it look like the elf has a tobacco pipe, with smoke particle effects and a soft ember glow that slowly fluctuates from the pipe.

The Pantry Door should be able to be pushed by the player and elves if they run into it. If an elf navigates to the Pantry, it should check if the door is in the way and try to open the door by grabbing the door and pulling it open, then going to the node before the pantry door, then entering the Pantry. 

Update the KleptoStash behavior to add leaf nodes with one edge back to the main graph that are in the corners of the main room. The elf should put the stolen item in front of them when stashing it, and remember the last item stashed and not target that item again to prevent getting stuck in a loop. There should also be a longer cooldown between stashing items, like 15 seconds.

If an elf is unable to navigate to its target node in 10 seconds, randomly choose a different node to navigate to, in order to prevent getting stuck. If the elf fails to navigate to a node 3 times, respawn the elf from the present vaccuum and play the vaccuum animation in reverse.

Item Idea: Bunny ears. On equip, player viewmodel should show floppy bunny ears with physics bob on the top of the view screen. This should also make bunnyhopping mechanic easier by setting the grace period to 1/10sec when equipped. Players with the bunny ears equipped jump 10% higher and have 2x the in air straining vs. normal limits.

Feature: Dynamic waypoint targeting: Elves and NPCs should be able to target other NPCs and the Player location as an option. When the moose is rampaging, if the player gets within 5m of the moose, set the target location to the player. As long as the player is within 6m of the moose once triggered, or 5 seconds have passed since the moose targeted the player, keep updating the target location of the moose to the Player. Once the moose contacts the player, the moose cannot target the player again for 3 seconds. Also, make the wander radius around waypoints slightly larger.

The stable door top half opens slowly and smoothly with Mr. Moose as he slowly pokes his head in across the 30+ second peeking stage (hunger 20 to 50), tracking his forward snout progression from fully closed to open.

When the player runs into elves at default speed cap, they should not get a large impulse. They should only ragdoll if the player is going above the default speed cap from sprinting or bunny hopping.

Player grabbing objects should find the object that is in direct raycasted Line of Sight with the player's reticle instead of being a general cone. 

The Tractor Beam does not give enough force on the player to fully suck them up, increase the force. Move the activation button further away from the tractor beam so that the player cannot activate it when in the tractor beam area.

Replace the moose treat bucket with a festive cookie plate on a table next to the Stable Door. It should have high friction so that treats thrown onto it are more likely to stick, and a raised rim as well. Make world gravity higher so physics objects feel less floaty, and give them some dampening. The player throw impulse should be scaled up slightly to compensate.

Moose body should have collision with the scarf.

The Pantry should only have one jar of Moose Treats at a time. Once removed, it should not refresh until the door is closed and then reopened, which should reset all of the Pantry supplies to the default initialized state.

The Pantry section is not fully closed off, there are big holes in the walls that allow the player to enter without opening the door.

in dialogue.rs: Make the text for the Elf Prompts larger and in a rectangular box at the bottom of the screen rather than in the middle. Leave the center of the wheel clear and transparent so the player can see the elf they are talking to. If the player inputs a keyboard navigation option, set the mouse position to the center so that it doesn't reset the chosen option on every frame.

Moose: Remove the previous double shutter window. Update the Moose default and resting location to the new area, and update all the coordinates for its pathing while rampaging. Make sure the food trough location and active area is updated to the new location in the center of the wall. Make the top of the stable door open as the Moose pokes his head in, and slam shut when he goes back.

Stove and oven: The yellow hose connector should be a flexible chain, similar to the scarf asset on the Moose. it should have one permanent attach point on the wall, and two detachable anchor points on the stove and the oven. It should snap to connect if the end of the hose is held by the player and gets within about .2 meters of the attach point. Make both the stove and oven bigger. Move Mr. Moose to be closer to the center of the wall, and replace the window he peeks through with a Stable door, which has a top and bottom half section that can open independently. When Moose pokes is head through, only the top half opens. When Moose rampages, both the top and bottom should fly open and be physics objects rotating freely on their hinges within 2 radian range from the closed orientation to the fully open orientation, with a large impulse from the Moose. When the Moose is fed or when the rampage ends, set both the top and bottom to be fully closed. Make the door grabbable but not movable by the player. This will be a key part of progression for the player to get outside. 

Feature: Oven and Stove: Make a model for an oven and a gas burning furnace stove. The player must keep the temperature up or everything will freeze and presents will stop spawning in. Repurpose the hunger_dial to be a Pressure gauge for the amount of Gas left to power the stove. Move the gauge to be by the stove, which should be by the window. Instead of a single large glass window, make a series of smaller windows that are large enough for the player to jump and climb through. Outside the house make a propane tank holder that the player can replace the propane tanks, and then refill them at the main tank which is a distance away. Inside, there should be a pipe with a flexible thin yellow hose. This can be hooked up to either the Oven or the Stove by pulling it off of one anchor point and attaching it to the other. If it's attached to neither and the gas is on, if a FLAME source gets within 1 meter of the hose, cause a fiery explosion.

Pantry Door Behavior: The player should grab and swing the door open and shut, not just apply a static force to it. When grabbing the door, if the player moves away then the grab should be released.

The Pantry door is glitched into the wall. It should open into the room and slowly close itself if less than 1/4 rad closed and flush with the wall. Make it a simple hinge that rotates around a fixed point, like a normal door would.

The Mouse sensitivity slider is always active, so holding Mouse1 will always move the slider. The Mouse Sensitivity slider should only activate when the options menu is active and should ONLY move when the mouse is over the slider itself.