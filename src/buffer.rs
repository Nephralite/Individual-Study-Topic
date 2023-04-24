use ash::vk;
use vk_mem::Alloc;

pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    allocation: vk_mem::Allocation,
    //allocation_info: vk_mem::AllocationInfo,
}

impl Buffer {
    pub(crate) fn new(
        allocator: &vk_mem::Allocator,
        size_in_bytes: u64,
        usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Result<Buffer, vk::Result> {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            ..Default::default()
        };
        let (buffer, allocation) = unsafe {
            allocator.create_buffer(
                &ash::vk::BufferCreateInfo::builder()
                    .size(size_in_bytes)
                    .usage(usage)
                    .build(),
                &allocation_create_info,
            )?
        };
        Ok(Buffer {
            buffer,
            allocation,
            //allocation_info,
        })
    }
    pub(crate) unsafe fn fill<T: Sized>(
        &mut self,
        allocator: &vk_mem::Allocator,
        data: &[T],
    ) -> Result<(), vk::Result> {
        let data_ptr = allocator.map_memory(&mut self.allocation)? as *mut T;
        data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        allocator.unmap_memory(&mut self.allocation);
        Ok(())
    }
}
