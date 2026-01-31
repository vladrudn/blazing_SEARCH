use actix_web::{web, App, HttpServer, Result, HttpResponse, middleware::Logger};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::process::Command;
use crate::search_engine::{SearchEngine, SearchMode};
use crate::auto_indexer::AutoIndexer;
use std::net::UdpSocket;
use walkdir::WalkDir;
use rayon::prelude::*;

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

#[derive(Deserialize)]
pub struct SearchFilesRequest {
    pub query: String,
    pub folder_path: String,
}

#[derive(Serialize, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct SearchFilesResponse {
    pub files: Vec<FileInfo>,
    pub count: usize,
    pub processing_time_ms: u128,
}

#[derive(Serialize)]
pub struct FileIndexResponse {
    pub files: Vec<FileInfo>,
    pub total_count: usize,
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
    pub file_index_cache: Arc<Mutex<Vec<FileInfo>>>,
}

// Функція для отримання локальної IP-адреси
fn get_local_ip() -> Option<String> {
    // Створюємо UDP-сокет для з'єднання (без реальної відправки даних)
    // Це дозволяє ОС визначити правильний мережевий інтерфейс
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

// Функція для побудови індексу файлів у папці
fn build_file_index(folder_path: &str) -> Vec<FileInfo> {
    const MAX_DEPTH: usize = 10;

    let path = std::path::Path::new(folder_path);
    if !path.exists() || !path.is_dir() {
        println!("⚠️  Папка не знайдена: {}", folder_path);
        return Vec::new();
    }

    println!("🔍 Побудова індексу файлів у: {}", folder_path);

    // Паралельно збираємо всі файли
    let files: Vec<FileInfo> = WalkDir::new(path)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .par_bridge()
        .filter_map(|entry| {
            entry.file_name().to_str().map(|file_name| FileInfo {
                name: file_name.to_string(),
                path: entry.path().to_string_lossy().to_string(),
            })
        })
        .collect();

    println!("✅ Індекс побудовано: {} файлів", files.len());
    files
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

// Новий handler для отримання кешованого індексу файлів
pub async fn get_file_index_handler(
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let cached_files = data.file_index_cache.lock().unwrap();
    let response = FileIndexResponse {
        total_count: cached_files.len(),
        files: cached_files.clone(),
    };
    Ok(HttpResponse::Ok().json(response))
}

// Handler для отримання вмісту файлу для превью
pub async fn get_file_preview_handler(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let file_path = path.into_inner();

    // Декодуємо URL-кодовану шлях
    let decoded_path = urlencoding::decode(&file_path)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| file_path);

    // Перевіряємо чи файл існує
    let path = std::path::Path::new(&decoded_path);
    if !path.exists() || !path.is_file() {
        return Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "Файл не знайдено".to_string(),
        }));
    }

    // Визначаємо тип контенту за розширенням
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Обробка документів (конвертація в PDF)
    if ext == "doc" || ext == "docx" {
        return convert_doc_to_pdf(&decoded_path).await;
    }

    // Читаємо вміст файлу
    match std::fs::read(&decoded_path) {
        Ok(content) => {
            let content_type = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "bmp" => "image/bmp",
                "pdf" => "application/pdf",
                _ => "application/octet-stream",
            };

            Ok(HttpResponse::Ok()
                .content_type(content_type)
                .body(content))
        }
        Err(_) => {
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Помилка читання файлу".to_string(),
            }))
        }
    }
}

