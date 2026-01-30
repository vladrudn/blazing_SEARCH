use crate::atomic_index_manager::{AtomicIndexManager, UpdateStats};
use crate::search_engine::SearchEngine;
use chrono::{DateTime, Local};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

pub struct AutoIndexer {
    folder_path: String,      // Мережева папка \\salem\Documents\Наказі
    local_cache_path: String, // Локальна копія файлів
    index_file_path: String,
    inverted_index_path: String,
    search_engine: Arc<SearchEngine>,
}

impl AutoIndexer {
    pub fn new(search_engine: Arc<SearchEngine>) -> Self {
        Self {
            folder_path: "/mnt/salem-documents/Накази".to_string(),
            // folder_path: "C:\\Users\\vladr\\Desktop\\НАКАЗИ\\".to_string(),
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
            let mut interval_timer = interval(Duration::from_secs(180)); //оновлення наказів
            let mut first_run = true;

            loop {
                interval_timer.tick().await;

                let now: DateTime<Local> = Local::now();
                let time_str = now.format("%H:%M:%S").to_string();

                if first_run {
                    println!("");
                    println!(
                        "🚀 [{time_str}] Запуск автоматичної перевірки файлів кожні 180 секунд..."
                    );
                    first_run = false;
                } else {
                    println!("");
                    println!("🔄 [{time_str}] Автоматична перевірка файлів...");
                }

                // КРОК 1: Перевіряємо чи є зміни на сервері (для синхронізації)
                let should_sync = match Self::check_for_changes(&folder_path, &local_cache_path)
                    .await
                {
                    Ok(has_changes) => {
                        if has_changes {
                            println!(
                                "📥 [{time_str}] Виявлено зміни на сервері - копіюємо файли..."
                            );
                        } else {
                            let end_time_str = Local::now().format("%H:%M:%S").to_string();
                            println!(
                                "ℹ️ [{end_time_str}] Змін на сервері не виявлено - пропускаємо копіювання"
                            );
                        }
                        has_changes
                    }
                    Err(e) => {
                        // 🔒 ОФЛАЙН-РЕЖИМ: Мережа недоступна
                        let end_time_str = Local::now().format("%H:%M:%S").to_string();
                        println!("⚠️ [{end_time_str}] {}", e);
                        println!("💡 [{end_time_str}] Працюємо в офлайн-режимі з локальним кешем");
                        false // Не синхронізуємо, але продовжуємо перевіряти індекс
                    }
                };

                // КРОК 2: Копіюємо файли з сервера ТІЛЬКИ якщо є зміни
                if should_sync {
                    if let Err(e) = Self::sync_to_local_cache(&folder_path, &local_cache_path).await
                    {
                        let end_time_str = Local::now().format("%H:%M:%S").to_string();
                        println!("❌ [{end_time_str}] Помилка копіювання: {e}");
                        // Не продовжуємо цикл - перевіримо індекс нижче
                    }
                }

                // КРОК 3: ЗАВЖДИ перевіряємо чи кеш синхронізований з індексом
                // Це захищає від ситуації коли копіювання відбулося, але індексування перервалося
                let cache_needs_indexing = match Self::check_cache_vs_index(
                    &local_cache_path,
                    &index_file_path,
                )
                .await
                {
                    Ok(needs_indexing) => {
                        if needs_indexing {
                            println!(
                                "🔍 [{time_str}] Виявлено неіндексовані файли в кеші - запускаємо індексацію..."
                            );
                        } else {
                            let end_time_str = Local::now().format("%H:%M:%S").to_string();
                            println!(
                                "✅ [{end_time_str}] Кеш синхронізований з індексом - індексування не потрібне"
                            );
                        }
                        needs_indexing
                    }
                    Err(e) => {
                        println!("⚠️ Помилка перевірки кешу vs індекс: {}", e);
                        true // Перестраховуємось - індексуємо
                    }
                };

                // КРОК 4: Індексуємо ТІЛЬКИ якщо потрібно
                if cache_needs_indexing {
                    match Self::perform_incremental_update(
                        &local_cache_path, // 👈 Індексуємо локальні файли з кешу
                        &index_file_path,
                        &inverted_index_path,
                        &search_engine,
                    )
                    .await
                    {
                        Ok(stats) => {
                            let end_time: DateTime<Local> = Local::now();
                            let end_time_str = end_time.format("%H:%M:%S").to_string();

                            if stats.has_changes() {
                                println!(
                                    "✅ [{end_time_str}] Автоматичне оновлення завершено: {stats}"
                                );
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
                    if let Err(e) = Self::reload_search_engine(search_engine, index_file_path).await
                    {
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

    async fn reload_search_engine(
        search_engine: &Arc<SearchEngine>,
        index_file_path: &str,
    ) -> Result<(), String> {
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
        use std::path::Path;
        use walkdir::WalkDir;

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
                        let relative_path_buf =
                            entry.path().strip_prefix(base_path).unwrap_or(entry.path());

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

    /// Перевіряє чи є неіндексовані файли в локальному кеші
    /// Порівнює файли в nakazi_cache з тими що є в documents_index.json
    /// Повертає: Ok(true) - потрібно індексувати, Ok(false) - все синхронізовано
    async fn check_cache_vs_index(cache_path: &str, index_file_path: &str) -> Result<bool, String> {
        use crate::document_record::DocumentIndex;
        use std::path::Path;

        // Якщо кешу немає - нічого індексувати
        if !Path::new(cache_path).exists() {
            return Ok(false);
        }

        // Збираємо метадані з локального кешу
        let cache_metadata = match Self::collect_metadata(cache_path) {
            Ok(metadata) => metadata,
            Err(e) => {
                // Помилка читання кешу - краще перестрахуватися та запустити індексацію
                println!("⚠️  Помилка читання кешу: {}", e);
                return Ok(true);
            }
        };

        // Якщо кеш порожній - нічого індексувати
        if cache_metadata.is_empty() {
            return Ok(false);
        }

        // Завантажуємо існуючий індекс
        let existing_index = match DocumentIndex::load_from_file(index_file_path) {
            Ok(index) => index,
            Err(_) => {
                // Індексу немає - потрібно створити
                println!("ℹ️  Індекс не знайдено - потрібне повне індексування");
                return Ok(true);
            }
        };

        // Створюємо мапу індексованих файлів: шлях → (розмір, час модифікації)
        let mut indexed_files = std::collections::HashMap::new();
        for doc in &existing_index.documents {
            // Отримуємо відносний шлях (прибираємо префікс nakazi_cache/)
            let relative_path = if let Some(rel) = doc.file_path.strip_prefix(cache_path) {
                rel.trim_start_matches('\\')
                    .trim_start_matches('/')
                    .to_string()
            } else {
                doc.file_path.clone()
            };

            indexed_files.insert(relative_path, (doc.file_size, doc.last_modified));
        }

        // Перевіряємо чи всі файли з кешу є в індексі
        for (cache_file_path, cache_size, cache_modified) in &cache_metadata {
            let cache_modified_secs = cache_modified
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            match indexed_files.get(cache_file_path) {
                Some((indexed_size, indexed_modified)) => {
                    // Файл є в індексі - перевіряємо чи він не змінився
                    if cache_size != indexed_size || cache_modified_secs > *indexed_modified {
                        println!("🔄 Файл змінився: {}", cache_file_path);
                        return Ok(true); // Файл оновлено
                    }
                }
                None => {
                    // Файл є в кеші, але немає в індексі!
                    println!("➕ Новий файл в кеші: {}", cache_file_path);
                    return Ok(true);
                }
            }
        }

        // Перевіряємо чи не видалені файли з кешу (є в індексі, але немає в кеші)
        let cache_files_set: std::collections::HashSet<_> = cache_metadata
            .iter()
            .map(|(path, _, _)| path.clone())
            .collect();

        for indexed_file in indexed_files.keys() {
            if !cache_files_set.contains(indexed_file) {
                println!("➖ Файл видалено з кешу: {}", indexed_file);
                return Ok(true);
            }
        }

        // Все синхронізовано!
        Ok(false)
    }

    /// Швидка перевірка - порівнює метадані без копіювання файлів
    /// Повертає: Ok(true) - є зміни, Ok(false) - немає змін, Err - мережа недоступна
    async fn check_for_changes(remote_path: &str, local_cache_path: &str) -> Result<bool, String> {
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
        let first_component = relative_path
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("");

        // Перевіряємо, чи це папка з роком (починається з 4 цифр)
        let is_year_folder = first_component.len() >= 4
            && first_component.chars().take(4).all(|c| c.is_ascii_digit());

        // Отримуємо ім'я файлу та розширення
        let filename = relative_path.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");

        // Синхронізуємо ТІЛЬКИ .docx файли (крім тимчасових ~$)
        let is_docx = path_str.to_lowercase().ends_with(".docx");
        let is_temp_office = filename.starts_with("~$");

        is_year_folder && is_docx && !is_temp_office
    }

    /// Синхронізує файли з сервера на локальний диск (копіює нові/оновлені, видаляє застарілі)
    async fn sync_to_local_cache(remote_path: &str, local_cache_path: &str) -> Result<(), String> {
        use std::collections::HashSet;
        use std::fs;
        use std::path::Path;
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
                let relative_path = remote_file
                    .strip_prefix(remote_path)
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
                    if let (Ok(remote_meta), Ok(local_meta)) =
                        (remote_file.metadata(), local_file.metadata())
                    {
                        if let (Ok(remote_modified), Ok(local_modified)) =
                            (remote_meta.modified(), local_meta.modified())
                        {
                            remote_modified > local_modified
                                || remote_meta.len() != local_meta.len()
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
                    fs::copy(remote_file, &local_file).map_err(|e| {
                        format!("Помилка копіювання {}: {}", remote_file.display(), e)
                    })?;
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
                let relative_path = local_file
                    .strip_prefix(local_cache_path)
                    .map_err(|e| format!("Помилка шляху: {}", e))?;

                // Якщо файлу немає на сервері - видаляємо
                if !remote_files.contains(relative_path) {
                    fs::remove_file(local_file).map_err(|e| {
                        format!("Помилка видалення {}: {}", local_file.display(), e)
                    })?;
                }
            }
        }

        Ok(())
    }
}
