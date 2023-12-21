use std::mem::size_of;

use async_trait::async_trait;
use wasmtime::{AsContextMut, SharedMemory};

use super::{memory::SharedMemoryExt, HostCtx, WasmAllocator};

pub const NUM_THREAD_LOCAL_STORAGE_POINTERS: usize = 5;

// #[derive(Debug, Default)]
// pub struct ThreadLocalStorage {
//     pub tasks: HashMap<Task, TaskStorage>,
// }

// impl ThreadLocalStorage {
//     pub async fn get(
//         &mut self,
//         store: impl AsContextMut<Data = impl Send>,
//         allocator: &WasmAllocator,
//         task: Task,
//     ) -> TaskStorage {
//         if let Some(storage) = self.tasks.get(&task) {
//             return *storage;
//         }

//         let storage = TaskStorage::new(store, allocator).await;
//         self.tasks.insert(task, storage);
//         storage
//     }
// }

#[derive(Debug, Clone, Copy)]
pub struct TaskStorage {
    base_ptr: u32,
}

impl TaskStorage {
    pub async fn new(
        store: impl AsContextMut<Data = impl Send>,
        allocator: &WasmAllocator,
    ) -> Self {
        let base_ptr = allocator
            .memalign(
                store,
                std::alloc::Layout::new::<[u32; NUM_THREAD_LOCAL_STORAGE_POINTERS]>(),
            )
            .await;
        Self { base_ptr }
    }

    fn assert_in_bounds(index: i32) {
        if index < 0 || index as usize >= NUM_THREAD_LOCAL_STORAGE_POINTERS {
            panic!(
                "Thread local storage index out of bounds:\n\
                index {index} should be more than 0 and less than {NUM_THREAD_LOCAL_STORAGE_POINTERS}."
            );
        }
    }
    pub fn get_address(&self, index: i32) -> u32 {
        Self::assert_in_bounds(index);

        self.base_ptr + (index as u32 * size_of::<u32>() as u32)
    }
    pub fn get(&self, memory: SharedMemory, index: i32) -> u32 {
        Self::assert_in_bounds(index);
        let address = self.get_address(index);
        let buffer = memory
            .read_relaxed(address as usize, size_of::<u32>())
            .unwrap();
        u32::from_le_bytes(buffer.try_into().unwrap())
    }
    pub fn set(&mut self, memory: SharedMemory, index: i32, value: u32) {
        Self::assert_in_bounds(index);
        let address = self.get_address(index);
        let buffer = value.to_le_bytes();
        memory.write_relaxed(address as usize, &buffer).unwrap();
    }
}

#[async_trait]
pub trait GetTaskStorage {
    async fn task_storage(&mut self, task_handle: u32) -> TaskStorage;
}

#[async_trait]
impl<T, D> GetTaskStorage for T
where
    T: HostCtx + wasmtime::AsContextMut<Data = D> + Send,
    D: Send,
{
    async fn task_storage(&mut self, task_handle: u32) -> TaskStorage {
        let task = self
            .tasks_lock()
            .await
            .by_id(task_handle)
            .expect("invalid task handle");

        let mut task = task.lock().await;
        task.local_storage(self).await
    }
}