// Функція для конвертації .doc/.docx у PDF
async fn convert_doc_to_pdf(file_path: &str) -> Result<HttpResponse> {
    use std::process::Command;
    use std::path::PathBuf;

    let input_path = PathBuf::from(file_path);
    let temp_dir = std::env::temp_dir();
    let file_name = input_path.file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    // Список можливих шляхів до LibreOffice на Windows
    let possible_paths = vec![
        "soffice",
        "soffice.exe",
        "C:\\Program Files\\LibreOffice\\program\\soffice.exe",
        "C:\\Program Files (x86)\\LibreOffice\\program\\soffice.exe",
    ];

    // Спробуємо кожен можливий шлях
    for libreoffice_path in possible_paths {
        let cmd_result = if cfg!(target_os = "windows") {
            Command::new(libreoffice_path)
                .args(&[
                    "--headless",
                    "--convert-to", "pdf",
                    "--outdir", temp_dir.to_str().unwrap_or("."),
                    file_path
                ])
                .output()
        } else {
            Command::new(libreoffice_path)
                .args(&[
                    "--headless",
                    "--convert-to", "pdf",
                    "--outdir", temp_dir.to_str().unwrap_or("."),
                    file_path
                ])
                .output()
        };

        if let Ok(output) = cmd_result {
            if output.status.success() {
                // Шукаємо згенерований PDF файл
                let expected_pdf = temp_dir.join(format!("{}.pdf", file_name));
                if expected_pdf.exists() {
                    match std::fs::read(&expected_pdf) {
                        Ok(content) => {
                            // Видаляємо тимчасовий файл після читання
                            let _ = std::fs::remove_file(&expected_pdf);
                            println!("✅ Документ успішно конвертовано: {}", file_path);
                            return Ok(HttpResponse::Ok()
                                .content_type("application/pdf")
                                .body(content));
                        }
                        Err(_) => {
                            println!("⚠️  Помилка читання конвертованого PDF");
                        }
                    }
                }
            } else {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                println!("⚠️  Помилка конвертації: {}", error_msg);
            }
        }
    }

    println!("⚠️  LibreOffice не знайдено у жодному зі стандартних місць");

    // Якщо конвертація не вдалася, повертаємо помилку
    Ok(HttpResponse::InternalServerError().json(ErrorResponse {
        error: "Не вдалося конвертувати документ у PDF. Переконайтеся, що LibreOffice встановлено.".to_string(),
    }))
}

pub async fn search_files_handler(
    data: web::Data<AppState>,
    request: web::Json<SearchFilesRequest>,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();

    if request.query.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Порожній запит пошуку".to_string(),
        }));
    }

    // Використовуємо кешований індекс замість проходження по папці
    let cached_files = data.file_index_cache.lock().unwrap();
    let query_lower = request.query.to_lowercase();
    const MAX_RESULTS: usize = 200; // Обмежуємо кількість результатів

    // Шукаємо у кешованому індексі (дуже швидко)
    let mut found_files: Vec<FileInfo> = cached_files
        .par_iter()
        .filter(|file| file.name.to_lowercase().contains(&query_lower))
        .cloned()
        .collect();

    // Обмежуємо кількість результатів
    found_files.truncate(MAX_RESULTS);

    let processing_time = start_time.elapsed().as_millis();

    let response = SearchFilesResponse {
        count: found_files.len(),
        files: found_files,
        processing_time_ms: processing_time,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn start_web_server(search_engine: SearchEngine) -> std::io::Result<()> {
    let search_engine_arc = Arc::new(search_engine);

    // Побудова індексу файлів при старті
    const DEFAULT_FOLDER_PATH: &str = "/mnt/salem-documents/ФОТО ВК";
    let file_index = build_file_index(DEFAULT_FOLDER_PATH);
    let file_index_cache = Arc::new(Mutex::new(file_index));

    let app_state = web::Data::new(AppState {
        search_engine: search_engine_arc.clone(),
        file_index_cache: file_index_cache.clone(),
    });

    // Запускаємо автоматичний індексер
    println!("🚀 Запуск автоматичного індексера (перевірка кожні 3 хвилини)...");
    let auto_indexer = AutoIndexer::new(search_engine_arc);
    auto_indexer.start_background_indexing().await;

    // Запускаємо автоматичне оновлення індексу файлів кожні 3 хвилини
    println!("🚀 Запуск оновлення індексу файлів (кожні 3 хвилини)...");
    let file_index_cache_clone = file_index_cache.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(180)).await; // 3 хвилини

            println!("🔄 Оновлення індексу файлів...");
            let updated_index = build_file_index(DEFAULT_FOLDER_PATH);

            // Оновлюємо кеш
            if let Ok(mut cache) = file_index_cache_clone.lock() {
                *cache = updated_index;
                println!("✅ Індекс файлів оновлено");
            }
        }
    });

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
            .route("/api/file-index", web::get().to(get_file_index_handler))
            .route("/api/file-preview/{path:.*}", web::get().to(get_file_preview_handler))
            .route("/api/search-files", web::post().to(search_files_handler))
            .route("/api/open-file", web::post().to(open_file_handler))
            .route("/static/{filename:.*}", web::get().to(static_handler))
            .route("/static/{filename:.*}", web::head().to(static_handler))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
