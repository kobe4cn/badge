//! 字段级数据加密模块
//!
//! 基于 AES-256-GCM 实现字段级加密，满足 specs/instructions.md 3.3 条款要求：
//! "敏感数据在存储和日志中需加密或脱敏，不得以明文保存。"
//!
//! ## 设计决策
//!
//! - **AES-256-GCM**：AEAD 算法同时提供加密和完整性验证，防止密文被篡改
//! - **随机 Nonce**：每次加密生成独立的 12 字节 nonce，相同明文产生不同密文
//! - **Passthrough 模式**：未配置密钥时不加密，方便开发环境使用
//! - **线程安全**：FieldEncryptor 实现 Send + Sync，可安全跨线程共享

use aes_gcm::{
    Aes256Gcm, AeadCore, KeyInit,
    aead::Aead,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use thiserror::Error;

/// 加密模块错误类型
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("加密失败: {0}")]
    EncryptionFailed(String),

    #[error("解密失败: {0}")]
    DecryptionFailed(String),

    #[error("无效的密钥长度: 预期 32 字节, 实际 {0} 字节")]
    InvalidKeyLength(usize),

    #[error("无效的密文格式: {0}")]
    InvalidCiphertext(String),

    #[error("JSON 序列化失败: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// AES-256-GCM Nonce 长度（12 字节，GCM 标准推荐值）
const NONCE_SIZE: usize = 12;

/// 字段级加密器
///
/// 支持两种运行模式：
/// - **加密模式**：持有 AES-256-GCM 密钥，对数据进行真实加密
/// - **Passthrough 模式**：未配置密钥时，encrypt/decrypt 直接透传原文，
///   确保开发环境无需配置密钥即可运行
///
/// 输出格式: `base64(nonce[12] || ciphertext || tag[16])`
#[derive(Clone)]
pub struct FieldEncryptor {
    /// None 表示 passthrough 模式（开发环境不加密）
    cipher: Option<Aes256Gcm>,
}

// 编译期验证 Send + Sync，确保可安全注入到 Axum 的 Arc<AppState> 中
const _: () = {
    fn _assert_send_sync<T: Send + Sync>() {}
    fn _check() {
        _assert_send_sync::<FieldEncryptor>();
    }
};

impl FieldEncryptor {
    /// 从 32 字节密钥创建加密器
    ///
    /// 密钥长度必须恰好 32 字节（256 位），不接受其他长度以避免
    /// 意外使用弱密钥。
    pub fn new(key: &[u8]) -> Result<Self, CryptoError> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key.len()));
        }
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        Ok(Self {
            cipher: Some(cipher),
        })
    }

    /// 创建 passthrough 模式的加密器（不执行加密）
    ///
    /// 用于开发环境——未配置 BADGE_ENCRYPTION_KEY 时自动降级为此模式，
    /// 日志中会输出警告提示。
    pub fn passthrough() -> Self {
        Self { cipher: None }
    }

    /// 从 hex 编码的密钥字符串创建加密器
    ///
    /// 生产环境通过 `BADGE_ENCRYPTION_KEY` 环境变量传入 64 字符的 hex 字符串。
    pub fn from_hex(hex_key: &str) -> Result<Self, CryptoError> {
        let bytes = hex_decode(hex_key).map_err(|_| CryptoError::InvalidKeyLength(0))?;
        Self::new(&bytes)
    }

    /// 是否处于加密模式（非 passthrough）
    pub fn is_enabled(&self) -> bool {
        self.cipher.is_some()
    }

    /// 加密字符串
    ///
    /// 返回 `base64(nonce || ciphertext || tag)` 格式的密文。
    /// Passthrough 模式下直接返回原文。
    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let Some(ref cipher) = self.cipher else {
            return Ok(plaintext.to_string());
        };

        let nonce = Aes256Gcm::generate_nonce(&mut aes_gcm::aead::OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        // 将 nonce 和密文拼接后统一 base64 编码，解密时按固定偏移拆分
        let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// 解密字符串
    ///
    /// 输入为 `encrypt()` 返回的 base64 编码密文。
    /// Passthrough 模式下直接返回原文。
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, CryptoError> {
        let Some(ref cipher) = self.cipher else {
            return Ok(ciphertext.to_string());
        };

        let combined = BASE64
            .decode(ciphertext)
            .map_err(|e| CryptoError::InvalidCiphertext(format!("base64 解码失败: {e}")))?;

        if combined.len() < NONCE_SIZE {
            return Err(CryptoError::InvalidCiphertext(format!(
                "密文过短: 至少需要 {NONCE_SIZE} 字节 nonce，实际 {} 字节",
                combined.len()
            )));
        }

        let (nonce_bytes, ciphertext_bytes) = combined.split_at(NONCE_SIZE);
        let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext_bytes)
            .map_err(|_| CryptoError::DecryptionFailed("密文损坏或密钥不匹配".to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| CryptoError::DecryptionFailed(format!("解密结果非 UTF-8: {e}")))
    }

    /// 加密 JSON Value
    ///
    /// 先将 JSON 序列化为字符串，再加密。存储时原 JSONB 列改为存储加密字符串。
    pub fn encrypt_json(&self, value: &serde_json::Value) -> Result<String, CryptoError> {
        let json_str = serde_json::to_string(value)?;
        self.encrypt(&json_str)
    }

    /// 解密为 JSON Value
    ///
    /// 解密后将字符串反序列化为 JSON Value，用于 JSONB 字段读取。
    pub fn decrypt_json(&self, ciphertext: &str) -> Result<serde_json::Value, CryptoError> {
        let json_str = self.decrypt(ciphertext)?;
        let value = serde_json::from_str(&json_str)?;
        Ok(value)
    }
}

