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

impl Activity for FileReadActivity {
    fn name(&self) -> &str {
        "file_read"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("FileReadActivity not implemented")
    }
}


pub struct FileWriteActivity;
impl Default for FileWriteActivity {
    fn default() -> Self { Self }
}

impl Activity for FileWriteActivity {
    fn name(&self) -> &str {
        "file_write"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("FileWriteActivity not implemented")
    }
}

pub struct FileCopyActivity;
impl Default for FileCopyActivity {
    fn default() -> Self { Self }
}

impl Activity for FileCopyActivity {
    fn name(&self) -> &str {
        "file_copy"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("FileCopyActivity not implemented")
    }
}

pub struct CsvParseActivity;
impl Default for CsvParseActivity {
    fn default() -> Self { Self }
}

impl Activity for CsvParseActivity {
    fn name(&self) -> &str {
        "csv_parse"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("CsvParseActivity not implemented")
    }
}

pub struct ZipCreateActivity;
impl Default for ZipCreateActivity {
    fn default() -> Self { Self }
}

impl Activity for ZipCreateActivity {
    fn name(&self) -> &str {
        "zip_create"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("ZipCreateActivity not implemented")
    }
}
