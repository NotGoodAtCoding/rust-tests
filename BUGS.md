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

DONE:

Pantry Door Behavior: The player should grab and swing the door open and shut, not just apply a static force to it. When grabbing the door, if the player moves away then the grab should be released.

The Pantry door is glitched into the wall. It should open into the room and slowly close itself if less than 1/4 rad closed and flush with the wall. Make it a simple hinge that rotates around a fixed point, like a normal door would.

The Mouse sensitivity slider is always active, so holding Mouse1 will always move the slider. The Mouse Sensitivity slider should only activate when the options menu is active and should ONLY move when the mouse is over the slider itself.