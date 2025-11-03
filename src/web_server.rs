use actix_web::{web, App, HttpServer, Result, HttpResponse, middleware::Logger};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::process::Command;
use crate::search_engine::{SearchEngine, SearchMode};
use crate::auto_indexer::AutoIndexer;
use std::net::UdpSocket;

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub full_search: Option<bool>,
    pub view_mode: Option<String>, // "fragments" або "full-document"
}

#[derive(Deserialize)]
pub struct OpenFileRequest {
    pub file_path: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub count: usize,
    pub total_count: usize,
    pub query: String,
    pub processing_time_ms: u128,
}

#[derive(Serialize, Clone)]
pub struct ParagraphData {
    pub text: String,
    #[serde(default)]
    pub line_breaks_after: usize,
}

#[derive(Serialize, Clone)]
pub struct SearchResult {
    pub file_name: String,
    pub file_path: String,
    pub full_path: String,
    pub matches: Vec<MatchInfo>,
    pub all_paragraphs: Vec<ParagraphData>,
    pub file_size: u64,
    pub last_modified: u64,
}

#[derive(Serialize, Clone)]
pub struct MatchInfo {
    pub context: String,
    pub position: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct AppState {
    pub search_engine: Arc<SearchEngine>,
}

// Функція для отримання локальної IP-адреси
fn get_local_ip() -> Option<String> {
    // Створюємо UDP-сокет для з'єднання (без реальної відправки даних)
    // Це дозволяє ОС визначити правильний мережевий інтерфейс
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

pub async fn search_handler(
    data: web::Data<AppState>,
    query: web::Json<SearchRequest>,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();


    if query.query.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Порожній запит пошуку".to_string(),
        }));
    }

    let search_mode = if query.full_search.unwrap_or(false) {
        SearchMode::Remaining
    } else {
        SearchMode::Quick
    };

    let results = match data.search_engine.search(&query.query, search_mode, query.view_mode.as_deref()).await {
        Ok(all_results) => all_results,
        Err(err) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Помилка пошуку: {}", err),
            }));
        }
    };

    let total_doc_count = data.search_engine.get_stats().0;
    let processing_time = start_time.elapsed().as_millis();

    let search_results: Vec<SearchResult> = results.into_iter().map(|r| {
        SearchResult {
            file_name: r.file_name,
            file_path: r.file_path.clone(),
            full_path: r.file_path,
            matches: r.matches.into_iter().map(|m| MatchInfo {
                context: m.context,
                position: m.position,
            }).collect(),
            all_paragraphs: r.all_paragraphs.into_iter().map(|p| ParagraphData {
                text: p.text,
                line_breaks_after: p.line_breaks_after,
            }).collect(),
            file_size: r.file_size,
            last_modified: r.last_modified,
        }
    }).collect();

    let response = SearchResponse {
        count: search_results.len(),
        total_count: total_doc_count,
        results: search_results,
        query: query.query.clone(),
        processing_time_ms: processing_time,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn index_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
        .insert_header(("Pragma", "no-cache"))
        .insert_header(("Expires", "0"))
        .body(include_str!("../web/nakaz.html")))
}

pub async fn static_handler(req: actix_web::HttpRequest) -> Result<HttpResponse> {
    let path: std::path::PathBuf = req.match_info()
        .query("filename")
        .parse()
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid file path"))?;
    let file_path = std::path::Path::new("./web").join(path);

    match std::fs::read(&file_path) {
        Ok(content) => {
            let content_type = mime_guess::from_path(&file_path).first_or_octet_stream().to_string();
            Ok(HttpResponse::Ok()
                .content_type(content_type)
                .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                .insert_header(("Pragma", "no-cache"))
                .insert_header(("Expires", "0"))
                .body(content))
        },
        Err(_) => Ok(HttpResponse::NotFound().body("File not found"))
    }
}

pub async fn open_file_handler(
    request: web::Json<OpenFileRequest>,
) -> Result<HttpResponse> {
    // Перевіряємо пароль
    const CORRECT_PASSWORD: &str = "4053@115";
    if request.password != CORRECT_PASSWORD {
        return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Неправильний пароль".to_string(),
        }));
    }

    // Перевіряємо чи файл існує
    if !std::path::Path::new(&request.file_path).exists() {
        return Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "Файл не знайдено".to_string(),
        }));
    }

    // Спробуємо відкрити файл через системний виклик
    let result = if cfg!(target_os = "windows") {
        // Для Windows використовуємо cmd /c start
        Command::new("cmd")
            .args(&["/c", "start", "", &request.file_path])
            .spawn()
    } else if cfg!(target_os = "macos") {
        // Для macOS використовуємо open
        Command::new("open")
            .arg(&request.file_path)
            .spawn()
    } else {
        // Для Linux використовуємо xdg-open
        Command::new("xdg-open")
            .arg(&request.file_path)
            .spawn()
    };

    match result {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Файл відкрито"
            })))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Помилка відкриття файлу: {}", e),
            }))
        }
    }
}


pub async fn start_web_server(search_engine: SearchEngine) -> std::io::Result<()> {
    let search_engine_arc = Arc::new(search_engine);

    let app_state = web::Data::new(AppState {
        search_engine: search_engine_arc.clone(),
    });

    // Запускаємо автоматичний індексер
    println!("🚀 Запуск автоматичного індексера (перевірка кожні 120 секунд)...");
    let auto_indexer = AutoIndexer::new(search_engine_arc);
    auto_indexer.start_background_indexing().await;

    println!("Запуск веб-сервера на http://0.0.0.0:8080");

    // Виводимо актуальну локальну IP-адресу
    if let Some(local_ip) = get_local_ip() {
        println!("Доступ з локальної мережі: http://{}:8080", local_ip);
    } else {
        println!("⚠️  Не вдалося визначити локальну IP-адресу");
        println!("💡 Використовуйте localhost або перевірте ipconfig");
    }

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .route("/", web::get().to(index_handler))
            .route("/api/search", web::post().to(search_handler))
            .route("/api/open-file", web::post().to(open_file_handler))
            .route("/static/{filename:.*}", web::get().to(static_handler))
            .route("/static/{filename:.*}", web::head().to(static_handler))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
