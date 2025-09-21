//! Cloud activities - stub implementation
// TODO: Implement cloud activities

use kotoba_workflow::Activity;
use kotoba_workflow::ActivityError;
use std::collections::HashMap;
use serde_json::Value;

pub struct S3UploadActivity;
impl Default for S3UploadActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for S3UploadActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("S3UploadActivity not implemented")
    }
}

pub struct S3DownloadActivity;
impl Default for S3DownloadActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for S3DownloadActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("S3DownloadActivity not implemented")
    }
}

pub struct S3DeleteActivity;
impl Default for S3DeleteActivity {
    fn default() -> Self { Self }
}

#[async_trait::async_trait]
impl Activity for S3DeleteActivity {
    async fn execute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, ActivityError> {
        todo!("S3DeleteActivity not implemented")
    }
}
