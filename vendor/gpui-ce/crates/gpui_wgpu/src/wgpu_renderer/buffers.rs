use collections::FxHashMap;
use gpui::AtlasTextureId;
use gpui_render::InstanceRange;
use gpui_render::artifacts::DATA_TEXTURE_WIDTH;
use gpui_render::shaders::interface::{self as shader_interface, BufferData};
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    num::NonZeroU64,
    ops::Range,
};

use crate::{RendererTier, WgpuTextureIdentity};

use super::pipelines::{InstanceBindingSource, WgpuBindGroupLayouts};

/// Size of one `rgba32uint` data-texture texel, the downlevel transport's unit.
const TEXEL_BYTES: u64 = 16;

/// How scene instance payloads reach the shaders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InstanceTransport {
    /// Vertex-stage read-only storage buffer (WebGPU-class devices).
    StorageBuffer,
    /// Data texture plus a per-batch dynamic-offset range uniform (WebGL2/GLES).
    DataTexture,
}

impl InstanceTransport {
    pub(super) fn from_tier(tier: RendererTier) -> Self {
        match tier {
            RendererTier::Modern => Self::StorageBuffer,
            RendererTier::WebGl2 => Self::DataTexture,
        }
    }

    /// Byte alignment between batches; the downlevel base must land on a texel.
    pub(super) fn batch_alignment(self, element_stride: u64) -> u64 {
        match self {
            Self::StorageBuffer => element_stride,
            Self::DataTexture => TEXEL_BYTES,
        }
    }
}

pub(super) struct InstanceSlice<T> {
    /// The draw arguments. On the storage-buffer transport the base indexes the whole
    /// arena; downlevel draws always start at instance zero and carry the base in
    /// their range-uniform slot instead.
    instances: InstanceRange,
    transport: InstanceTransport,
    /// Byte offset of this batch's range-uniform slot; downlevel transport only.
    range_offset: u32,
    value: PhantomData<T>,
}

impl<T> InstanceSlice<T> {
    pub(super) fn range(&self) -> Range<u32> {
        self.instances.as_range()
    }

    /// Binds the group-1 data bind group, adding the dynamic offset downlevel.
    pub(super) fn set_data_bind_group(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        bind_group: &wgpu::BindGroup,
    ) {
        match self.transport {
            InstanceTransport::StorageBuffer => {
                pass.set_bind_group(shader_interface::DATA_BIND_GROUP, bind_group, &[])
            }
            InstanceTransport::DataTexture => pass.set_bind_group(
                shader_interface::DATA_BIND_GROUP,
                bind_group,
                &[self.range_offset],
            ),
        }
    }
}

/// One frame's mapped scene-data upload: staging for all payloads plus downlevel slots.
pub(super) struct InstanceUpload {
    staging: Option<wgpu::QueueWriteBufferView>,
    cursor: u64,
    size: u64,
    transport: InstanceTransport,
    /// Downlevel only: mapped `DATA_RANGE` region and its slot allocator.
    range_upload: Option<RangeUpload>,
    /// Downlevel only: staging-to-texture copy recorded at finish.
    texture_copy: Option<TextureCopy>,
}

impl InstanceUpload {
    pub(super) fn write<T: BufferData>(&mut self, values: &[T]) -> Option<InstanceSlice<T>> {
        if values.is_empty() {
            return Some(self.empty_batch());
        }
        let stride = std::mem::size_of::<T>() as u64;
        let data = shader_interface::slice_as_bytes(values);
        let count = u32::try_from(values.len()).ok()?;
        let (offset, range_offset) = self.allocate_batch(data.len() as u64, stride, count)?;
        let staging = self
            .staging
            .as_mut()
            .expect("non-empty frame uploads have mapped staging memory");
        staging
            .slice(offset as usize..(offset + data.len() as u64) as usize)
            .copy_from_slice(data);
        Some(self.batch(offset, stride, count, range_offset))
    }

    pub(super) fn write_iter<T: BufferData>(
        &mut self,
        count: usize,
        values: impl IntoIterator<Item = T>,
    ) -> Option<InstanceSlice<T>> {
        if count == 0 {
            return Some(self.empty_batch());
        }
        let count = u32::try_from(count).ok()?;
        let stride = std::mem::size_of::<T>() as u64;
        let size = stride.checked_mul(count as u64)?;
        let (offset, range_offset) = self.allocate_batch(size, stride, count)?;
        let staging = self.staging.as_mut()?;
        let mut destination = staging.slice(offset as usize..(offset + size) as usize);
        let mut values = values.into_iter();
        for index in 0..count {
            let value = values.next()?;
            let index = index as u64;
            destination
                .slice((index * stride) as usize..((index + 1) * stride) as usize)
                .copy_from_slice(shader_interface::bytes_of(&value));
        }
        if values.next().is_some() {
            return None;
        }
        Some(self.batch(offset, stride, count, range_offset))
    }

