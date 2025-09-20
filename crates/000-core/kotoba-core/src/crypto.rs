//! # 暗号化エンジン
//!
//! システム全体の暗号化・復号ロジックを提供します。
//! エンベロープ暗号化を実装し、データ暗号鍵（DEK）を
//! 鍵暗号鍵（KEK）で保護します。

use kotoba_errors::KotobaError;
use crate::types::Cid;
use crate::prelude::Principal;
use std::collections::HashMap;

/// どのようにペイロードが暗号化されているかを示す情報。
/// OcelEventに格納され、復号のレシピとして機能します。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EncryptionInfo {
    /// 使用された暗号化アルゴリズム (例: "AES-256-GCM")
    pub algorithm: String,

    /// 暗号化されたデータ暗号鍵 (DEK)。
    /// DEKはペイロードの暗号化に直接使われた鍵。
    /// このDEK自体が、各受信者の鍵暗号鍵(KEK)で暗号化されてここに格納される。
    /// キーは受信者のPrincipalId、バリューは暗号化されたDEK。
    pub encrypted_deks: HashMap<String, Vec<u8>>,
}

impl EncryptionInfo {
    /// 新しいEncryptionInfoを作成
    pub fn new(algorithm: String) -> Self {
        Self {
            algorithm,
            encrypted_deks: HashMap::new(),
        }
    }

    /// 受信者と暗号化されたDEKを追加
    pub fn add_recipient(&mut self, recipient_id: String, encrypted_dek: Vec<u8>) {
        self.encrypted_deks.insert(recipient_id, encrypted_dek);
    }

    /// 指定された受信者の暗号化されたDEKを取得
    pub fn get_encrypted_dek(&self, recipient_id: &str) -> Option<&Vec<u8>> {
        self.encrypted_deks.get(recipient_id)
    }

    /// 受信者の数を返す
    pub fn recipient_count(&self) -> usize {
        self.encrypted_deks.len()
    }
}

/// 暗号化エンジンのトレイト
pub trait CryptoEngine {
    /// データを暗号化する
    /// - plaintext: 平文データ
    /// - recipients: 受信者(Principal)のリスト。彼らの公開鍵でDEKが暗号化される。
    /// 戻り値は (暗号文, 暗号化情報)
    fn encrypt(
        &self,
        plaintext: &[u8],
        recipients: &[&Principal],
    ) -> Result<(Vec<u8>, EncryptionInfo), KotobaError>;

    /// データを復号する
    /// - ciphertext: 暗号文データ
    /// - encryption_info: 対応する暗号化情報
    /// - principal: 復号を試みる主体。彼の秘密鍵が使われる。
    fn decrypt(
        &self,
        ciphertext: &[u8],
        encryption_info: &EncryptionInfo,
        principal: &Principal,
    ) -> Result<Vec<u8>, KotobaError>;
}

/// デフォルトの暗号化エンジン実装（プレースホルダー）
pub struct DefaultCryptoEngine {
    /// アルゴリズム設定
    algorithm: String,
}

impl DefaultCryptoEngine {
    /// 新しい暗号化エンジンを作成
    pub fn new(algorithm: String) -> Self {
        Self { algorithm }
    }

    /// AES-256-GCMアルゴリズムで初期化
    pub fn new_aes256() -> Self {
        Self::new("AES-256-GCM".to_string())
    }
}

impl CryptoEngine for DefaultCryptoEngine {
    fn encrypt(
        &self,
        plaintext: &[u8],
        recipients: &[&Principal],
    ) -> Result<(Vec<u8>, EncryptionInfo), KotobaError> {
        // プレースホルダー実装
        // 実際にはAES-256-GCMなどのアルゴリズムを実装する必要がある

        if recipients.is_empty() {
            return Err(KotobaError::InvalidArgument("No recipients specified".to_string()));
        }

        // 簡易的なXOR暗号化（デモンストレーション用）
        let key = [42u8; 32]; // 固定鍵（実際には安全な鍵生成が必要）
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        for (i, &byte) in plaintext.iter().enumerate() {
            ciphertext.push(byte ^ key[i % key.len()]);
        }

        // 暗号化情報の作成
        let mut info = EncryptionInfo::new(self.algorithm.clone());
        for recipient in recipients {
            // 実際には受信者の公開鍵でDEKを暗号化する
            // ここではプレースホルダーとしてDEKをそのまま格納
            let encrypted_dek = key.to_vec();
            info.add_recipient(recipient.id.clone(), encrypted_dek);
        }

        Ok((ciphertext, info))
    }

