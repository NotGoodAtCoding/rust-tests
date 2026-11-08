HOW TO USE: 
Bug descriptions will be added to the ACTIVE section. Fix the bug and move them to the DONE section.

ACTIVE:

The lighting from the candles is flickering. This may be just due to debug fastbuild in cargo rather than a full binary build.

Feature: Dynamic waypoint targeting: Elves and NPCs should be able to target other NPCs and the Player location as an option. 

When the player runs into elves at default speed cap, they should not get a large impulse. They should only ragdoll if the player is going above the default speed cap from sprinting or bunny hopping.

Item Idea: Bunny ears. On equip, player viewmodel should show floppy bunny ears with physics bob on the top of the view screen. This should also make bunnyhopping mechanic easier by setting the grace period to 1/10sec when equipped.

The Moose Hunger meter is behind the wall, it needs to be brought forward into the room. Also, make it a physics object so it gets knocked off the wall if hit.

If an elf is unable to navigate to its target node in 10 seconds, randomly choose a different node to navigate to, in order to prevent getting stuck. If the elf fails to navigate to a node 3 times, respawn the elf from the present vaccuum and play the vaccuum animation in reverse.

The Pantry Door should be able to be pushed by the player and elves if they run into it. If an elf navigates to the Pantry, it should check if the door is in the way and try to open the door by grabbing the door and pulling it open, then going to the node before the pantry door, then entering the Pantry. 

Idea: Elf Mischief behavior: Philosopher. If an elf gains this Mischief trait, it will go to the Mirror Node and stay there. In order to get the elf back to work, the player needs to navigate through a deep dialogue tree. If left alone, eventually it will start recruiting other elves which will make them navigate to the Philosopher elf and then stay there with a "thinking" pose.

Update the KleptoStash behavior to add leaf nodes with one edge back to the main graph that are in the corners of the main room. The elf should put the stolen item in front of them when stashing it, and remember the last item stashed and not target that item again to prevent getting stuck in a loop. There should also be a longer cooldown between stashing items, like 15 seconds.

DONE:

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