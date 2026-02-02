//! 密码处理
//!
//! 提供密码哈希和验证功能

use bcrypt::{hash, verify, DEFAULT_COST};

use crate::error::AdminError;

/// 对密码进行哈希处理
///
/// 使用 bcrypt 算法生成密码哈希
pub fn hash_password(password: &str) -> Result<String, AdminError> {
    hash(password, DEFAULT_COST)
        .map_err(|e| AdminError::Internal(format!("密码哈希失败: {}", e)))
}

/// 验证密码
///
/// 比较明文密码与存储的哈希值
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AdminError> {
    verify(password, hash).map_err(|e| AdminError::Internal(format!("密码验证失败: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test_password_123";
        let hashed = hash_password(password).unwrap();

        assert!(verify_password(password, &hashed).unwrap());
        assert!(!verify_password("wrong_password", &hashed).unwrap());
    }
}
