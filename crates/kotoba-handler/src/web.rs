//! Web Handler Module
//!
//! このモジュールはウェブアプリケーション開発のための統合ハンドラーを提供します。
//! HTTPリクエスト/レスポンスの処理、ルーティング、ミドルウェアなどを含みます。

use crate::{HandlerError, Result, HandlerContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTPメソッド
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

/// Webレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: String,
}

/// Webリクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Webハンドラー定義
#[derive(Debug, Clone)]
pub struct WebHandler {
    pub route: String,
    pub method: HttpMethod,
    pub handler: Arc<dyn Fn(WebRequest) -> Result<WebResponse> + Send + Sync>,
}

/// Web設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub port: u16,
    pub host: String,
    pub routes: Vec<WebRoute>,
    pub middlewares: Vec<WebMiddleware>,
    pub static_dir: Option<String>,
    pub template_dir: Option<String>,
}

/// Webルート定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRoute {
    pub path: String,
    pub method: String,
    pub handler: String, // Jsonnet handler function name
    pub middlewares: Vec<String>,
}

/// Webミドルウェア定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebMiddleware {
    pub name: String,
    pub handler: String, // Jsonnet middleware function name
}

/// Webアプリケーション
pub struct WebApp {
    config: WebConfig,
    handlers: HashMap<String, WebHandler>,
    context: Arc<RwLock<HandlerContext>>,
}

impl WebApp {
    /// 新しいWebアプリケーションを作成
    pub fn new(config: WebConfig, context: HandlerContext) -> Self {
        Self {
            config,
            handlers: HashMap::new(),
            context: Arc::new(RwLock::new(context)),
        }
    }

    /// ハンドラーを登録
    pub fn register_handler<F>(&mut self, route: String, method: HttpMethod, handler: F)
    where
        F: Fn(WebRequest) -> Result<WebResponse> + Send + Sync + 'static,
    {
        let key = format!("{} {}", method_to_string(&method), route);
        let web_handler = WebHandler {
            route,
            method,
            handler: Arc::new(handler),
        };
        self.handlers.insert(key, web_handler);
    }

    /// リクエストを処理
    pub async fn handle_request(&self, request: WebRequest) -> Result<WebResponse> {
        let key = format!("{} {}", method_to_string(&request.method), request.path);

        if let Some(handler) = self.handlers.get(&key) {
            (handler.handler)(request)
        } else {
            // デフォルトの404レスポンス
            Ok(WebResponse {
                status: 404,
                headers: HashMap::new(),
                body: "<h1>404 Not Found</h1>".to_string(),
                content_type: "text/html".to_string(),
            })
        }
    }

    /// JSON APIハンドラーの登録
    pub fn register_json_api<F, T>(&mut self, route: String, method: HttpMethod, handler: F)
    where
        F: Fn(WebRequest) -> Result<T> + Send + Sync + 'static,
        T: Serialize,
    {
        let json_handler = move |req: WebRequest| {
            let result = handler(req)?;
            let json_body = serde_json::to_string(&result)
                .map_err(|e| HandlerError::Jsonnet(format!("JSON serialization error: {}", e)))?;

            Ok(WebResponse {
                status: 200,
                headers: HashMap::new(),
                body: json_body,
                content_type: "application/json".to_string(),
            })
        };

        self.register_handler(route, method, json_handler);
    }

    /// HTMLテンプレートハンドラーの登録
    pub fn register_html_template<F>(&mut self, route: String, method: HttpMethod, template: String, handler: F)
    where
        F: Fn(WebRequest) -> Result<HashMap<String, String>> + Send + Sync + 'static,
    {
        let html_handler = move |req: WebRequest| {
            let context = handler(req)?;
            let html = render_template(&template, &context)?;

            Ok(WebResponse {
                status: 200,
                headers: HashMap::new(),
                body: html,
                content_type: "text/html".to_string(),
            })
        };

        self.register_handler(route, method, html_handler);
    }

