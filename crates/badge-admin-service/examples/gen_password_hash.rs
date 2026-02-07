//! 生成密码哈希
//!
//! 运行: cargo run -p badge-admin-service --example gen_password_hash

use bcrypt::{hash, verify};

fn main() {
    let passwords = vec![
        ("admin123", "admin"),
        ("operator123", "operator"),
        ("viewer123", "viewer"),
    ];

    for (password, user) in passwords {
        match hash(password, 12) {
            Ok(h) => {
                println!("User: {} | Password: {} | Hash: {}", user, password, h);
                // Verify the hash works
                match verify(password, &h) {
                    Ok(true) => println!("  ✓ Verification passed"),
                    Ok(false) => println!("  ✗ Verification failed"),
                    Err(e) => println!("  ✗ Error: {}", e),
                }
            }
            Err(e) => eprintln!("Error hashing {}: {}", password, e),
        }
        println!();
    }

    // Test the migration hash
    let migration_hash = "$2a$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4.F5g5Y5cPfJqzXe";
    println!("Testing migration hash for admin123...");
    match verify("admin123", migration_hash) {
        Ok(true) => println!("  ✓ Migration hash is valid for admin123"),
        Ok(false) => println!("  ✗ Migration hash is NOT valid for admin123"),
        Err(e) => println!("  ✗ Error: {}", e),
    }
}
