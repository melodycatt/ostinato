/*struct Interner {
    map: HashMap<String, usize>,
    id: usize,
    free: Vec<usize>,
}
impl Interner {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            id: 0,
            free: Vec::new(),
        }
    }
    pub fn intern(&mut self, s: &str) -> usize {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = if !self.free.is_empty() {
            self.free.pop().unwrap()
        } else {
            self.id += 1;
            self.id - 1
        };
        self.map.insert(s.to_owned(), id);
        id
    }
    pub fn remove(&mut self, s: &str) -> anyhow::Result<()> {
        let id = self
            .map
            .remove(s)
            .ok_or(anyhow::Error::msg("x_x :: removed uninterned string"))?;
        self.free.push(id);
        Ok(())
    }
}

// TODO make a wrapper for the usize so that we can implement &str::into::<index>()
/// a kind of vec used to make stuff faster than a hashmap
/// i wanted to be able to register resources like `Material`s and `BindingResource`s with string ids
/// without the overhead of a hashmap
/// so this interns strings as indices for you to store and then index into `resources` later on
/// this might be a stupid implementation but whatever
pub struct ResourceCollection<T: 'static> {
    interner: Interner,
    pub resources: Vec<Option<T>>,
}
impl<T: 'static> Default for ResourceCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: make an error type for this
impl<T: 'static> ResourceCollection<T> {
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            resources: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: T) -> usize {
        let id = self.interner.intern(key);

        if id >= self.resources.len() {
            self.resources.resize_with(id + 1, || None);
        }
        dbg!(key, id);
        self.resources[id] = Some(value);

        id
    }

    /// returns index or creates one if it doesnt exist
    pub fn index(&mut self, key: &str) -> usize {
        dbg!(key);
        self.interner.intern(key)
    }
    /// returns index if it exists
    pub fn index_of(&self, key: &str) -> anyhow::Result<usize> {
        self.interner.map.get(key).map(|x| *x).ok_or(anyhow!(
            "x_x :: tried to get index_of uninterned key in a resourcecollection"
        ))
    }

    /// unsafely marks an item as removed
    ///
    /// # Safety
    /// this function does not actually erase the value from memory, it just marks the index as empty to be overwritten
    /// you can still access the value behind `key` via its index (but not the string key), but it may be changed at any time
    /// which is why this function is unsafe
    pub unsafe fn remove(&mut self, key: &str) -> anyhow::Result<()> {
        self.interner.remove(key)
    }

    pub fn get(&self, index: usize) -> anyhow::Result<&T> {
        if self.resources.len() <= index {
            return Err(anyhow!(
                "x_x :: tried to get resource collection item greater that collection len!"
            ));
        }
        dbg!(index);
        self.resources[index]
            .as_ref()
            .with_context(|| "x_x :: tried to get uninitialized resource collection item")
    }
    pub fn get_mut(&mut self, index: usize) -> anyhow::Result<&mut T> {
        if self.resources.len() <= index {
            return Err(anyhow!(
                "x_x :: tried to get_mut resource collection item greater that collection len!"
            ));
        }
        Ok(self.resources[index]
            .as_mut()
            .expect("x_x :: tried to get uninitialized resource collection item"))
    }
    pub fn entry(&mut self, index: usize) -> ResourceEntry<'_, T> {
        ResourceEntry {
            id: index,
            collection: self,
        }
    }
}

impl ResourceCollection<Box<dyn BindingResource>> {
    pub fn downcast_ref<U: 'static>(&self, index: usize) -> anyhow::Result<&U> where {
        let resource = self.get(index).unwrap();
        let any = resource.as_ref() as &dyn Any;

        any.downcast_ref::<U>()
            .ok_or(anyhow!("x_x :: incorrectly downcasted resource"))
    }
    pub fn downcast_mut<U: 'static>(&mut self, index: usize) -> anyhow::Result<&mut U> {
        let resource = self.get_mut(index).unwrap();
        let any = resource.as_mut() as &mut dyn Any;

        any.downcast_mut::<U>()
            .ok_or(anyhow!("x_x :: incorrectly downcasted resource"))
    }
}

pub struct ResourceEntry<'a, T: 'static> {
    id: usize,
    collection: &'a mut ResourceCollection<T>,
}

impl<'a, T: 'static> ResourceEntry<'a, T> {
    pub fn exists(&self) -> bool {
        self.id < self.collection.resources.len()
    }

    /// Inserts created value if missing, using the closure.
    /// If the slot already exists returns &mut existing.
    pub fn or_insert_with<F: FnOnce() -> T>(self, f: F) -> &'a mut T {
        let id = self.id;
        let coll = self.collection;

        if id >= coll.resources.len() {
            coll.resources.resize_with(id + 1, || None);
            coll.resources[id] = Some(f());
        }
        coll.resources[id].as_mut().unwrap()
    }

    /// Get mutable reference if already present — None if absent.
    pub fn get_mut(self) -> Option<&'a mut T> {
        if self.id < self.collection.resources.len() {
            Some(
                self.collection.resources[self.id]
                    .as_mut()
                    .expect("x_x :: tried to get uninitialized resources item"),
            )
        } else {
            None
        }
    }
    pub fn get(self) -> Option<&'a T> {
        if self.id < self.collection.resources.len() {
            Some(
                self.collection.resources[self.id]
                    .as_ref()
                    .expect("x_x :: tried to get uninitialized resources item"),
            )
        } else {
            None
        }
    }
}*/

use std::collections::HashMap;
use std::ops::Index;

use crate::resources::texture;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct SparseEntry(usize, bool);

pub struct ResourceCollection<T> {
    dense: Vec<T>,
    dense_ids: Vec<usize>,
    sparse: Vec<SparseEntry>,

    // Allocated ONLY if string keys are used
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
