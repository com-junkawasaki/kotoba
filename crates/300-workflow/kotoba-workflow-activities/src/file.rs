//! File activities - stub implementation
// TODO: Implement file activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct FileReadActivity;
impl Default for FileReadActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for FileReadActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("FileReadActivity not implemented")
    }
}

pub struct FileWriteActivity;
impl Default for FileWriteActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for FileWriteActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("FileWriteActivity not implemented")
    }
}

pub struct FileCopyActivity;
impl Default for FileCopyActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for FileCopyActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("FileCopyActivity not implemented")
    }
}

pub struct CsvParseActivity;
impl Default for CsvParseActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for CsvParseActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("CsvParseActivity not implemented")
    }
}

pub struct ZipCreateActivity;
impl Default for ZipCreateActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for ZipCreateActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("ZipCreateActivity not implemented")
    }
}
