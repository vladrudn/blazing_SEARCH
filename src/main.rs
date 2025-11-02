mod docx_parser;
mod document_record;
mod folder_processor;
mod search_engine;
mod web_server;
mod inverted_index;
mod auto_indexer;
mod atomic_index_manager;

use std::path::Path;
use std::env;
use search_engine::SearchEngine;
use inverted_index::InvertedIndex;
use document_record::DocumentIndex;
use atomic_index_manager::AtomicIndexManager;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    // Перевіряємо аргументи командного рядка
    if args.len() > 1 && args[1] == "web" {
        start_web_mode().await;
    } else {
        start_cli_mode().await;
    }
}

async fn start_web_mode() {
    println!("🔥 Blazing Search - Web Mode");
    println!("=============================");

    let index_path = "documents_index.json";
    println!("🔍 Перевірка індексу: {}", index_path);

    // Якщо індексів немає - створюємо їх автоматично
    if !Path::new(index_path).exists() {
        println!("⚠️  Файл індексу не знайдено: {}", index_path);
        println!("🔧 Створюємо початковий індекс...");
        println!("");

        // Викликаємо початкову індексацію
        perform_initial_indexing().await;

        println!("");
        println!("=============================");
    }

    // Завантажуємо пошуковий движок
    let mut search_engine = SearchEngine::new();

    if Path::new(index_path).exists() {
        if let Ok(metadata) = std::fs::metadata(index_path) {
            println!("📁 Розмір файлу індексу: {:.2} MB", metadata.len() as f64 / 1_048_576.0);
        }

        println!("⏳ Завантаження індексу...");
        match search_engine.load_from_file(index_path) {
            Ok(_) => {
                let (docs, words) = search_engine.get_stats();
                println!("✅ Завантажено {} документів з {} слів", docs, words);
            }
            Err(e) => {
                println!("❌ Помилка завантаження індексу: {}", e);
                println!("💡 Спробуйте видалити файли індексів та перезапустити");
                return;
            }
        }
    } else {
        println!("❌ Не вдалося створити індекс");
        println!("💡 Перевірте доступ до мережевої папки \\\\salem\\Documents\\Накази");
        return;
    }

    // Запуск веб-сервера
    if let Err(e) = web_server::start_web_server(search_engine).await {
        eprintln!("❌ Помилка запуску сервера: {}", e);
    }
}

async fn start_cli_mode() {
    println!("🔥 Blazing Search - Auto Indexer");
    println!("================================");

    // Автоматично запускаємо індексацію папки
    perform_initial_indexing().await;
}

