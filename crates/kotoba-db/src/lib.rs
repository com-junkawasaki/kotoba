use kotoba_db_engine_memory::MemoryStorageEngine;

pub struct DB {
    engine: Box<dyn kotoba_db_core::engine::StorageEngine>,
}

impl DB {
    pub fn open_memory() -> Self {
        DB {
            engine: Box::new(MemoryStorageEngine::new()),
        }
    }
}
