use sqlx::PgPool;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect("postgres://badge:badge_secret@localhost:5432/badge_db").await?;
    sqlx::query("ALTER TABLE benefits ALTER COLUMN total_stock TYPE BIGINT USING total_stock::bigint")
        .execute(&pool).await?;
    sqlx::query("ALTER TABLE benefits ALTER COLUMN remaining_stock TYPE BIGINT USING remaining_stock::bigint")
        .execute(&pool).await?;
    println!("类型修复完成");
    Ok(())
}