/// 将 hex 字符串解码为字节数组
///
/// 不依赖外部 crate，手动实现避免引入额外依赖。
fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err(format!("hex 字符串长度必须为偶数，实际 {}", hex.len()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("位置 {i} 处无效的 hex 字符: {e}"))
        })
        .collect()
}

// ============================================================
// 脱敏辅助函数
// ============================================================

/// 邮箱脱敏：保留首字符和 @ 后域名
///
/// 示例: `kevin@example.com` -> `k***@example.com`
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            if local.is_empty() {
                return format!("***@{domain}");
            }
            let first_char: String = local.chars().next().unwrap().to_string();
            format!("{first_char}***@{domain}")
        }
        // 格式不合法时全部遮蔽
        None => "***".to_string(),
    }
}

/// 手机号脱敏：保留前 3 位和后 4 位
///
/// 示例: `13812345678` -> `138****5678`
/// 不足 7 位的短号码全部遮蔽，防止反推原始号码。
pub fn mask_phone(phone: &str) -> String {
    let digits: Vec<char> = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 7 {
        return "****".to_string();
    }
    let prefix: String = digits[..3].iter().collect();
    let suffix: String = digits[digits.len() - 4..].iter().collect();
    format!("{prefix}****{suffix}")
}

/// IP 地址脱敏：保留前两段
///
/// 示例: `192.168.1.100` -> `192.168.*.*`
/// IPv6 地址仅保留前两组。
pub fn mask_ip(ip: &str) -> String {
    // IPv4
    if ip.contains('.') {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            return format!("{}.{}.*.*", parts[0], parts[1]);
        }
    }
    // IPv6 或其他格式：保留前两组
    if ip.contains(':') {
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.len() >= 2 {
            return format!("{}:{}:*:*", parts[0], parts[1]);
        }
    }
    // 无法识别的格式全部遮蔽
    "***".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 生成用于测试的固定 32 字节密钥
    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = i as u8;
        }
        key
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let encryptor = FieldEncryptor::new(&test_key()).unwrap();
        let plaintext = "hello, 加密世界!";

        let encrypted = encryptor.encrypt(plaintext).unwrap();
        // 密文应与明文不同（base64 编码后的格式）
        assert_ne!(encrypted, plaintext);

        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_nonces_produce_different_ciphertexts() {
        let encryptor = FieldEncryptor::new(&test_key()).unwrap();
        let plaintext = "same input";

        let c1 = encryptor.encrypt(plaintext).unwrap();
        let c2 = encryptor.encrypt(plaintext).unwrap();

        // 随机 nonce 保证相同明文产生不同密文，防止密文比对泄露相等关系
        assert_ne!(c1, c2);

        // 但两者解密后应还原为相同明文
        assert_eq!(encryptor.decrypt(&c1).unwrap(), plaintext);
        assert_eq!(encryptor.decrypt(&c2).unwrap(), plaintext);
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let key1 = test_key();
        let mut key2 = test_key();
        key2[0] = 0xFF; // 篡改一个字节

        let encryptor1 = FieldEncryptor::new(&key1).unwrap();
        let encryptor2 = FieldEncryptor::new(&key2).unwrap();

        let encrypted = encryptor1.encrypt("secret data").unwrap();
        let result = encryptor2.decrypt(&encrypted);

        // GCM 的认证标签校验会失败
        assert!(result.is_err());
    }

    #[test]
    fn invalid_key_length_rejected() {
        let short_key = [0u8; 16]; // AES-128 长度，不允许
        assert!(FieldEncryptor::new(&short_key).is_err());

        let long_key = [0u8; 64];
        assert!(FieldEncryptor::new(&long_key).is_err());
    }

    #[test]
    fn passthrough_mode() {
        let encryptor = FieldEncryptor::passthrough();
        assert!(!encryptor.is_enabled());

        let plaintext = "no encryption here";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        // passthrough 模式下密文就是明文
        assert_eq!(encrypted, plaintext);

        let decrypted = encryptor.decrypt(plaintext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn json_encrypt_decrypt_roundtrip() {
        let encryptor = FieldEncryptor::new(&test_key()).unwrap();
        let value = serde_json::json!({
            "coupon_code": "SAVE20",
            "address": "北京市朝阳区",
            "amount": 99.5
        });

        let encrypted = encryptor.encrypt_json(&value).unwrap();
        let decrypted = encryptor.decrypt_json(&encrypted).unwrap();

        assert_eq!(decrypted, value);
    }

    #[test]
    fn from_hex_key() {
        // 64 个 hex 字符 = 32 字节
        let hex_key = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let encryptor = FieldEncryptor::from_hex(hex_key).unwrap();
        assert!(encryptor.is_enabled());

        let encrypted = encryptor.encrypt("test").unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "test");
    }

    #[test]
    fn from_hex_invalid() {
        // 长度不对
        assert!(FieldEncryptor::from_hex("0011").is_err());
        // 非 hex 字符
        assert!(FieldEncryptor::from_hex("zzzz").is_err());
    }

    #[test]
    fn invalid_ciphertext_rejected() {
        let encryptor = FieldEncryptor::new(&test_key()).unwrap();

        // 非 base64 字符串
        assert!(encryptor.decrypt("not-valid-base64!!!").is_err());

        // 过短的 base64（解码后不足 12 字节 nonce）
        assert!(encryptor.decrypt(&BASE64.encode([0u8; 5])).is_err());

        // 被篡改的密文（GCM tag 校验失败）
        let encrypted = encryptor.encrypt("test").unwrap();
        let mut bytes = BASE64.decode(&encrypted).unwrap();
        if let Some(last) = bytes.last_mut() {
            *last ^= 0xFF;
        }
        let tampered = BASE64.encode(&bytes);
        assert!(encryptor.decrypt(&tampered).is_err());
    }

    #[test]
    fn empty_string_encrypt_decrypt() {
        let encryptor = FieldEncryptor::new(&test_key()).unwrap();
        let encrypted = encryptor.encrypt("").unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "");
    }

    // ==================== 脱敏函数测试 ====================

    #[test]
    fn test_mask_email() {
        assert_eq!(mask_email("kevin@example.com"), "k***@example.com");
        assert_eq!(mask_email("a@b.com"), "a***@b.com");
        assert_eq!(mask_email("@domain.com"), "***@domain.com");
        assert_eq!(mask_email("no-at-sign"), "***");
    }

    #[test]
    fn test_mask_phone() {
        assert_eq!(mask_phone("13812345678"), "138****5678");
        // 带国际区号时，过滤非数字后为 8613812345678，前3后4脱敏
        assert_eq!(mask_phone("+8613812345678"), "861****5678");
        assert_eq!(mask_phone("123"), "****"); // 太短，全部遮蔽
    }

    #[test]
    fn test_mask_ip() {
        assert_eq!(mask_ip("192.168.1.100"), "192.168.*.*");
        assert_eq!(mask_ip("10.0.0.1"), "10.0.*.*");
        assert_eq!(mask_ip("2001:0db8:85a3::8a2e"), "2001:0db8:*:*");
        assert_eq!(mask_ip("invalid"), "***");
    }
}
