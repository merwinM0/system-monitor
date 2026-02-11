use axum::{
    body::Body,
    extract::State,
    http::{header, Request, Response, StatusCode},
    middleware::Next,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

pub async fn basic_auth(
    State((valid_user, valid_pass)): State<(String, String)>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // 获取 Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let is_valid = match auth_header {
        Some(header) if header.starts_with("Basic ") => {
            let credentials = &header[6..]; // 去掉 "Basic "
            match BASE64.decode(credentials) {
                Ok(decoded) => {
                    if let Ok(pair) = String::from_utf8(decoded) {
                        let parts: Vec<&str> = pair.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            parts[0] == valid_user && parts[1] == valid_pass
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
        _ => false,
    };

    if is_valid {
        Ok(next.run(request).await)
    } else {
        // 认证失败，返回 401 并要求认证
        let response = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"System Monitor\"")
            .body(Body::from("Unauthorized"))
            .unwrap();
        Ok(response)
    }
}
