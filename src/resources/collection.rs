use std::collections::HashMap;
use std::ops::Index;

use crate::resources::texture;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct SparseEntry(usize, bool);

pub struct ResourceCollection<T> {
    dense: Vec<T>,
    dense_ids: Vec<usize>,
    sparse: Vec<SparseEntry>,

    keys: HashMap<String, usize>,
}

impl<T> ResourceCollection<T> {
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            dense_ids: Vec::new(),
            sparse: Vec::new(),
            keys: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: impl ResourceId, value: impl Into<T>) {
        let value = value.into();
        let id = id.to_index(self);

        if self.sparse.len() < id {
            self.sparse.resize(id, SparseEntry(0, false));
        }
        self.sparse.push(SparseEntry(self.dense.len(), true));

        self.dense.push(value);
        self.dense_ids.push(id);
    }

    fn key(&mut self, key: impl Into<String>) -> usize {
        let i = self.keys.len();
        *(self.keys.entry(key.into()).or_insert(i))
    }

    pub fn remove(&mut self, id: impl ResourceId) -> Option<T> {
        let id = id.get_index(self)?;
        let entry = &mut self.sparse[id];
        if !entry.1 {
            return None;
        }

        let dense_index = entry.0;
        let last_index = self.dense.len() - 1;

        if dense_index == last_index {
            self.dense_ids.pop();
            return self.dense.pop();
        }

        self.dense.swap(dense_index, last_index);
        self.dense_ids.swap(dense_index, last_index);

        let removed = self.dense.pop();

        entry.1 = false;
        self.sparse[self.dense_ids[dense_index]].0 = dense_index;

        removed
    }

    pub fn get(&self, id: impl ResourceId) -> Option<&T> {
        let id = id.get_index(self)?;
        let entry = &self.sparse[id];
        if !entry.1 {
            return None;
        }

        Some(&self.dense[entry.0])
    }
    pub fn get_mut(&mut self, id: impl ResourceId) -> Option<&mut T> {
        let id = id.get_index(self)?;
        let entry = &self.sparse[id];
        if !entry.1 {
            return None;
        }

        Some(&mut self.dense[entry.0])
    }

    pub fn is_alive(&self, id: impl ResourceId) -> bool {
        let Some(id) = id.get_index(self) else {
            return false;
        };
        self.sparse[id].1
    }
}

impl<T> Default for ResourceCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ResourceId {
    /// converts `self` to a `usize` index for `collection.sparse`, by getting returning an
    /// existing index or creating one
    fn to_index<T>(self, collection: &mut ResourceCollection<T>) -> usize;
    /// converts `self` to a `usize` index for `collection.sparse`, returning `None` if it doesnt
    /// exist
    fn get_index<T>(self, collection: &ResourceCollection<T>) -> Option<usize>;
}

impl ResourceId for usize {
    #[inline(always)]
    fn to_index<T>(self, _: &mut ResourceCollection<T>) -> usize {
        self
    }
    #[inline]
    fn get_index<T>(self, collection: &ResourceCollection<T>) -> Option<usize> {
        if collection.sparse.len() > self {
            Some(self)
        } else {
            None
        }
    }
}
impl ResourceId for &str {
    #[inline]
    fn to_index<T>(self, collection: &mut ResourceCollection<T>) -> usize {
        collection.key(self)
    }
    #[inline]
    fn get_index<T>(self, collection: &ResourceCollection<T>) -> Option<usize> {
        collection.keys.get(self).copied()
    }
}
impl ResourceId for String {
    #[inline]
    fn to_index<T>(self, collection: &mut ResourceCollection<T>) -> usize {
        collection.key(self)
    }
    #[inline]
    fn get_index<T>(self, collection: &ResourceCollection<T>) -> Option<usize> {
        collection.keys.get(&self).copied()
    }
}
/*impl<T> Index<Entity> for ResourceCollection<T> {
    type Output = T;

    fn index(&self, entity: Entity) -> &Self::Output {
        self.get(entity).expect("Invalid entity")
    }
}*/

impl<T> Index<&'static str> for ResourceCollection<T> {
    type Output = T;

    fn index(&self, key: &'static str) -> &Self::Output {
        self.get(key)
            .expect("tried to get dead or or non-existent resource")
    }
}

impl<T> Index<usize> for ResourceCollection<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("tried to get dead or non-existent resource")
    }
}
use bytemuck::Pod;
use std::mem::size_of;
use wgpu::util::DeviceExt;

pub struct StorageVec {
    pub buffer: wgpu::Buffer,
    pub len: u32,
    pub capacity: u32,
    pub align: u32,
}

impl StorageVec {
    pub fn new<T: Pod>(device: &wgpu::Device, usage: wgpu::BufferUsages, initial: &[T]) -> Self {
        let align = size_of::<T>() as u32;
        assert!(align > 0, "Element size must be > 0");

        let capacity = initial.len().max(1) as u32;

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(initial),
            usage: usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        Self {
            buffer,
            len: initial.len() as u32,
            capacity,
            align,
        }
    }

    fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, new_capacity: u32) {
        let new_size = new_capacity as u64 * self.align as u64;

        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: new_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("storage_vec_resize"),
        });

        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &new_buffer,
            0,
            self.len as u64 * self.align as u64,
        );

        queue.submit(Some(encoder.finish()));

        self.buffer = new_buffer;
        self.capacity = new_capacity;
    }

    pub fn push<T: Pod>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, value: &T) {
        assert_eq!(
            size_of::<T>() as u32,
            self.align,
            "StorageVec element size mismatch"
        );

        assert_eq!(
            size_of::<T>() as u32,
            self.align,
            "StorageVec element size mismatch"
        );

        if self.len == self.capacity {
            let new_capacity = (self.capacity.max(1)) * 2;
            self.resize(device, queue, new_capacity);
        }

        let offset = self.len as u64 * self.align as u64;

        queue.write_buffer(&self.buffer, offset, bytemuck::bytes_of(value));

        self.len += 1;
    }

    pub fn insert<T: Pod>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: u32,
        value: &T,
    ) {
        assert!(index <= self.len, "Index out of bounds");
        assert_eq!(
            size_of::<T>() as u32,
            self.align,
            "StorageVec element size mismatch"
        );

        // resize if necessary
        if self.len == self.capacity {
            let new_capacity = (self.capacity.max(1)) * 2;
            self.resize(device, queue, new_capacity);
        }

        let elem_size = self.align as u64;
        let offset = index as u64 * elem_size;
        let move_bytes = (self.len - index) as u64 * elem_size;

        if move_bytes > 0 {
            // shift existing elements up
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("storage_vec_insert"),
            });

            encoder.copy_buffer_to_buffer(
                &self.buffer,
                offset,
                &self.buffer,
                offset + elem_size,
                move_bytes,
            );

            queue.submit(Some(encoder.finish()));
        }

        // write the new element
        queue.write_buffer(&self.buffer, offset, bytemuck::bytes_of(value));

        self.len += 1;
    }

    pub fn extend<T: Pod>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, values: &[T]) {
        assert_eq!(
            size_of::<T>() as u32,
            self.align,
            "StorageVec element size mismatch"
        );

        let required = self.len + values.len() as u32;

        if required > self.capacity {
            let mut new_capacity = self.capacity.max(1);
            while new_capacity < required {
                new_capacity *= 2;
            }

            self.resize(device, queue, new_capacity);
        }

        let offset = self.len as u64 * self.align as u64;

        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(values));

        self.len += values.len() as u32;
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn byte_len(&self) -> u64 {
        self.len as u64 * self.align as u64
    }

    pub fn byte_capacity(&self) -> u64 {
        self.capacity as u64 * self.align as u64
    }
}
// TODO: sampler and texture arrays
pub enum BindingResource {
    Buffer(wgpu::Buffer),
    BufferArray(Vec<wgpu::Buffer>),
    Texture(texture::Texture),
    AccelerationStructure(wgpu::Tlas),
    ExternalTexture(wgpu::ExternalTexture),
    StorageVec(StorageVec),
}
impl BindingResource {
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            Self::Buffer(b) => b.as_entire_binding(),
            Self::BufferArray(_) =>
            //wgpu::BindingResource::BufferArray(
            {
                todo!()
            } //b.iter().map(|x| x.as_entire_buffer_binding()).collect(),
            //),
            Self::Texture(_) => panic!(
                "x_x :: tried to get the binding() of a texture. do this on its individual components"
            ),
            Self::AccelerationStructure(a) => a.as_binding(),
            Self::ExternalTexture(e) => wgpu::BindingResource::ExternalTexture(e),
            Self::StorageVec(s) => s.buffer.as_entire_binding(),
        }
    }

    // TODO: try as inner buffer
    // also, maybe? change to as_buffer? hm?
    pub fn as_inner_buffer(&self) -> &wgpu::Buffer {
        let BindingResource::Buffer(buf) = self else {
            panic!("aaa noo oh no!")
        };
        buf
    }
}
impl From<wgpu::Buffer> for BindingResource {
    fn from(val: wgpu::Buffer) -> Self {
        BindingResource::Buffer(val)
    }
}
impl From<Vec<wgpu::Buffer>> for BindingResource {
    fn from(val: Vec<wgpu::Buffer>) -> Self {
        BindingResource::BufferArray(val)
    }
}
impl From<texture::Texture> for BindingResource {
    fn from(val: texture::Texture) -> Self {
        BindingResource::Texture(val)
    }
}
impl From<wgpu::Tlas> for BindingResource {
    fn from(val: wgpu::Tlas) -> Self {
        BindingResource::AccelerationStructure(val)
    }
}
impl From<wgpu::ExternalTexture> for BindingResource {
    fn from(val: wgpu::ExternalTexture) -> Self {
        BindingResource::ExternalTexture(val)
    }
}
impl From<StorageVec> for BindingResource {
    fn from(val: StorageVec) -> Self {
        BindingResource::StorageVec(val)
    }
}
