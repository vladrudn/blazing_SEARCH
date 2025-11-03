use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use chrono::{DateTime, Local};
use crate::search_engine::SearchEngine;
use crate::atomic_index_manager::{AtomicIndexManager, UpdateStats};

pub struct AutoIndexer {
    folder_path: String,           // Мережева папка \\salem\Documents\Наказі
    local_cache_path: String,      // Локальна копія файлів
    index_file_path: String,
    inverted_index_path: String,
    search_engine: Arc<SearchEngine>,
}

impl AutoIndexer {
    pub fn new(search_engine: Arc<SearchEngine>) -> Self {
        Self {
            folder_path: "\\\\salem\\Documents\\Накази".to_string(),
            local_cache_path: "./nakazi_cache".to_string(),
            index_file_path: "documents_index.json".to_string(),
            inverted_index_path: "inverted_index.json".to_string(),
            search_engine,
        }
    }

    pub async fn start_background_indexing(&self) {
        let folder_path = self.folder_path.clone();
        let local_cache_path = self.local_cache_path.clone();
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

                // КРОК 1: Швидка перевірка - чи є зміни?
                match Self::check_for_changes(&folder_path, &local_cache_path).await {
                    Ok(has_changes) => {
                        if !has_changes {
                            let end_time_str = Local::now().format("%H:%M:%S").to_string();
                            println!("ℹ️ [{end_time_str}] Змін не виявлено - пропускаємо копіювання");
                            continue; // ❌ НЕ КОПІЮЄМО, НЕ ІНДЕКСУЄМО
                        }

                        println!("📥 [{time_str}] Виявлено зміни - копіюємо файли...");

                        // КРОК 2: Копіюємо ТІЛЬКИ якщо є зміни
                        if let Err(e) = Self::sync_to_local_cache(&folder_path, &local_cache_path).await {
                            let end_time_str = Local::now().format("%H:%M:%S").to_string();
                            println!("❌ [{end_time_str}] Помилка копіювання: {e}");
                            continue;
                        }

                        // КРОК 3: Індексуємо ЛОКАЛЬНУ копію
                        match Self::perform_incremental_update(
                            &local_cache_path,  // 👈 Індексуємо локальні файли
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
                                    println!("ℹ️ [{end_time_str}] Індексація завершена без змін");
                                }
                            }
                            Err(e) => {
                                let end_time_str = Local::now().format("%H:%M:%S").to_string();
                                println!("❌ [{end_time_str}] Помилка індексації: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        // 🔒 ОФЛАЙН-РЕЖИМ: Мережа недоступна - НЕ ВИДАЛЯЄМО БАЗУ!
                        let end_time_str = Local::now().format("%H:%M:%S").to_string();
                        println!("⚠️ [{end_time_str}] {}", e);
                        println!("💡 [{end_time_str}] Застосунок продовжує працювати з існуючою базою даних");
                        // ⚠️ ВАЖЛИВО: НЕ виконуємо жодних операцій з базою даних!
                        // continue - просто чекаємо до наступної перевірки
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

    /// Перевіряє доступність мережевої папки
    fn is_network_path_accessible(path: &str) -> bool {
        use std::path::Path;

        let network_path = Path::new(path);

        // Перевіряємо чи це мережевий шлях
        if !path.starts_with("\\\\") {
            return network_path.exists();
        }

        // Для мережевих шляхів робимо більш ретельну перевірку
        if !network_path.exists() {
            return false;
        }

        // Пробуємо прочитати вміст папки для перевірки доступу
        match std::fs::read_dir(path) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Збирає метадані файлів (шлях, розмір, дата модифікації) БЕЗ читання вмісту
    /// ВАЖЛИВО: Зберігає ВІДНОСНІ шляхи для коректного порівняння
    /// Фільтрує тільки файли з папок-років
    fn collect_metadata(path: &str) -> Result<Vec<(String, u64, std::time::SystemTime)>, String> {
        use walkdir::WalkDir;
        use std::path::Path;

        let mut metadata = Vec::new();
        let base_path = Path::new(path);

        // Перевіряємо, чи існує шлях
        if !base_path.exists() {
            return Err(format!("Шлях не існує або недоступний: {}", path));
        }

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        // Отримуємо ВІДНОСНИЙ шлях від базової папки
                        let relative_path_buf = entry.path()
                            .strip_prefix(base_path)
                            .unwrap_or(entry.path());

                        // Фільтруємо тільки файли з папок-років
                        if !Self::should_sync_file(relative_path_buf) {
                            continue;
                        }

                        let relative_path = relative_path_buf.to_string_lossy().to_string();

                        metadata.push((relative_path, meta.len(), modified));
                    }
                }
            }
        }

        metadata.sort();
        Ok(metadata)
    }

    /// Швидка перевірка - порівнює метадані без копіювання файлів
    /// Повертає: Ok(true) - є зміни, Ok(false) - немає змін, Err - мережа недоступна
    async fn check_for_changes(
        remote_path: &str,
        local_cache_path: &str,
    ) -> Result<bool, String> {
        use std::path::Path;

        // 🔒 КРИТИЧНА ПЕРЕВІРКА: Чи доступна мережева папка?
        if !Self::is_network_path_accessible(remote_path) {
            return Err(format!(
                "🌐 ОФЛАЙН-РЕЖИМ: Мережева папка недоступна: {}\n\
                 💾 Працюємо з існуючим локальним кешем без оновлень",
                remote_path
            ));
        }

        // Якщо локального кешу немає - потрібно копіювати
        if !Path::new(local_cache_path).exists() {
            return Ok(true);
        }

        // Читаємо метадані з мережевої папки (ШВИДКО - без копіювання)
        let remote_metadata = Self::collect_metadata(remote_path)?;
        let local_metadata = match Self::collect_metadata(local_cache_path) {
            Ok(metadata) => metadata,
            Err(_) => {
                // Якщо локальний кеш не читається - потрібно синхронізувати
                return Ok(true);
            }
        };

        // Порівнюємо: кількість файлів, розміри, дати модифікації
        Ok(remote_metadata != local_metadata)
    }

    /// Перевіряє, чи файл належить до папки з роком (2022, 2023, 2024, 2025 тощо)
    /// Виключає: ZIP-архіви, Excel-файли, папку "ЕРДР", .git репозиторій
    fn should_sync_file(relative_path: &std::path::Path) -> bool {
        let path_str = relative_path.to_string_lossy();

        // Виключаємо файли в кореневій папці (не в підпапках)
        if relative_path.components().count() == 1 {
            return false;
        }

        // Отримуємо першу частину шляху (папку верхнього рівня)
        let first_component = relative_path.components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("");

        // Перевіряємо, чи це папка з роком (починається з 4 цифр)
        let is_year_folder = first_component.len() >= 4
            && first_component.chars().take(4).all(|c| c.is_ascii_digit());

        // Виключаємо небажані файли та папки
        let is_excluded = path_str.ends_with(".zip")
            || path_str.ends_with(".xlsx")
            || path_str.ends_with(".xls")
            || path_str.contains("ЕРДР")
            || path_str.contains(".git");

        is_year_folder && !is_excluded
    }

    /// Синхронізує файли з сервера на локальний диск (копіює нові/оновлені, видаляє застарілі)
    async fn sync_to_local_cache(
        remote_path: &str,
        local_cache_path: &str,
    ) -> Result<(), String> {
        use std::fs;
        use std::path::Path;
        use std::collections::HashSet;
        use walkdir::WalkDir;

        // Створюємо локальну папку якщо не існує
        fs::create_dir_all(local_cache_path)
            .map_err(|e| format!("Помилка створення кешу: {}", e))?;

        // Збираємо список всіх файлів на сервері
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

                // Фільтруємо файли - тільки папки з роками
                if !Self::should_sync_file(relative_path) {
                    continue;
                }

                // Додаємо до списку файлів на сервері
                remote_files.insert(relative_path.to_path_buf());

                let local_file = Path::new(local_cache_path).join(relative_path);

                // Перевіряємо, чи потрібно копіювати файл
                let should_copy = if local_file.exists() {
                    // Порівнюємо дати модифікації та розміри
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

                // Якщо файлу немає на сервері - видаляємо
                if !remote_files.contains(relative_path) {
                    fs::remove_file(local_file)
                        .map_err(|e| format!("Помилка видалення {}: {}", local_file.display(), e))?;
                }
            }
        }

        Ok(())
    }
}

