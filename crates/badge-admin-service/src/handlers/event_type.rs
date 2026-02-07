//! 事件类型 API 处理器
//!
//! 提供事件类型列表查询，用于规则配置时选择事件类型

use axum::{extract::State, Json};
use serde::Serialize;

use crate::{dto::ApiResponse, error::AdminError, state::AppState};

/// 事件类型 DTO
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTypeDto {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
}

/// 事件类型数据库行
#[derive(sqlx::FromRow)]
struct EventTypeRow {
    code: String,
    name: String,
    description: Option<String>,
    enabled: bool,
}

impl From<EventTypeRow> for EventTypeDto {
    fn from(row: EventTypeRow) -> Self {
        Self {
            code: row.code,
            name: row.name,
            description: row.description,
            enabled: row.enabled,
        }
    }
}

/// 获取事件类型列表
///
/// GET /api/admin/event-types
///
/// 返回所有启用的事件类型，用于规则配置
pub async fn list_event_types(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<EventTypeDto>>>, AdminError> {
    let rows = sqlx::query_as::<_, EventTypeRow>(
        r#"
        SELECT code, name, description, enabled
        FROM event_types
        WHERE enabled = true
        ORDER BY code
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<EventTypeDto> = rows.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(items)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_dto_serialization() {
        let dto = EventTypeDto {
            code: "purchase".to_string(),
            name: "购买事件".to_string(),
            description: Some("用户完成购买时触发".to_string()),
            enabled: true,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"code\":\"purchase\""));
        assert!(json.contains("\"name\":\"购买事件\""));
    }
}
