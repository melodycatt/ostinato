okay guys...... this is the plan.!

we have 4 bind groups:
- global (everything on the pass uses these)
- environment (lights etc)
- shader-type (e.g. material info)
- object (e.g. a texture)

ex: for a blinn phong shader
globals     - static stuff: camera, time, resolution, etc
environment - we might have 2 different environments (e. g. we a mirror), so we can change the lights for it
shader-type - maybe we have a "light indices" storage buffer containing the lights to actually render for each object with dynamic offsets
              and a "light count" uniform buffer with the amount of indices to read from that storage buffer
            - maybe the material goes here
object      - mmmmmayyyybe the material goes here
            - textures probably here

then push constants for stuff like:
- transform
- maybe material
- etc



the actual framework for this could be like traits that define how to apply push constants and bind groups
we can sort by these bind groups to decrease the amount of times we set the bind groups
  

pass structure:
bind group : a static group for the pass
render layers?  maybe a better name:
- we have them ordered by priority,

just writing this down to remember
we have the pass type and we have different levels as structs with a train and all the same layout and bind group generator blah blah
pipeline type struct that contains l4 bind group as well aswhich other levels it uses? or smth like that and ofc the pipeline layout info
