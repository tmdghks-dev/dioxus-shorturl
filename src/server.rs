use std::sync::OnceLock;

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use dioxus::prelude::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::Shortened;

const SCHEMA: &str = include_str!("../migrations/schema.sql");
const MAX_URL_LEN: usize = 2048;

static POOL: OnceLock<PgPool> = OnceLock::new();

fn pool() -> &'static PgPool {
    POOL.get().expect("DB pool is initialized at startup")
}

fn base_url() -> String {
    std::env::var("PUBLIC_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".into())
        .trim_end_matches('/')
        .to_string()
}

pub async fn router(app: fn() -> Element) -> Result<Router, anyhow::Error> {
    if POOL.get().is_none() {
        let url = std::env::var("DATABASE_URL")?;
        let pool = PgPoolOptions::new().max_connections(10).connect(&url).await?;
        sqlx::raw_sql(SCHEMA).execute(&pool).await?;
        let _ = POOL.set(pool);
    }

    Ok(dioxus::server::router(app).route(
        "/{id}",
        get(redirect)
            .post(reject_change)
            .put(reject_change)
            .patch(reject_change)
            .delete(reject_change),
    ))
}

/// 서버 측 입력 검증: 공백 제거 후 http/https 절대 URL만 허용.
pub fn validate(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("URL을 입력해 주세요.".into());
    }
    if raw.len() > MAX_URL_LEN {
        return Err(format!("URL은 {MAX_URL_LEN}자 이하여야 합니다."));
    }
    let parsed = url::Url::parse(raw).map_err(|_| "올바른 URL 형식이 아닙니다.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("http:// 또는 https:// 로 시작하는 URL만 사용할 수 있습니다.".into());
    }
    if parsed.host_str().map_or(true, str::is_empty) {
        return Err("호스트가 없는 URL입니다.".into());
    }
    // 원문을 그대로 저장해 경로·쿼리·프래그먼트를 보존한다.
    Ok(raw.to_string())
}

pub async fn shorten(raw: String) -> ServerFnResult<Shortened> {
    let original = validate(&raw).map_err(|message| ServerFnError::ServerError {
        message,
        code: 400,
        details: None,
    })?;

    // 번호는 DB의 IDENTITY 시퀀스가 발급한다. 동시 요청에도 nextval은 겹치지 않고,
    // PRIMARY KEY가 중복을 한 번 더 막는다. INSERT만 하므로 기존 매핑은 덮어쓸 수 없다.
    let id: i64 = sqlx::query_scalar("INSERT INTO links (original_url) VALUES ($1) RETURNING id")
        .bind(&original)
        .fetch_one(pool())
        .await
        .map_err(|e| ServerFnError::new(format!("저장에 실패했습니다: {e}")))?;

    Ok(Shortened {
        id,
        short_url: format!("{}/{id}", base_url()),
    })
}

async fn redirect(Path(id): Path<String>) -> Response {
    let Some(num) = parse_id(&id) else {
        return not_found();
    };
    let row: Option<String> = sqlx::query_scalar("SELECT original_url FROM links WHERE id = $1")
        .bind(num)
        .fetch_optional(pool())
        .await
        .unwrap_or(None);
    match row {
        // 302 + 저장된 원문 그대로의 Location 헤더 → 경로·쿼리·프래그먼트 유지
        Some(url) => (StatusCode::FOUND, [(header::LOCATION, url)]).into_response(),
        None => not_found(),
    }
}

async fn reject_change() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "GET")],
        "발급된 단축 주소는 수정하거나 삭제할 수 없습니다.",
    )
        .into_response()
}

fn parse_id(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(
            r#"<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>404 · 없는 주소</title>
<body style="font-family:system-ui;max-width:32rem;margin:4rem auto;padding:0 1rem">
<h1>404 · 존재하지 않는 단축 주소</h1>
<p>이 번호로 등록된 URL이 없습니다. 주소를 다시 확인해 주세요.</p>
<p><a href="/">새 단축 주소 만들기</a></p></body></html>"#,
        ),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_urls() {
        assert!(validate("https://example.org/a?b=1#c").is_ok());
        assert!(validate("http://example.org").is_ok());
        assert!(validate("ftp://example.org").is_err());
        assert!(validate("javascript:alert(1)").is_err());
        assert!(validate("example.org").is_err());
        assert!(validate("").is_err());
        assert_eq!(validate(" https://a.b/x#y ").unwrap(), "https://a.b/x#y");
    }

    #[test]
    fn parses_numeric_ids_only() {
        assert_eq!(parse_id("123"), Some(123));
        assert_eq!(parse_id("12a"), None);
        assert_eq!(parse_id("-1"), None);
        assert_eq!(parse_id(""), None);
        assert_eq!(parse_id("99999999999999999999999"), None);
    }
}
