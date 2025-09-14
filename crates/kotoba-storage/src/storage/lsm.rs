//! LSMツリー（Log-Structured Merge Tree）

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write, Seek};
use std::path::PathBuf;
use crate::types::*;

/// LSMエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSMEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub deleted: bool,
}

/// MemTable（メモリ内）
#[derive(Debug)]
pub struct MemTable {
    entries: BTreeMap<String, LSMEntry>,
    size_threshold: usize,
}

impl MemTable {
    pub fn new(size_threshold: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            size_threshold,
        }
    }

    /// エントリを追加
    pub fn put(&mut self, key: String, value: Vec<u8>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = LSMEntry {
            key: key.clone(),
            value,
            timestamp,
            deleted: false,
        };

        self.entries.insert(key, entry);
    }

    /// エントリを削除
    pub fn delete(&mut self, key: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = LSMEntry {
            key,
            value: Vec::new(),
            timestamp,
            deleted: true,
        };

        self.entries.insert(entry.key.clone(), entry);
    }

    /// エントリを取得
    pub fn get(&self, key: &str) -> Option<&LSMEntry> {
        self.entries.get(key)
    }

    /// サイズ閾値を超えているか
    pub fn should_flush(&self) -> bool {
        self.entries.len() >= self.size_threshold
    }

    /// 全てのエントリを取得（フラッシュ用）
    pub fn entries(&self) -> &BTreeMap<String, LSMEntry> {
        &self.entries
    }

    /// クリア
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// サイズを取得
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// SSTable（ソート済み文字列テーブル）
#[derive(Debug)]
pub struct SSTable {
    path: PathBuf,
    index: BTreeMap<String, u64>,  // key -> offset
    min_key: String,
    max_key: String,
}

impl SSTable {
    /// SSTableを作成
    pub fn create(path: PathBuf, entries: &BTreeMap<String, LSMEntry>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .map_err(|e| KotobaError::Storage(format!("Failed to create SSTable: {}", e)))?;

        let mut writer = BufWriter::new(file);
        let mut index = BTreeMap::new();
        let mut min_key = String::new();
        let mut max_key = String::new();
        let mut offset = 0u64;

        for (key, entry) in entries {
            if min_key.is_empty() {
                min_key = key.clone();
            }
            max_key = key.clone();

            index.insert(key.clone(), offset);

            let data = serde_json::to_vec(entry)
                .map_err(|e| KotobaError::Storage(format!("Serialization error: {}", e)))?;

            let size = data.len() as u32;
            writer.write_all(&size.to_le_bytes())
                .map_err(|e| KotobaError::Storage(format!("Write error: {}", e)))?;
            writer.write_all(&data)
                .map_err(|e| KotobaError::Storage(format!("Write error: {}", e)))?;

            offset += 4 + data.len() as u64;
        }

        writer.flush()
            .map_err(|e| KotobaError::Storage(format!("Flush error: {}", e)))?;

        Ok(Self {
            path,
            index,
            min_key,
            max_key,
        })
    }

    /// SSTableを読み込み
    pub fn load(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(&path)
            .map_err(|e| KotobaError::Storage(format!("Failed to open SSTable: {}", e)))?;

        let mut reader = BufReader::new(file);
        let mut index = BTreeMap::new();
        let mut min_key = String::new();
        let mut max_key = String::new();
        let mut offset = 0u64;

        loop {
            let mut size_buf = [0u8; 4];
            if reader.read_exact(&mut size_buf).is_err() {
                break;  // EOF
            }
            let size = u32::from_le_bytes(size_buf) as usize;

            let mut data = vec![0u8; size];
            reader.read_exact(&mut data)
                .map_err(|e| KotobaError::Storage(format!("Read error: {}", e)))?;

            let entry: LSMEntry = serde_json::from_slice(&data)
                .map_err(|e| KotobaError::Storage(format!("Deserialization error: {}", e)))?;

            if min_key.is_empty() {
                min_key = entry.key.clone();
            }
            max_key = entry.key.clone();

            index.insert(entry.key, offset);
            offset += 4 + size as u64;
        }

        Ok(Self {
            path,
            index,
            min_key,
            max_key,
        })
    }