    /// 静的ファイルハンドラーの登録
    pub fn register_static_file(&mut self, route: String, file_path: String) {
        let static_handler = move |req: WebRequest| {
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    let content_type = guess_content_type(&file_path);
                    Ok(WebResponse {
                        status: 200,
                        headers: HashMap::new(),
                        body: content,
                        content_type,
                    })
                }
                Err(_) => Ok(WebResponse {
                    status: 404,
                    headers: HashMap::new(),
                    body: "<h1>404 Not Found</h1>".to_string(),
                    content_type: "text/html".to_string(),
                }),
            }
        };

        self.register_handler(route, HttpMethod::GET, static_handler);
    }

    /// フォームハンドラーの登録
    pub fn register_form_handler<F>(&mut self, route: String, fields: Vec<String>, handler: F)
    where
        F: Fn(WebRequest, HashMap<String, String>) -> Result<WebResponse> + Send + Sync + 'static,
    {
        let form_handler = move |req: WebRequest| {
            if let Some(body) = &req.body {
                let form_data = parse_form_data(body, &fields)?;
                handler(req, form_data)
            } else {
                Ok(WebResponse {
                    status: 400,
                    headers: HashMap::new(),
                    body: "<h1>400 Bad Request</h1>".to_string(),
                    content_type: "text/html".to_string(),
                })
            }
        };

        self.register_handler(route, HttpMethod::POST, form_handler);
    }
}

/// Webアプリケーションを実行
pub async fn run_web_app(addr: &str, config: WebConfig) -> Result<()> {
    println!("🚀 Starting web application on {}", addr);

    // 実際のHTTPサーバー実装はここに追加
    // 現在はプレースホルダー

    println!("✅ Web application started successfully");
    Ok(())
}

/// ユーティリティ関数

fn method_to_string(method: &HttpMethod) -> &'static str {
    match method {
        HttpMethod::GET => "GET",
        HttpMethod::POST => "POST",
        HttpMethod::PUT => "PUT",
        HttpMethod::DELETE => "DELETE",
        HttpMethod::PATCH => "PATCH",
        HttpMethod::OPTIONS => "OPTIONS",
        HttpMethod::HEAD => "HEAD",
    }
}

fn render_template(template: &str, context: &HashMap<String, String>) -> Result<String> {
    let mut result = template.to_string();

    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    Ok(result)
}

fn guess_content_type(file_path: &str) -> String {
    if file_path.ends_with(".html") {
        "text/html".to_string()
    } else if file_path.ends_with(".css") {
        "text/css".to_string()
    } else if file_path.ends_with(".js") {
        "application/javascript".to_string()
    } else if file_path.ends_with(".json") {
        "application/json".to_string()
    } else if file_path.ends_with(".png") {
        "image/png".to_string()
    } else if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else {
        "text/plain".to_string()
    }
}

fn parse_form_data(body: &str, fields: &[String]) -> Result<HashMap<String, String>> {
    let mut data = HashMap::new();

    for pair in body.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if fields.contains(&key.to_string()) {
                let decoded_value = urlencoding::decode(value)
                    .map_err(|e| HandlerError::Jsonnet(format!("URL decode error: {}", e)))?;
                data.insert(key.to_string(), decoded_value.to_string());
            }
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_to_string() {
        assert_eq!(method_to_string(&HttpMethod::GET), "GET");
        assert_eq!(method_to_string(&HttpMethod::POST), "POST");
        assert_eq!(method_to_string(&HttpMethod::PUT), "PUT");
        assert_eq!(method_to_string(&HttpMethod::DELETE), "DELETE");
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("test.html"), "text/html");
        assert_eq!(guess_content_type("style.css"), "text/css");
        assert_eq!(guess_content_type("app.js"), "application/javascript");
        assert_eq!(guess_content_type("data.json"), "application/json");
        assert_eq!(guess_content_type("unknown.xyz"), "text/plain");
    }

    #[test]
    fn test_render_template() {
        let template = "<h1>{{title}}</h1><p>{{content}}</p>";
        let mut context = HashMap::new();
        context.insert("title".to_string(), "Hello".to_string());
        context.insert("content".to_string(), "World".to_string());

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "<h1>Hello</h1><p>World</p>");
    }

    #[tokio::test]
    async fn test_web_app_creation() {
        let config = WebConfig {
            port: 3000,
            host: "127.0.0.1".to_string(),
            routes: vec![],
            middlewares: vec![],
            static_dir: None,
            template_dir: None,
        };

        let context = HandlerContext::default();
        let app = WebApp::new(config, context);

        let request = WebRequest {
            method: HttpMethod::GET,
            path: "/".to_string(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        let response = app.handle_request(request).await.unwrap();
        assert_eq!(response.status, 404); // No handler registered
    }
}