    fn decrypt(
        &self,
        ciphertext: &[u8],
        encryption_info: &EncryptionInfo,
        principal: &Principal,
    ) -> Result<Vec<u8>, KotobaError> {
        // プレースホルダー実装
        // 実際にはAES-256-GCMなどのアルゴリズムを実装する必要がある

        // 受信者の暗号化されたDEKを取得
        let encrypted_dek = encryption_info
            .get_encrypted_dek(&principal.id)
            .ok_or_else(|| {
                KotobaError::Security(format!("No encryption key found for principal: {}", principal.id))
            })?;

        // 簡易的なXOR復号（デモンストレーション用）
        let key: [u8; 32] = encrypted_dek.clone().try_into().map_err(|_| {
            KotobaError::Security("Invalid key length".to_string())
        })?;

        let mut plaintext = Vec::with_capacity(ciphertext.len());
        for (i, &byte) in ciphertext.iter().enumerate() {
            plaintext.push(byte ^ key[i % key.len()]);
        }

        Ok(plaintext)
    }
}

impl Default for DefaultCryptoEngine {
    fn default() -> Self {
        Self::new_aes256()
    }
}

/// 暗号化のユーティリティ関数
pub mod utils {
    use super::*;
    use super::EncryptionInfo;

    /// ランダムなデータ暗号鍵（DEK）を生成
    pub fn generate_dek() -> Result<Vec<u8>, KotobaError> {
        // 実際には安全な乱数生成を使用
        let mut key = [0u8; 32];
        for byte in key.iter_mut() {
            *byte = (rand::random::<u8>()).wrapping_add(1); // 簡易的な乱数
        }
        Ok(key.to_vec())
    }

    /// 鍵暗号鍵（KEK）でDEKを暗号化
    pub fn encrypt_dek_with_kek(dek: &[u8], kek: &[u8]) -> Result<Vec<u8>, KotobaError> {
        if kek.len() != 32 {
            return Err(KotobaError::InvalidArgument("KEK must be 32 bytes".to_string()));
        }

        // 簡易的なXOR暗号化（デモンストレーション用）
        let mut encrypted = Vec::with_capacity(dek.len());
        for (i, &byte) in dek.iter().enumerate() {
            encrypted.push(byte ^ kek[i % kek.len()]);
        }

        Ok(encrypted)
    }

    /// KEKで暗号化されたDEKを復号
    pub fn decrypt_dek_with_kek(encrypted_dek: &[u8], kek: &[u8]) -> Result<Vec<u8>, KotobaError> {
        // 暗号化と復号が同じXOR操作なので、同じ関数を再利用
        encrypt_dek_with_kek(encrypted_dek, kek)
    }