    /// Reserves an aligned batch region, allocating its downlevel range slot.
    fn allocate_batch(&mut self, size: u64, stride: u64, elements: u32) -> Option<(u64, u32)> {
        let align = self.transport.batch_alignment(stride);
        let offset = self.cursor.next_multiple_of(align);
        if offset + size > self.size {
            return None;
        }
        self.cursor = offset + size;
        let range_offset = match self.transport {
            InstanceTransport::StorageBuffer => 0,
            InstanceTransport::DataTexture => {
                let base_texel = u32::try_from(offset / TEXEL_BYTES).ok()?;
                self.range_upload.as_mut()?.write(base_texel, elements)?
            }
        };
        Some((offset, range_offset))
    }

    /// An empty batch: no bytes, no range slot, no draw.
    fn empty_batch<T>(&self) -> InstanceSlice<T> {
        InstanceSlice {
            instances: InstanceRange::new(0..0).expect("the empty range is addressable"),
            transport: self.transport,
            range_offset: 0,
            value: PhantomData,
        }
    }

    fn batch<T>(
        &self,
        offset: u64,
        stride: u64,
        count: u32,
        range_offset: u32,
    ) -> InstanceSlice<T> {
        let first = match self.transport {
            InstanceTransport::StorageBuffer => {
                usize::try_from(offset / stride).expect("batch offsets are stride-aligned")
            }
            InstanceTransport::DataTexture => 0,
        };
        InstanceSlice {
            instances: InstanceRange::new(first..first + count as usize)
                .expect("arena capacity is bounded by the 32-bit instance index"),
            transport: self.transport,
            range_offset,
            value: PhantomData,
        }
    }

