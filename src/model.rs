use crate::buffer::Buffer;
use ash::vk;
use obj::Position;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub(crate) struct InstanceData {
    pub(crate) modelmatrix: [[f32; 4]; 4],
    pub(crate) colour: [f32; 3],
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct VertexData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Debug, Clone)]
struct InvalidHandle;
impl std::fmt::Display for InvalidHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "invalid handle")
    }
}
impl std::error::Error for InvalidHandle {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

pub(crate) struct Model<V, I> {
    vertexdata: Vec<V>,
    handle_to_index: std::collections::HashMap<usize, usize>,
    handles: Vec<usize>,
    instances: Vec<I>,
    first_invisible: usize,
    next_handle: usize,
    vertexbuffer: Option<Buffer>,
    instancebuffer: Option<Buffer>,
}
impl<V, I> Model<V, I> {
    fn get(&self, handle: usize) -> Option<&I> {
        if let Some(&index) = self.handle_to_index.get(&handle) {
            self.instances.get(index)
        } else {
            None
        }
    }
    fn get_mut(&mut self, handle: usize) -> Option<&mut I> {
        if let Some(&index) = self.handle_to_index.get(&handle) {
            self.instances.get_mut(index)
        } else {
            None
        }
    }
    fn is_visible(&self, handle: usize) -> Result<bool, InvalidHandle> {
        if let Some(index) = self.handle_to_index.get(&handle) {
            Ok(index < &self.first_invisible)
        } else {
            Err(InvalidHandle)
        }
    }
    fn make_visible(&mut self, handle: usize) -> Result<(), InvalidHandle> {
        //if already visible: do nothing
        if let Some(&index) = self.handle_to_index.get(&handle) {
            if index < self.first_invisible {
                return Ok(());
            }
            //else: move to position first_invisible and increase value of first_invisible
            self.swap_by_index(index, self.first_invisible);
            self.first_invisible += 1;
            Ok(())
        } else {
            Err(InvalidHandle)
        }
    }
    fn make_invisible(&mut self, handle: usize) -> Result<(), InvalidHandle> {
        //if already invisible: do nothing
        if let Some(&index) = self.handle_to_index.get(&handle) {
            if index >= self.first_invisible {
                return Ok(());
            }
            //else: move to position before first_invisible and decrease value of first_invisible
            self.swap_by_index(index, self.first_invisible - 1);
            self.first_invisible -= 1;
            Ok(())
        } else {
            Err(InvalidHandle)
        }
    }
    fn insert(&mut self, element: I) -> usize {
        let handle = self.next_handle;
        self.next_handle += 1;
        let index = self.instances.len();
        self.instances.push(element);
        self.handles.push(handle);
        self.handle_to_index.insert(handle, index);
        handle
    }
    pub(crate) fn insert_visibly(&mut self, element: I) -> usize {
        let new_handle = self.insert(element);
        self.make_visible(new_handle).ok();
        new_handle
    }
    fn remove(&mut self, handle: usize) -> Result<I, InvalidHandle> {
        if let Some(&index) = self.handle_to_index.get(&handle) {
            if index < self.first_invisible {
                self.swap_by_index(index, self.first_invisible - 1);
                self.first_invisible -= 1;
            }
            self.swap_by_index(self.first_invisible, self.instances.len() - 1);
            self.handles.pop();
            self.handle_to_index.remove(&handle);
            //must be Some(), otherwise we couldn't have found an index
            Ok(self.instances.pop().unwrap())
        } else {
            Err(InvalidHandle)
        }
    }
    fn swap_by_handle(&mut self, handle1: usize, handle2: usize) -> Result<(), InvalidHandle> {
        if handle1 == handle2 {
            return Ok(());
        }
        if let (Some(&index1), Some(&index2)) = (
            self.handle_to_index.get(&handle1),
            self.handle_to_index.get(&handle2),
        ) {
            self.handles.swap(index1, index2);
            self.instances.swap(index1, index2);
            self.handle_to_index.insert(index1, handle2);
            self.handle_to_index.insert(index2, handle1);
            Ok(())
        } else {
            Err(InvalidHandle)
        }
    }
    fn swap_by_index(&mut self, index1: usize, index2: usize) {
        if index1 == index2 {
            return;
        }
        let handle1 = self.handles[index1];
        let handle2 = self.handles[index2];
        self.handles.swap(index1, index2);
        self.instances.swap(index1, index2);
        self.handle_to_index.insert(index1, handle2);
        self.handle_to_index.insert(index2, handle1);
    }
    pub(crate) fn update_vertexbuffer(
        &mut self,
        allocator: &vk_mem::Allocator,
    ) -> Result<(), vk::Result> {
        if let Some(buffer) = &mut self.vertexbuffer {
            unsafe { buffer.fill(allocator, &self.vertexdata)? };
            Ok(())
        } else {
            let bytes = (self.vertexdata.len() * std::mem::size_of::<V>()) as u64;
            let mut buffer = Buffer::new(
                &allocator,
                bytes,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk_mem::MemoryUsage::CpuToGpu,
            )?;
            unsafe { buffer.fill(allocator, &self.vertexdata)? };
            self.vertexbuffer = Some(buffer);
            Ok(())
        }
    }
    pub(crate) fn update_instancebuffer(
        &mut self,
        allocator: &vk_mem::Allocator,
    ) -> Result<(), vk::Result> {
        if let Some(buffer) = &mut self.instancebuffer {
            unsafe { buffer.fill(allocator, &self.instances[0..self.first_invisible])? };
            Ok(())
        } else {
            let bytes = (self.first_invisible * std::mem::size_of::<I>()) as u64;
            let mut buffer = Buffer::new(
                &allocator,
                bytes,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk_mem::MemoryUsage::CpuToGpu,
            )?;
            unsafe { buffer.fill(allocator, &self.instances[0..self.first_invisible])? };
            self.instancebuffer = Some(buffer);
            Ok(())
        }
    }
    pub(crate) fn draw(&self, logical_device: &ash::Device, commandbuffer: vk::CommandBuffer) {
        if let Some(vertexbuffer) = &self.vertexbuffer {
            if let Some(instancebuffer) = &self.instancebuffer {
                if self.first_invisible > 0 {
                    unsafe {
                        logical_device.cmd_bind_vertex_buffers(
                            commandbuffer,
                            0,
                            &[vertexbuffer.buffer],
                            &[0],
                        );
                        logical_device.cmd_bind_vertex_buffers(
                            commandbuffer,
                            1,
                            &[instancebuffer.buffer],
                            &[0],
                        );
                        logical_device.cmd_draw(
                            commandbuffer,
                            self.vertexdata.len() as u32,
                            self.first_invisible as u32,
                            0,
                            0,
                        );
                    }
                }
            }
        }
    }
}
impl Model<[f32; 3], InstanceData> {
    pub(crate) fn cube() -> Model<[f32; 3], InstanceData> {
        let lbf = [-1.0, 1.0, -1.0]; //lbf: left-bottom-front
        let lbb = [-1.0, 1.0, 1.0];
        let ltf = [-1.0, -1.0, -1.0];
        let ltb = [-1.0, -1.0, 1.0];
        let rbf = [1.0, 1.0, -1.0];
        let rbb = [1.0, 1.0, 1.0];
        let rtf = [1.0, -1.0, -1.0];
        let rtb = [1.0, -1.0, 1.0];
        Model {
            vertexdata: vec![
                lbf, lbb, rbb, lbf, rbb, rbf, //bottom
                ltf, rtb, ltb, ltf, rtf, rtb, //top
                lbf, rtf, ltf, lbf, rbf, rtf, //front
                lbb, ltb, rtb, lbb, rtb, rbb, //back
                lbf, ltf, lbb, lbb, ltf, ltb, //left
                rbf, rbb, rtf, rbb, rtb, rtf, //right
            ],
            handle_to_index: std::collections::HashMap::new(),
            handles: Vec::new(),
            instances: Vec::new(),
            first_invisible: 0,
            next_handle: 0,
            vertexbuffer: None,
            instancebuffer: None,
        }
    }
    pub(crate) fn object(filepath: &str) -> Model<[f32; 3], InstanceData> {
        let mut reader = std::io::BufReader::new(std::fs::File::open(filepath).expect("404"));
        let converted: obj::Obj<Position> = obj::load_obj(reader).unwrap();
        let pos_to_vec: Vec<[f32; 3]> = converted.vertices.iter().map(|p| p.position).collect();
        Model {
            vertexdata: pos_to_vec,
            handle_to_index: std::collections::HashMap::new(),
            handles: Vec::new(),
            instances: Vec::new(),
            first_invisible: 0,
            next_handle: 0,
            vertexbuffer: None,
            instancebuffer: None,
        }
    }
}
