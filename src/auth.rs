use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// JWT 密钥（生产环境应从环境变量读取）
const JWT_SECRET: &[u8] = b"your-secret-key-change-this-in-production";
const TOKEN_EXPIRE_HOURS: i64 = 24;

// 存储层（生产环境应使用 Redis/数据库）
pub struct AuthState {
    pub users: Vec<(String, String)>, // (username, hashed_password)
}

impl AuthState {
    pub fn new() -> Self {
        // 默认账号：admin / admin123
        let hashed = hash("admin123", DEFAULT_COST).unwrap();
        Self {
            users: vec![("admin".to_string(), hashed)],
        }
    }
}

// JWT Claims 结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // 用户名
    pub exp: i64,       // 过期时间戳
    pub iat: i64,       // 签发时间戳
}

// 登录请求
#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// 登录响应
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// 错误响应
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// 登录处理器
pub async fn login(
    State(state): State<Arc<AuthState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 查找用户并验证密码
    let user = state
        .users
        .iter()
        .find(|(u, _)| u == &req.username);

    match user {
        Some((username, hashed)) => {
            if verify(&req.password, hashed).map_err(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: "密码验证失败".to_string(),
                }))
            })? {
                // 生成 JWT
                let now = Utc::now();
                let exp = now + Duration::hours(TOKEN_EXPIRE_HOURS);
                
                let claims = Claims {
                    sub: username.clone(),
                    exp: exp.timestamp(),
                    iat: now.timestamp(),
                };

                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(JWT_SECRET),
                ).map_err(|_| {
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        error: "Token生成失败".to_string(),
                    }))
                })?;

                Ok(Json(LoginResponse {
                    token,
                    token_type: "Bearer".to_string(),
                    expires_in: TOKEN_EXPIRE_HOURS * 3600,
                }))
            } else {
                Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                    error: "密码错误".to_string(),
                })))
            }
        }
        None => Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "用户不存在".to_string(),
        }))),
    }
}

// JWT 验证提取器（用于保护路由）
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 从 Header 提取 Token
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));

        match auth_header {
            Some(token) => {
                let validation = Validation::default();
                match decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(JWT_SECRET),
                    &validation,
                ) {
                    Ok(token_data) => Ok(token_data.claims),
                    Err(_) => Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                        error: "Token无效或已过期".to_string(),
                    }))),
                }
            }
            None => Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                error: "缺少Authorization Header".to_string(),
            }))),
        }
    }
}
