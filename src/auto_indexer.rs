use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use chrono::{DateTime, Local};
use crate::search_engine::SearchEngine;
use crate::atomic_index_manager::{AtomicIndexManager, UpdateStats};

pub struct AutoIndexer {
    folder_path: String,
    index_file_path: String,
    inverted_index_path: String,
    search_engine: Arc<SearchEngine>,
}

impl AutoIndexer {
    pub fn new(search_engine: Arc<SearchEngine>) -> Self {
        Self {
            folder_path: "\\\\salem\\Documents\\Накази".to_string(),
            index_file_path: "documents_index.json".to_string(),
            inverted_index_path: "inverted_index.json".to_string(),
            search_engine,
        }
    }

    pub async fn start_background_indexing(&self) {
        let folder_path = self.folder_path.clone();
        let index_file_path = self.index_file_path.clone();
        let inverted_index_path = self.inverted_index_path.clone();
        let search_engine = Arc::clone(&self.search_engine);

        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_secs(300)); //оновлення наказів
            let mut first_run = true;

            loop {
                interval_timer.tick().await;

                let now: DateTime<Local> = Local::now();
                let time_str = now.format("%H:%M:%S").to_string();

                if first_run {
                    println!("");
                    println!("🚀 [{time_str}] Запуск автоматичної перевірки файлів кожні 300 секунд...");
                    first_run = false;
                } else {
                    println!("");
                    println!("🔄 [{time_str}] Автоматична перевірка файлів...");
                }

                match Self::perform_incremental_update(
                    &folder_path,
                    &index_file_path,
                    &inverted_index_path,
                    &search_engine,
                ).await {
                    Ok(stats) => {
                        let end_time: DateTime<Local> = Local::now();
                        let end_time_str = end_time.format("%H:%M:%S").to_string();

                        if stats.has_changes() {
                            println!("✅ [{end_time_str}] Автоматичне оновлення завершено: {stats}");
                        } else {
                            println!("ℹ️ [{end_time_str}] Змін не виявлено");
                        }
                    }
                    Err(e) => {
                        let end_time: DateTime<Local> = Local::now();
                        let end_time_str = end_time.format("%H:%M:%S").to_string();
                        println!("❌ [{end_time_str}] Помилка автоматичного оновлення: {e}");
                    }
                }
            }
        });
    }

    async fn perform_incremental_update(
        folder_path: &str,
        index_file_path: &str,
        inverted_index_path: &str,
        search_engine: &Arc<SearchEngine>,
    ) -> Result<UpdateStats, String> {
        // Створюємо атомарний менеджер індексів
        let index_manager = AtomicIndexManager::new(index_file_path, inverted_index_path);

        // Очищуємо старі тимчасові файли
        index_manager.cleanup_temp_files();

        // Виконуємо атомарне інкрементне оновлення
        match index_manager.perform_incremental_update_atomically(folder_path) {
            Ok(stats) => {
                // Якщо є зміни, оновлюємо SearchEngine
                if stats.has_changes() {
                    // Перевіряємо цілісність індексів перед оновленням пошукового движка
                    if let Err(e) = index_manager.validate_indices() {
                        println!("⚠️ Попередження при перевірці цілісності індексів: {}", e);
                    }

                    // Оновлюємо SearchEngine
                    if let Err(e) = Self::reload_search_engine(search_engine, index_file_path).await {
                        println!("⚠️  Помилка оновлення пошукового движка: {}", e);
                    }
                }

                Ok(stats)
            }
            Err(e) => {
                println!("❌ Помилка атомарного оновлення: {}", e);
                // Очищуємо тимчасові файли при помилці
                index_manager.cleanup_temp_files();
                Err(e)
            }
        }
    }

    async fn reload_search_engine(search_engine: &Arc<SearchEngine>, index_file_path: &str) -> Result<(), String> {
        // Використовуємо новий метод reload для оновлення існуючого SearchEngine
        search_engine.reload(index_file_path)?;
        println!("✅ Пошуковий індекс успішно оновлено в пам'яті");

        Ok(())
    }
}

