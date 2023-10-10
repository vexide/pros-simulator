use std::{collections::HashMap, mem::size_of};

use wasmtime::{AsContextMut, Memory};

use super::WasmAllocator;

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
    pub fn set_address(
        &mut self,
        store: impl AsContextMut<Data = impl Send>,
        memory: Memory,
        index: i32,
        value: u32,
    ) {
        let address = self.get_address(index);
        let buffer = value.to_le_bytes();
        memory.write(store, address as usize, &buffer).unwrap();
    }
}
