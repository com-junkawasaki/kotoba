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

impl Activity for S3UploadActivity {
    fn name(&self) -> &str {
        "s3_upload"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("S3UploadActivity not implemented")
    }
}


pub struct S3DownloadActivity;
impl Default for S3DownloadActivity {
    fn default() -> Self { Self }
}

impl Activity for S3DownloadActivity {
    fn name(&self) -> &str {
        "s3_download"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("S3DownloadActivity not implemented")
    }
}


pub struct S3DeleteActivity;
impl Default for S3DeleteActivity {
    fn default() -> Self { Self }
}

impl Activity for S3DeleteActivity {
    fn name(&self) -> &str {
        "s3_delete"
    }

    async fn execute(&self, _inputs: HashMap<String, Value>) -> std::result::Result<HashMap<String, Value>, ActivityError> {
        todo!("S3DeleteActivity not implemented")
    }
}

