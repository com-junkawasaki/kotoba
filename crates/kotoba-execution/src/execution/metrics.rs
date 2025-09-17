//! 実行メトリクス

use std::time::{Duration, Instant};

/// 実行メトリクス
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub rows_processed: usize,
    pub bytes_processed: usize,
    pub operations_count: usize,
}

impl ExecutionMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            end_time: None,
            rows_processed: 0,
            bytes_processed: 0,
            operations_count: 0,
        }
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Instant::now());
    }

    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }

    pub fn record_row(&mut self, row_size: usize) {
        self.rows_processed += 1;
        self.bytes_processed += row_size;
    }

    pub fn record_operation(&mut self) {
        self.operations_count += 1;
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self::new()
    }
}