    pub(super) fn finish(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // Dropping the mapped views schedules queue writes before the next submission.
        self.staging.take();
        self.range_upload.take();
        if let Some(copy) = &self.texture_copy {
            let used_texels = self.cursor.div_ceil(TEXEL_BYTES);
            if used_texels > 0 {
                let height = used_texels.div_ceil(u64::from(DATA_TEXTURE_WIDTH));
                encoder.copy_buffer_to_texture(
                    wgpu::TexelCopyBufferInfo {
                        buffer: &copy.staging,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(DATA_TEXTURE_WIDTH * TEXEL_BYTES as u32),
                            rows_per_image: Some(height as u32),
                        },
                    },
                    copy.texture.as_image_copy(),
                    wgpu::Extent3d {
                        width: DATA_TEXTURE_WIDTH,
                        height: height as u32,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }
}

/// Mapped per-batch `DATA_RANGE` slots: batch base in texels plus element count.
///
/// The shader reads only the first two fields, but browser/WebGL downlevel
/// backends require uniform binding sizes to be 16-byte aligned.
struct RangeUpload {
    staging: wgpu::QueueWriteBufferView,
    stride: u64,
    slots: u64,
    next_slot: u64,
}

impl RangeUpload {
    fn write(&mut self, base_texel: u32, elements: u32) -> Option<u32> {
        let slot = self.next_slot;
        if slot >= self.slots {
            return None;
        }
        self.next_slot += 1;
        let offset = slot * self.stride;
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&base_texel.to_le_bytes());
        bytes[4..8].copy_from_slice(&elements.to_le_bytes());
        self.staging
            .slice(offset as usize..(offset + 16) as usize)
            .copy_from_slice(&bytes);
        u32::try_from(offset).ok()
    }
}

/// Staging-to-texture copy destination recorded when the frame upload finishes.
struct TextureCopy {
    staging: wgpu::Buffer,
    texture: wgpu::Texture,
}

/// Alignment-aware uniform storage with frame-local slot allocation.
pub(super) struct DynamicUniformBuffer<T> {
    pub(super) buffer: wgpu::Buffer,
    label: &'static str,
    stride: u64,
    capacity: u64,
    generation: u64,
    next_slot: Cell<u64>,
    staging: RefCell<Option<wgpu::QueueWriteBufferView>>,
    value: PhantomData<T>,
}

impl<T: BufferData> DynamicUniformBuffer<T> {
    pub(super) fn new(
        device: &wgpu::Device,
        label: &'static str,
        initial_capacity: u64,
        uniform_alignment: u64,
    ) -> Self {
        let stride = (std::mem::size_of::<T>() as u64).next_multiple_of(uniform_alignment);
        let buffer = create_buffer(
            device,
            label,
            stride * initial_capacity,
            wgpu::BufferUsages::UNIFORM,
        );
        Self {
            buffer,
            label,
            stride,
            capacity: initial_capacity,
            generation: 0,
            next_slot: Cell::new(0),
            staging: RefCell::new(None),
            value: PhantomData,
        }
    }

    pub(super) fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        required_capacity: u64,
        maximum_buffer_size: u64,
    ) -> bool {
        if required_capacity <= self.capacity {
            return true;
        }

        let new_capacity = required_capacity.next_power_of_two();
        let Some(new_size) = self.stride.checked_mul(new_capacity) else {
            return false;
        };
        if new_size > maximum_buffer_size || new_size > u32::MAX as u64 {
            return false;
        }

        self.buffer = create_buffer(device, self.label, new_size, wgpu::BufferUsages::UNIFORM);
        self.capacity = new_capacity;
        self.generation = self.generation.wrapping_add(1);
        true
    }

    pub(super) fn begin_upload(&self, queue: &wgpu::Queue, required_capacity: u64) -> bool {
        if required_capacity > self.capacity {
            return false;
        }
        self.next_slot.set(0);
        let Some(size) = NonZeroU64::new(self.stride * required_capacity) else {
            self.staging.take();
            return true;
        };
        let Some(mut staging) = queue.write_buffer_with(&self.buffer, 0, size) else {
            return false;
        };
        staging.slice(..).fill(0);
        self.staging.replace(Some(staging));
        true
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn finish_upload(&self) {
        self.staging.take();
    }

    pub(super) fn write(&self, value: &T) -> u32 {
        let slot = self.next_slot.get();
        assert!(
            slot < self.capacity,
            "{} capacity must be ensured before rendering",
            self.label
        );
        self.next_slot.set(slot + 1);
        let offset = slot * self.stride;
        let bytes = shader_interface::bytes_of(value);
        self.staging
            .borrow_mut()
            .as_mut()
            .expect("uniform uploads must begin before rendering")
            .slice(offset as usize..offset as usize + bytes.len())
            .copy_from_slice(bytes);
        u32::try_from(offset).expect("dynamic uniform offsets are limited to u32")
    }
}

/// One frame's zero-copy scene data arena.
pub(super) struct InstanceBufferArena {
    transport: InstanceTransport,
    storage: InstanceStorage,
    bind_group: wgpu::BindGroup,
    textured_bind_groups: RefCell<TexturedBindGroups>,
    capacity: u64,
    maximum_size: u64,
}

/// Backing store for the arena's payload bytes.
enum InstanceStorage {
    /// Read-only storage buffer consumed directly by the shaders.
    Buffer(wgpu::Buffer),
    /// Data texture fed from per-frame staging, plus range-uniform slots.
    DataTexture {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        staging: wgpu::Buffer,
        ranges: RangeUniformArena,
    },
}

/// Per-batch range slots in dynamic-offset-alignment stride, one bound per draw.
struct RangeUniformArena {
    buffer: wgpu::Buffer,
    stride: u64,
    capacity: u64,
    maximum: u64,
}

impl RangeUniformArena {
    const INITIAL_CAPACITY: u64 = 256;

    fn new(device: &wgpu::Device) -> Self {
        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let stride = alignment.max(256);
        // The whole-buffer limit constrains slot count, not the binding-size limit.
        let maximum = device.limits().max_buffer_size / stride;
        let capacity = Self::INITIAL_CAPACITY.min(maximum);
        Self {
            buffer: create_buffer(
                device,
                "downlevel_range_uniforms",
                capacity * stride,
                wgpu::BufferUsages::UNIFORM,
            ),
            stride,
            capacity,
            maximum,
        }
    }

    fn ensure_capacity(&mut self, device: &wgpu::Device, required_slots: u64) -> bool {
        if required_slots <= self.capacity {
            return true;
        }
        if required_slots > self.maximum {
            return false;
        }
        self.capacity = required_slots.next_power_of_two().min(self.maximum);
        self.buffer = create_buffer(
            device,
            "downlevel_range_uniforms",
            self.capacity * self.stride,
            wgpu::BufferUsages::UNIFORM,
        );
        true
    }

    fn binding(&self) -> wgpu::BufferBinding<'_> {
        // One full slot stays inside max_uniform_buffer_binding_size.
        wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: 0,
            size: NonZeroU64::new(self.stride),
        }
    }
}