    /// キーを検索
    pub fn get(&self, key: &str) -> Result<Option<LSMEntry>, Box<dyn std::error::Error>> {
        if key < self.min_key.as_str() || key > self.max_key.as_str() {
            return Ok(None);
        }

        if let Some(&offset) = self.index.get(key) {
            let file = File::open(&self.path)
                .map_err(|e| KotobaError::Storage(format!("Failed to open SSTable: {}", e)))?;

            let mut reader = BufReader::new(file);
            reader.seek(std::io::SeekFrom::Start(offset))
                .map_err(|e| KotobaError::Storage(format!("Seek error: {}", e)))?;

            let mut size_buf = [0u8; 4];
            reader.read_exact(&mut size_buf)
                .map_err(|e| KotobaError::Storage(format!("Read error: {}", e)))?;
            let size = u32::from_le_bytes(size_buf) as usize;

            let mut data = vec![0u8; size];
            reader.read_exact(&mut data)
                .map_err(|e| KotobaError::Storage(format!("Read error: {}", e)))?;

            let entry: LSMEntry = serde_json::from_slice(&data)
                .map_err(|e| KotobaError::Storage(format!("Deserialization error: {}", e)))?;

            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// 範囲クエリ
    pub fn range(&self, start: &str, end: &str) -> Result<Vec<LSMEntry>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for (_key, &offset) in self.index.range(start.to_string()..=end.to_string()) {
            let file = File::open(&self.path)
                .map_err(|e| KotobaError::Storage(format!("Failed to open SSTable: {}", e)))?;

            let mut reader = BufReader::new(file);
            reader.seek(std::io::SeekFrom::Start(offset))
                .map_err(|e| KotobaError::Storage(format!("Seek error: {}", e)))?;

            let mut size_buf = [0u8; 4];
            reader.read_exact(&mut size_buf)
                .map_err(|e| KotobaError::Storage(format!("Read error: {}", e)))?;
            let size = u32::from_le_bytes(size_buf) as usize;

            let mut data = vec![0u8; size];
            reader.read_exact(&mut data)
                .map_err(|e| KotobaError::Storage(format!("Read error: {}", e)))?;

            let entry: LSMEntry = serde_json::from_slice(&data)
                .map_err(|e| KotobaError::Storage(format!("Deserialization error: {}", e)))?;

            results.push(entry);
        }

        Ok(results)
    }
}

/// LSMツリーマネージャー
#[derive(Debug)]
pub struct LSMTree {
    memtable: MemTable,
    sstables: Vec<SSTable>,
    data_dir: PathBuf,
    next_sstable_id: u64,
}

impl LSMTree {
    pub fn new(data_dir: PathBuf, memtable_size: usize) -> Self {
        Self {
            memtable: MemTable::new(memtable_size),
            sstables: Vec::new(),
            data_dir,
            next_sstable_id: 0,
        }
    }

    /// データを書き込み
    pub fn put(&mut self, key: String, value: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.memtable.put(key, value);

        if self.memtable.should_flush() {
            self.flush()?;
        }

        Ok(())
    }

    /// データを削除
    pub fn delete(&mut self, key: String) -> Result<(), Box<dyn std::error::Error>> {
        self.memtable.delete(key);

        if self.memtable.should_flush() {
            self.flush()?;
        }

        Ok(())
    }

    /// データを読み込み
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        // まずMemTableを検索
        if let Some(entry) = self.memtable.get(key) {
            return if entry.deleted {
                Ok(None)
            } else {
                Ok(Some(entry.value.clone()))
            };
        }

        // SSTableを検索（新しい順）
        for sstable in self.sstables.iter().rev() {
            if let Some(entry) = sstable.get(key)? {
                return if entry.deleted {
                    Ok(None)
                } else {
                    Ok(Some(entry.value))
                };
            }
        }

        Ok(None)
    }

    /// MemTableをSSTableにフラッシュ
    pub fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.memtable.len() == 0 {
            return Ok(());
        }

        let sstable_path = self.data_dir.join(format!("sstable_{}.sst", self.next_sstable_id));
        self.next_sstable_id += 1;

        let sstable = SSTable::create(sstable_path, self.memtable.entries())?;
        self.sstables.push(sstable);

        self.memtable.clear();

        // 必要に応じてコンパクションを実行
        if self.sstables.len() > 5 {
            self.compact()?;
        }

        Ok(())
    }

    /// コンパクションを実行
    pub fn compact(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.sstables.len() < 2 {
            return Ok(());
        }

        // 最も古い2つのSSTableをマージ
        let old_sstable1 = self.sstables.remove(0);
        let old_sstable2 = self.sstables.remove(0);

        let mut merged_entries = BTreeMap::new();

        // 最初のSSTableのエントリを収集
        for (key, _) in &old_sstable1.index {
            if let Some(entry) = old_sstable1.get(key)? {
                merged_entries.insert(key.clone(), entry);
            }
        }

        // 2番目のSSTableのエントリをマージ
        for (key, _) in &old_sstable2.index {
            if let Some(entry) = old_sstable2.get(key)? {
                merged_entries.insert(key.clone(), entry);
            }
        }

        // 新しいSSTableを作成
        let sstable_path = self.data_dir.join(format!("sstable_{}.sst", self.next_sstable_id));
        self.next_sstable_id += 1;

        let new_sstable = SSTable::create(sstable_path, &merged_entries)?;
        self.sstables.insert(0, new_sstable);

        // 古いSSTableファイルを削除
        std::fs::remove_file(&old_sstable1.path)?;
        std::fs::remove_file(&old_sstable2.path)?;

        Ok(())
    }
}