    /// 暗号化情報を検証
    pub fn validate_encryption_info(info: &EncryptionInfo) -> Result<(), KotobaError> {
        if info.algorithm.is_empty() {
            return Err(KotobaError::InvalidArgument("Algorithm cannot be empty".to_string()));
        }

        if info.recipient_count() == 0 {
            return Err(KotobaError::InvalidArgument("No recipients specified".to_string()));
        }

        // 各受信者の暗号化DEKが有効かチェック
        for (recipient_id, encrypted_dek) in &info.encrypted_deks {
            if recipient_id.is_empty() {
                return Err(KotobaError::InvalidArgument("Empty recipient ID".to_string()));
            }
            if encrypted_dek.is_empty() {
                return Err(KotobaError::InvalidArgument("Empty encrypted DEK".to_string()));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Principal;
    use super::EncryptionInfo;

    #[test]
    fn test_crypto_engine_creation() {
        let engine = DefaultCryptoEngine::new("AES-256-GCM".to_string());
        assert_eq!(engine.algorithm, "AES-256-GCM");
    }

    #[test]
    fn test_default_crypto_engine() {
        let engine = DefaultCryptoEngine::default();
        assert_eq!(engine.algorithm, "AES-256-GCM");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let engine = DefaultCryptoEngine::default();

        let principal = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let plaintext = b"Hello, World! This is a test message for encryption.";

        // 暗号化
        let (ciphertext, encryption_info) = engine.encrypt(plaintext, &[&principal]).unwrap();

        assert!(!ciphertext.is_empty());
        assert_ne!(ciphertext, plaintext);
        assert_eq!(encryption_info.algorithm, "AES-256-GCM");
        assert_eq!(encryption_info.recipient_count(), 1);

        // 復号
        let decrypted = engine.decrypt(&ciphertext, &encryption_info, &principal).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_no_recipients() {
        let engine = DefaultCryptoEngine::default();
        let plaintext = b"test";

        let result = engine.encrypt(plaintext, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_principal() {
        let engine = DefaultCryptoEngine::default();

        let alice = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let bob = Principal {
            id: "user:bob".to_string(),
            attributes: HashMap::new(),
        };

        let plaintext = b"secret message";

        // Alice向けに暗号化
        let (ciphertext, encryption_info) = engine.encrypt(plaintext, &[&alice]).unwrap();

        // Bobで復号を試みる
        let result = engine.decrypt(&ciphertext, &encryption_info, &bob);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_recipients() {
        let engine = DefaultCryptoEngine::default();

        let alice = Principal {
            id: "user:alice".to_string(),
            attributes: HashMap::new(),
        };

        let bob = Principal {
            id: "user:bob".to_string(),
            attributes: HashMap::new(),
        };

        let plaintext = b"shared secret";

        // 複数受信者向けに暗号化
        let (ciphertext, encryption_info) = engine.encrypt(plaintext, &[&alice, &bob]).unwrap();

        assert_eq!(encryption_info.recipient_count(), 2);

        // Aliceで復号
        let decrypted_by_alice = engine.decrypt(&ciphertext, &encryption_info, &alice).unwrap();
        assert_eq!(decrypted_by_alice, plaintext);

        // Bobで復号
        let decrypted_by_bob = engine.decrypt(&ciphertext, &encryption_info, &bob).unwrap();
        assert_eq!(decrypted_by_bob, plaintext);
    }

    #[test]
    fn test_encryption_info_validation() {
        use utils::*;

        let mut info = EncryptionInfo::new("AES-256-GCM".to_string());
        info.add_recipient("user:alice".to_string(), vec![1, 2, 3, 4]);

        let result = validate_encryption_info(&info);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encryption_info_validation_failures() {
        use utils::*;

        // 空のアルゴリズム
        let info = EncryptionInfo::new("".to_string());
        let result = validate_encryption_info(&info);
        assert!(result.is_err());

        // 受信者なし
        let info = EncryptionInfo::new("AES-256-GCM".to_string());
        let result = validate_encryption_info(&info);
        assert!(result.is_err());

        // 空の受信者ID
        let mut info = EncryptionInfo::new("AES-256-GCM".to_string());
        info.add_recipient("".to_string(), vec![1, 2, 3]);
        let result = validate_encryption_info(&info);
        assert!(result.is_err());

        // 空の暗号化DEK
        let mut info = EncryptionInfo::new("AES-256-GCM".to_string());
        info.add_recipient("user:alice".to_string(), vec![]);
        let result = validate_encryption_info(&info);
        assert!(result.is_err());
    }

    #[test]
    fn test_utils_generate_dek() {
        use utils::*;

        let dek1 = generate_dek().unwrap();
        let dek2 = generate_dek().unwrap();

        assert_eq!(dek1.len(), 32);
        assert_eq!(dek2.len(), 32);
        // 乱数が異なることを確認（実際には同じになる可能性はあるが）
        assert_ne!(dek1, dek2);
    }

    #[test]
    fn test_utils_dek_encryption() {
        use utils::*;

        let dek = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                       17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];
        let kek = [42u8; 32];

        // DEKを暗号化
        let encrypted = encrypt_dek_with_kek(&dek, &kek).unwrap();
        assert_eq!(encrypted.len(), 32);

        // 暗号化されたDEKを復号
        let decrypted = decrypt_dek_with_kek(&encrypted, &kek).unwrap();
        assert_eq!(decrypted, dek);
    }
}