#[derive(Default)]
struct TexturedBindGroups {
    groups: FxHashMap<
        (shader_interface::DataLayout, AtlasTextureId),
        (WgpuTextureIdentity, wgpu::BindGroup),
    >,
    path: FxHashMap<shader_interface::DataLayout, wgpu::BindGroup>,
}

impl InstanceBufferArena {
    // Large scenes grow geometrically; ordinary UI frames skip a permanent floor.
    const INITIAL_CAPACITY: u64 = 64 * 1024;

    pub(super) fn new(
        device: &wgpu::Device,
        layouts: &WgpuBindGroupLayouts,
        tier: RendererTier,
    ) -> Self {
        let transport = InstanceTransport::from_tier(tier);
        let maximum_size = match transport {
            InstanceTransport::StorageBuffer => device
                .limits()
                .max_buffer_size
                .min(device.limits().max_storage_buffer_binding_size),
            InstanceTransport::DataTexture => {
                let texels = u64::from(DATA_TEXTURE_WIDTH)
                    * u64::from(device.limits().max_texture_dimension_2d);
                (texels * TEXEL_BYTES).min(device.limits().max_buffer_size) / TEXEL_BYTES
                    * TEXEL_BYTES
            }
        };
        let capacity = Self::INITIAL_CAPACITY.min(maximum_size);
        let storage = match transport {
            InstanceTransport::StorageBuffer => InstanceStorage::Buffer(create_buffer(
                device,
                "instance_buffer",
                capacity,
                wgpu::BufferUsages::STORAGE,
            )),
            InstanceTransport::DataTexture => {
                let texture = create_data_texture(device, capacity / TEXEL_BYTES);
                let view = view_of(&texture);
                InstanceStorage::DataTexture {
                    texture,
                    view,
                    staging: create_buffer(
                        device,
                        "instance_staging",
                        capacity,
                        wgpu::BufferUsages::COPY_SRC,
                    ),
                    ranges: RangeUniformArena::new(device),
                }
            }
        };
        let bind_group = instance_bind_group(device, layouts, &storage);
        Self {
            transport,
            storage,
            bind_group,
            textured_bind_groups: RefCell::default(),
            capacity,
            maximum_size,
        }
    }

    pub(super) fn transport(&self) -> InstanceTransport {
        self.transport
    }

