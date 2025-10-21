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
        start_cli_mode();
    }
}

async fn start_web_mode() {
    println!("🔥 Blazing Search - Web Mode");
    println!("=============================");

    let mut search_engine = SearchEngine::new();

    // Завантаження індексу
    let index_path = "documents_index.json";
    println!("🔍 Перевірка індексу: {}", index_path);

    if Path::new(index_path).exists() {
        let metadata = std::fs::metadata(index_path).unwrap();
        println!("📁 Розмір файлу індексу: {:.2} MB", metadata.len() as f64 / 1_048_576.0);

        println!("⏳ Завантаження індексу...");
        match search_engine.load_from_file(index_path) {
            Ok(_) => {
                let (docs, words) = search_engine.get_stats();
                println!("✅ Завантажено {} документів з {} слів", docs, words);
            }
            Err(e) => {
                println!("❌ Помилка завантаження індексу: {}", e);
                println!("💡 Спочатку проіндексуйте документи за допомогою CLI режиму");
                return;
            }
        }
    } else {
        println!("⚠️  Файл індексу не знайдено: {}", index_path);
        println!("💡 Спочатку проіндексуйте документи за допомогою команди:");
        println!("   cargo run");
        return;
    }

    // Запуск веб-сервера
    if let Err(e) = web_server::start_web_server(search_engine).await {
        eprintln!("❌ Помилка запуску сервера: {}", e);
    }
}

fn start_cli_mode() {
    println!("🔥 Blazing Search - Auto Indexer");
    println!("================================");

    // Автоматично запускаємо індексацію папки
    process_folder_auto();
}


fn process_folder_auto() {
    let folder_path = "\\\\salem\\Documents\\Накази";
    let documents_index_path = "documents_index.json";
    let inverted_index_path = "inverted_index.json";

    println!("🔍 Автоматична індексація папки: {}", folder_path);
    println!("📄 Результат буде збережено в: {} та {}", documents_index_path, inverted_index_path);

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