async fn perform_initial_indexing() {
    let remote_folder = "\\\\salem\\Documents\\Накази";
    let local_cache = "./nakazi_cache";
    let documents_index_path = "documents_index.json";
    let inverted_index_path = "inverted_index.json";

    println!("🔍 Автоматична індексація папки: {}", remote_folder);
    println!("📥 Копіювання файлів до локального кешу: {}", local_cache);
    println!("📄 Результат буде збережено в: {} та {}", documents_index_path, inverted_index_path);

    // Копіюємо файли з сервера до локального кешу
    match sync_files_to_cache(remote_folder, local_cache) {
        Ok(count) => println!("✅ Скопійовано {} файлів до локального кешу", count),
        Err(e) => {
            println!("❌ Помилка копіювання файлів: {}", e);
            return;
        }
    }

    // Тепер індексуємо ЛОКАЛЬНИЙ кеш замість мережевої папки
    let folder_path = local_cache;

    // Створюємо атомарний менеджер індексів
    let index_manager = AtomicIndexManager::new(documents_index_path, inverted_index_path);

    // Очищуємо старі тимчасові файли на початку
    index_manager.cleanup_temp_files();

    // Виконуємо інкрементне оновлення з атомарним збереженням
    match index_manager.perform_incremental_update_atomically(folder_path) {
        Ok(stats) => {
            println!("\n✅ Інкрементне оновлення завершено!");
            println!("📊 Статистика: {}", stats);

            // Перевіряємо цілісність індексів та виправляємо при необхідності
            match index_manager.validate_indices() {
                Ok(_) => println!("✅ Перевірка цілісності пройшла успішно"),
                Err(e) => {
                    println!("⚠️ Попередження при перевірці цілісності: {}", e);
                    
                    // Спробуємо перебудувати інвертований індекс якщо потрібно
                    match index_manager.rebuild_inverted_index_if_needed() {
                        Ok(rebuilt) => {
                            if rebuilt {
                                println!("✅ Критичні проблеми виправлено шляхом перебудови індексу");
                            }
                        }
                        Err(rebuild_error) => {
                            println!("❌ Помилка при спробі перебудови індексу: {}", rebuild_error);
                        }
                    }
                }
            }

            // Показуємо розміри файлів
            let doc_path = Path::new(documents_index_path);
            if let Ok(metadata) = std::fs::metadata(doc_path) {
                println!("📦 Розмір індексу документів: {:.2} MB", metadata.len() as f64 / 1_048_576.0);
            }

            let inv_path = Path::new(inverted_index_path);
            if let Ok(metadata) = std::fs::metadata(inv_path) {
                println!("📦 Розмір інвертованого індексу: {:.2} MB", metadata.len() as f64 / 1_048_576.0);
            }

            // Показуємо загальну статистику
            if let Ok(doc_index) = DocumentIndex::load_from_file(documents_index_path) {
                println!("📊 Загальна статистика:");
                println!("   - Загальна кількість документів: {}", doc_index.total_documents);
                println!("   - Загальна кількість слів: {}", doc_index.total_words);

                if let Ok(inv_index) = InvertedIndex::load_from_file(inverted_index_path) {
                    let (docs, words) = inv_index.get_stats();
                    println!("   - Документів в інвертованому індексі: {}", docs);
                    println!("   - Унікальних слів в індексі: {}", words);
                }
            }
        }
        Err(error) => {
            println!("❌ Помилка інкрементного оновлення: {}", error);
            println!("🔧 Спробуємо очистити тимчасові файли...");
            index_manager.cleanup_temp_files();
        }
    }
}

/// Синхронізує файли з мережевої папки до локального кешу
fn sync_files_to_cache(remote_path: &str, local_cache_path: &str) -> Result<usize, String> {
    use std::fs;
    use std::collections::HashSet;
    use walkdir::WalkDir;

    // Створюємо локальну папку якщо не існує
    fs::create_dir_all(local_cache_path)
        .map_err(|e| format!("Помилка створення кешу: {}", e))?;

    let mut file_count = 0;
    let mut remote_files = HashSet::new();

    // Копіюємо файли з сервера
    for entry in WalkDir::new(remote_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let remote_file = entry.path();
            let relative_path = remote_file.strip_prefix(remote_path)
                .map_err(|e| format!("Помилка шляху: {}", e))?;

            remote_files.insert(relative_path.to_path_buf());

            let local_file = Path::new(local_cache_path).join(relative_path);

            // Перевіряємо, чи потрібно копіювати файл
            let should_copy = if local_file.exists() {
                if let (Ok(remote_meta), Ok(local_meta)) = (remote_file.metadata(), local_file.metadata()) {
                    if let (Ok(remote_modified), Ok(local_modified)) = (remote_meta.modified(), local_meta.modified()) {
                        remote_modified > local_modified || remote_meta.len() != local_meta.len()
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };

            if should_copy {
                // Створюємо підпапки якщо потрібно
                if let Some(parent) = local_file.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Помилка створення папки: {}", e))?;
                }

                // Копіюємо файл
                fs::copy(remote_file, &local_file)
                    .map_err(|e| format!("Помилка копіювання {}: {}", remote_file.display(), e))?;

                file_count += 1;
            }
        }
    }

    // Видаляємо файли, яких немає на сервері
    for entry in WalkDir::new(local_cache_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let local_file = entry.path();
            let relative_path = local_file.strip_prefix(local_cache_path)
                .map_err(|e| format!("Помилка шляху: {}", e))?;

            if !remote_files.contains(relative_path) {
                fs::remove_file(local_file)
                    .map_err(|e| format!("Помилка видалення {}: {}", local_file.display(), e))?;
            }
        }
    }

    Ok(file_count)
}