    pub(super) fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        layouts: &WgpuBindGroupLayouts,
        required: u64,
        required_range_slots: u64,
    ) -> bool {
        if let InstanceStorage::DataTexture { ranges, .. } = &self.storage {
            if required_range_slots > ranges.maximum {
                log::error!(
                    "scene requires {required_range_slots} instance batches, exceeding the \
                     range uniform limit of {} slots",
                    ranges.maximum
                );
                return false;
            }
        }
        let storage_grew = match &self.storage {
            InstanceStorage::Buffer(_) => required > self.capacity,
            InstanceStorage::DataTexture { ranges, .. } => {
                required > self.capacity || required_range_slots > ranges.capacity
            }
        };
        if !storage_grew {
            return true;
        }
        if required > self.maximum_size {
            log::error!(
                "scene requires {required} storage bytes, exceeding the GPU limit of {}",
                self.maximum_size
            );
            return false;
        }

        match &mut self.storage {
            InstanceStorage::Buffer(buffer) => {
                self.capacity = required.next_power_of_two().min(self.maximum_size);
                *buffer = create_buffer(
                    device,
                    "instance_buffer",
                    self.capacity,
                    wgpu::BufferUsages::STORAGE,
                );
            }
            InstanceStorage::DataTexture {
                texture,
                view,
                staging,
                ranges,
            } => {
                if required > self.capacity {
                    self.capacity = required.next_power_of_two().min(self.maximum_size);
                    *texture = create_data_texture(device, self.capacity / TEXEL_BYTES);
                    *view = view_of(texture);
                    *staging = create_buffer(
                        device,
                        "instance_staging",
                        self.capacity,
                        wgpu::BufferUsages::COPY_SRC,
                    );
                }
                ranges.ensure_capacity(device, required_range_slots);
            }
        }
        self.bind_group = instance_bind_group(device, layouts, &self.storage);
        *self.textured_bind_groups.get_mut() = TexturedBindGroups::default();
        log::info!("increased instance buffer size to {}", self.capacity);
        true
    }

    pub(super) fn begin_upload(
        &self,
        queue: &wgpu::Queue,
        required: u64,
        required_range_slots: u64,
    ) -> Option<InstanceUpload> {
        if required > self.capacity {
            return None;
        }
        let staging = if let Some(size) = NonZeroU64::new(required) {
            Some(match &self.storage {
                InstanceStorage::Buffer(buffer) => queue.write_buffer_with(buffer, 0, size)?,
                InstanceStorage::DataTexture { staging, .. } => {
                    queue.write_buffer_with(staging, 0, size)?
                }
            })
        } else {
            None
        };
        let range_upload = match &self.storage {
            InstanceStorage::DataTexture { ranges, .. } if required_range_slots > 0 => {
                Some(RangeUpload {
                    staging: queue.write_buffer_with(
                        &ranges.buffer,
                        0,
                        NonZeroU64::new(required_range_slots.min(ranges.capacity) * ranges.stride)?,
                    )?,
                    stride: ranges.stride,
                    slots: required_range_slots.min(ranges.capacity),
                    next_slot: 0,
                })
            }
            _ => None,
        };
        Some(InstanceUpload {
            staging,
            cursor: 0,
            size: required,
            transport: self.transport,
            range_upload,
            texture_copy: match &self.storage {
                InstanceStorage::DataTexture {
                    texture, staging, ..
                } => Some(TextureCopy {
                    staging: staging.clone(),
                    texture: texture.clone(),
                }),
                InstanceStorage::Buffer(_) => None,
            },
        })
    }

    pub(super) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub(super) fn textured_bind_group(
        &self,
        device: &wgpu::Device,
        layouts: &WgpuBindGroupLayouts,
        data_layout: shader_interface::DataLayout,
        texture_id: AtlasTextureId,
        texture_identity: WgpuTextureIdentity,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let mut cache = self.textured_bind_groups.borrow_mut();
        let key = (data_layout, texture_id);
        if let Some((identity, bind_group)) = cache.groups.get(&key)
            && *identity == texture_identity
        {
            return bind_group.clone();
        }

        let bind_group = layouts.create_textured_instances(
            device,
            data_layout,
            self.binding_source(),
            texture,
            sampler,
        );
        cache
            .groups
            .insert(key, (texture_identity, bind_group.clone()));
        bind_group
    }

    pub(super) fn path_bind_group(
        &self,
        device: &wgpu::Device,
        layouts: &WgpuBindGroupLayouts,
        data_layout: shader_interface::DataLayout,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let mut cache = self.textured_bind_groups.borrow_mut();
        cache
            .path
            .entry(data_layout)
            .or_insert_with(|| {
                layouts.create_textured_instances(
                    device,
                    data_layout,
                    self.binding_source(),
                    texture,
                    sampler,
                )
            })
            .clone()
    }

    pub(super) fn binding_source(&self) -> InstanceBindingSource<'_> {
        match &self.storage {
            InstanceStorage::Buffer(buffer) => InstanceBindingSource::Buffer(whole_buffer(buffer)),
            InstanceStorage::DataTexture { view, ranges, .. } => {
                InstanceBindingSource::DataTexture {
                    texture: view,
                    range_uniforms: ranges.binding(),
                }
            }
        }
    }

    pub(super) fn invalidate_texture_bindings(&self) {
        *self.textured_bind_groups.borrow_mut() = TexturedBindGroups::default();
    }
}

/// Creates the group-1 bind group for the arena's current storage.
fn instance_bind_group(
    device: &wgpu::Device,
    layouts: &WgpuBindGroupLayouts,
    storage: &InstanceStorage,
) -> wgpu::BindGroup {
    let source = match storage {
        InstanceStorage::Buffer(buffer) => InstanceBindingSource::Buffer(whole_buffer(buffer)),
        InstanceStorage::DataTexture { view, ranges, .. } => InstanceBindingSource::DataTexture {
            texture: view,
            range_uniforms: ranges.binding(),
        },
    };
    layouts.create_instances(device, source)
}

fn create_data_texture(device: &wgpu::Device, texels: u64) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("instance_data_texture"),
        size: wgpu::Extent3d {
            width: DATA_TEXTURE_WIDTH,
            height: texels.div_ceil(u64::from(DATA_TEXTURE_WIDTH)) as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn view_of(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn whole_buffer(buffer: &wgpu::Buffer) -> wgpu::BufferBinding<'_> {
    wgpu::BufferBinding {
        buffer,
        offset: 0,
        size: None,
    }
}

fn create_buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    primary_usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: primary_usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
