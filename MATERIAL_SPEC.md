all wgpu enums should be uppercase
todo make default values and document them

# entry_points
the entry_points in the shaders this material uses for each stage
values:
- vertex (entry_point) : entry point for the vertex stage
- fragment (entry_point) : entry point for the fragment stage

## entry_point
an entry point function
values:
- module (string) : resource path of the wgsl shader containing the entry point
- function (string) : the function in the wgsl shader to be used as the entry point

# shared_bind_groups (optional)
TODO make this actually optional
a list of the labels of any shared bind groups to be used (e.g. CAMERA)
the point of this is to be able to store one uniform for resources that will be used for many shaders
values
- (string[]) : a shared_bind_groups key/label

# bind_groups (optional)
TODO make this actually optional (why tho)
a list that describes bind groups for this shader
the `@group` numbers of these bind groups in the wgsl will be the order they are described in the omi
values:
- (bind_group[]) : a bind group

## bind_group
a bind_group descriptor
values:
- label (string) : the label of the bind group (debug thing i think)
- entries (binding[]) : list of bindings in the group

## binding
a bind group binding descriptor
required properties:
- binding (uint) : the `@binding` number of this binding in wgsl shader
- visibility (`wgpu::ShaderStages`) : the shader stages this binding is visible to
- type: (`wgpu::BindingType`) : the type of this binding
  - BUFFER:
    - buffer (map) : buffer info
      - type ("UNIFORM" | "STORAGE" | "READ_ONLY_STORAGE") : buffer type
      - has_dynamic_offset (bool) : see wgpu spec
      - min_binding_size (null | uint) : see wgpu spec (non-zero)
  - SAMPLER:
    - sampler (`wgpu::SamplerBindingType`) : sampler binding type
    - image_path (string) : image path. just make this the same as resource because i dont know. thats scary...
  - TEXTURE:
    - texture (map) : texture info
      - multisampled (bool) : see wgpu spec
      - view_dimension (`wgpu::TextureViewDimension`) : see wgpu spec
      - sampler_type (`wgpu::TextureSampleType`) : see wgpu spec
      - filterable? (bool) : (only if sampler_type is FLOAT) see wgpu spec
     - image_path (string) : image path. just make this the same as resource because i dont know. thats scary...
  - STORAGE_TEXTURE:
    - scary, check the spec it should follow the structure of wgpu::BindingType::StorageTexture 
  - ACCELERATION_STRUCTURE:
    - same as STORAGE_TEXTURE
- count (null | uint) : see wgpu spec (non-zero)
- resource (string) : resource label of the binding value

# buffers (optional)
vertex buffers
depends on `custom_vertex` feature
values:
- (string[]) : vertex buffer type name. see `mesh::desc_from_name()` for possible values. if `custom_vertex` is enabled other values are allowed

# immediate_size
(uint) : size of immediate values in shader
