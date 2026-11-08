HOW TO USE: 
Bug descriptions will be added to the ACTIVE section. Fix the bug and move them to the DONE section.

ACTIVE:

The Pantry section is not fully closed off, there are big holes in the walls that allow the player to enter without opening the door.

The Pantry should only have one jar of Moose Treats at a time. Once removed, it should not refresh until the door is closed and then reopened, which should reset all of the Pantry supplies to the default initialized state.

Moose body should have collision with the scarf.

The Tractor Beam does not give enough force on the player to fully suck them up, increase the force. Move the activation button further away from the tractor beam so that the player cannot activate it when in the tractor beam area.

Player grabbing objects should find the object that is in direct raycasted Line of Sight with the player's reticle instead of being a general cone. 

In debug noclip mode, the player sometimes floats around without any input.

The lighting from the candles is flickering. This may be just due to debug fastbuild in cargo rather than a full binary build.

When the player runs into elves at default speed cap, they should not get a large impulse. They should only ragdoll if the player is going above the default speed cap from sprinting or bunny hopping.

Item Idea: Bunny ears. On equip, player viewmodel should show floppy bunny ears with physics bob on the top of the view screen. This should also make bunnyhopping mechanic easier by setting the grace period to 1/10sec when equipped.

The Moose Hunger meter is behind the wall, it needs to be brought forward into the room. Also, make it a physics object so it gets knocked off the wall if hit.

If an elf is unable to navigate to its target node in 10 seconds, randomly choose a different node to navigate to, in order to prevent getting stuck. If the elf fails to navigate to a node 3 times, respawn the elf from the present vaccuum and play the vaccuum animation in reverse.

The Pantry Door should be able to be pushed by the player and elves if they run into it. If an elf navigates to the Pantry, it should check if the door is in the way and try to open the door by grabbing the door and pulling it open, then going to the node before the pantry door, then entering the Pantry. 

Idea: Elf Mischief behavior: Philosopher. If an elf gains this Mischief trait, it will go to the Mirror Node and stay there. In order to get the elf back to work, the player needs to navigate through a deep dialogue tree. If left alone, eventually it will start recruiting other elves which will make them navigate to the Philosopher elf and then stay there with a "thinking" pose.

Update the KleptoStash behavior to add leaf nodes with one edge back to the main graph that are in the corners of the main room. The elf should put the stolen item in front of them when stashing it, and remember the last item stashed and not target that item again to prevent getting stuck in a loop. There should also be a longer cooldown between stashing items, like 15 seconds.

DONE:

Pantry Door Behavior: The player should grab and swing the door open and shut, not just apply a static force to it. When grabbing the door, if the player moves away then the grab should be released.

The Pantry door is glitched into the wall. It should open into the room and slowly close itself if less than 1/4 rad closed and flush with the wall. Make it a simple hinge that rotates around a fixed point, like a normal door would.

The Mouse sensitivity slider is always active, so holding Mouse1 will always move the slider. The Mouse Sensitivity slider should only activate when the options menu is active and should ONLY move when the mouse is over the slider itself.