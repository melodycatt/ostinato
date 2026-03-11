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

// TODO: sampler and texture arrays
pub enum BindingResource {
    Buffer(wgpu::Buffer),
    BufferArray(Vec<wgpu::Buffer>),
    Texture(texture::Texture),
    AccelerationStructure(wgpu::Tlas),
    ExternalTexture(wgpu::ExternalTexture),
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
