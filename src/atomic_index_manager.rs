use std::path::Path;
use std::fs::{self, OpenOptions};
use fs4::fs_std::FileExt;
use chrono::{DateTime, Local};
use crate::document_record::DocumentIndex;
use crate::inverted_index::InvertedIndex;
use crate::folder_processor::FolderProcessor;

/// Менеджер для атомарного оновлення індексів
/// Забезпечує, що обидва індекси (документний та інвертований) 
/// оновлюються разом або не оновлюються взагалі
pub struct AtomicIndexManager {
    pub documents_index_path: String,
    pub inverted_index_path: String,
}

impl AtomicIndexManager {
    pub fn new(documents_path: &str, inverted_path: &str) -> Self {
        Self {
            documents_index_path: documents_path.to_string(),
            inverted_index_path: inverted_path.to_string(),
        }
    }

    /// Атомарно зберігає обидва індекси
    /// Використовує систему тимчасових файлів та транзакційний підхід
    pub fn save_indices_atomically(
        &self,
        document_index: &DocumentIndex,
        inverted_index: &InvertedIndex,
    ) -> Result<(), String> {
        println!("🔄 Початок атомарного збереження індексів...");

        // Етап 1: Створюємо тимчасові файли для обох індексів
        let temp_doc_path = format!("{}.atomic_temp", self.documents_index_path);
        let temp_inv_path = format!("{}.atomic_temp", self.inverted_index_path);
        
        // Створюємо резервні копії існуючих файлів
        let backup_doc_path = format!("{}.atomic_backup", self.documents_index_path);
        let backup_inv_path = format!("{}.atomic_backup", self.inverted_index_path);

        // Очищуємо старі тимчасові файли якщо вони є
        let _ = fs::remove_file(&temp_doc_path);
        let _ = fs::remove_file(&temp_inv_path);

        println!("📝 Збереження в тимчасові файли...");
        
        // Етап 2: Зберігаємо обидва індекси в тимчасові файли
        if let Err(e) = self.save_document_index_to_temp(&temp_doc_path, document_index) {
            // Очищуємо тимчасові файли при помилці
            let _ = fs::remove_file(&temp_doc_path);
            let _ = fs::remove_file(&temp_inv_path);
            return Err(format!("Помилка збереження індексу документів в тимчасовий файл: {}", e));
        }

        if let Err(e) = self.save_inverted_index_to_temp(&temp_inv_path, inverted_index) {
            // Очищуємо тимчасові файли при помилці
            let _ = fs::remove_file(&temp_doc_path);
            let _ = fs::remove_file(&temp_inv_path);
            return Err(format!("Помилка збереження інвертованого індексу в тимчасовий файл: {}", e));
        }

        println!("💾 Створення резервних копій...");
        
        // Етап 3: Створюємо резервні копії існуючих файлів
        if Path::new(&self.documents_index_path).exists() {
            if let Err(e) = fs::copy(&self.documents_index_path, &backup_doc_path) {
                // Очищуємо тимчасові файли при помилці
                let _ = fs::remove_file(&temp_doc_path);
                let _ = fs::remove_file(&temp_inv_path);
                return Err(format!("Помилка створення резервної копії індексу документів: {}", e));
            }
        }

        if Path::new(&self.inverted_index_path).exists() {
            if let Err(e) = fs::copy(&self.inverted_index_path, &backup_inv_path) {
                // Очищуємо тимчасові файли при помилці
                let _ = fs::remove_file(&temp_doc_path);
                let _ = fs::remove_file(&temp_inv_path);
                let _ = fs::remove_file(&backup_doc_path);
                return Err(format!("Помилка створення резервної копії інвертованого індексу: {}", e));
            }
        }

        println!("🔄 Атомарне переміщення файлів...");

        // Етап 4: Атомарно переміщуємо тимчасові файли на місце основних
        // На Windows потрібно спочатку видалити існуючий файл перед rename

        // Спочатку переміщуємо індекс документів
        if Path::new(&self.documents_index_path).exists() {
            // Пробуємо видалити з кількома спробами (файл може бути тимчасово зайнятий)
            let mut attempts = 0;
            loop {
                match fs::remove_file(&self.documents_index_path) {
                    Ok(_) => break,
                    Err(_e) if attempts < 3 => {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        self.restore_from_backups(&backup_doc_path, &backup_inv_path);
                        let _ = fs::remove_file(&temp_doc_path);
                        let _ = fs::remove_file(&temp_inv_path);
                        return Err(format!("Помилка видалення старого індексу документів після {} спроб: {}", attempts + 1, e));
                    }
                }
            }
        }

        if let Err(e) = fs::rename(&temp_doc_path, &self.documents_index_path) {
            // При помилці відновлюємо з резервних копій
            self.restore_from_backups(&backup_doc_path, &backup_inv_path);
            let _ = fs::remove_file(&temp_inv_path);
            return Err(format!("Помилка переміщення індексу документів: {}", e));
        }

        // Потім переміщуємо інвертований індекс
        if Path::new(&self.inverted_index_path).exists() {
            // Пробуємо видалити з кількома спробами (файл може бути тимчасово зайнятий)
            let mut attempts = 0;
            loop {
                match fs::remove_file(&self.inverted_index_path) {
                    Ok(_) => break,
                    Err(_e) if attempts < 3 => {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        self.restore_from_backups(&backup_doc_path, &backup_inv_path);
                        let _ = fs::remove_file(&temp_inv_path);
                        return Err(format!("Помилка видалення старого інвертованого індексу після {} спроб: {}", attempts + 1, e));
                    }
                }
            }
        }

        if let Err(e) = fs::rename(&temp_inv_path, &self.inverted_index_path) {
            // При помилці відновлюємо з резервних копій
            self.restore_from_backups(&backup_doc_path, &backup_inv_path);
            return Err(format!("Помилка переміщення інвертованого індексу: {}", e));
        }

        println!("🧹 Очищення резервних копій...");
        
        // Етап 5: Видаляємо резервні копії після успішного збереження
        let _ = fs::remove_file(&backup_doc_path);
        let _ = fs::remove_file(&backup_inv_path);

        println!("✅ Атомарне збереження індексів завершено успішно!");
        Ok(())
    }

    /// Виконує повне інкрементне оновлення індексів з атомарним збереженням
    pub fn perform_incremental_update_atomically(
        &self,
        folder_path: &str,
    ) -> Result<UpdateStats, String> {
        let now: DateTime<Local> = Local::now();
        let time_str = now.format("%H:%M:%S").to_string();
        println!("🚀 [{time_str}] Початок інкрементного оновлення з атомарним збереженням...");
        
        // Створюємо lock файл для запобігання одночасному доступу
        let lock_file_path = "index_update.lock";
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_file_path)
            .map_err(|e| format!("Помилка створення lock файлу: {}", e))?;
        
        // Намагаємося отримати ексклюзивний lock
        match lock_file.try_lock_exclusive() {
            Ok(_) => {
                println!("🔒 [{time_str}] Отримано ексклюзивний доступ до оновлення індексів");
            },
            Err(_) => {
                return Err("⚠️ Інший процес вже оновлює індекси. Очікуйте завершення.".to_string());
            }
        }
        
        // Виконуємо оновлення в блоку, щоб гарантувати звільнення lock'у
        let result = self.perform_update_with_lock(folder_path);
        
        // Lock файл буде автоматично розблокований при виході зі scope
        // Але ми також можемо явно його видалити
        let _ = fs::remove_file(lock_file_path);
        
        result
    }
    
    /// Внутрішня функція для виконання оновлення під lock'ом
    fn perform_update_with_lock(&self, folder_path: &str) -> Result<UpdateStats, String> {

        let now: DateTime<Local> = Local::now();
        let _time_str = now.format("%H:%M:%S").to_string();
        
        // Завантажуємо існуючі індекси
        let existing_doc_index = if Path::new(&self.documents_index_path).exists() {
            match DocumentIndex::load_from_file(&self.documents_index_path) {
                Ok(index) => Some(index),
                Err(e) => {
                    println!("⚠️ Не вдалося завантажити існуючий індекс документів: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let existing_inv_index = if Path::new(&self.inverted_index_path).exists() {
            match InvertedIndex::load_from_file(&self.inverted_index_path) {
                Ok(index) => Some(index),
                Err(e) => {
                    println!("⚠️ Не вдалося завантажити існуючий інвертований індекс: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Виконуємо інкрементну обробку
        let mut processor = FolderProcessor::new();
        let updated_doc_index = processor.process_folder_incremental(folder_path, existing_doc_index)?;

        let stats = UpdateStats {
            processed: processor.processed_files,
            skipped: processor.skipped_files,
            deleted: processor.deleted_files,
            renamed: processor.renamed_indices.len(),
        };

        // Якщо є зміни, оновлюємо індекси атомарно
        if stats.has_changes() {
            let update_time: DateTime<Local> = Local::now();
            let update_time_str = update_time.format("%H:%M:%S").to_string();
            
            // Визначаємо що саме потрібно оновити
            let content_changed = !processor.new_or_updated_indices.is_empty() || !processor.deleted_file_paths.is_empty();
            let only_renamed = processor.new_or_updated_indices.is_empty() && processor.deleted_file_paths.is_empty() && !processor.renamed_indices.is_empty();
            
            if only_renamed {
                println!("📊 [{update_time_str}] Виявлено тільки перейменування файлів - оновлюємо лише документний індекс...");
            } else {
                println!("📊 [{update_time_str}] Зміни виявлено, оновлення індексів...");
            }

            // Оновлюємо інвертований індекс тільки для дійсно змінених документів
            // Виключаємо перейменовані файли, оскільки їх контент не змінився
            let mut updated_inv_index = if content_changed {
                if !processor.renamed_indices.is_empty() {
                    println!("ℹ️  Перейменовано {} файлів - інвертований індекс не потребує оновлення для них", processor.renamed_indices.len());
                }
                println!("🔄 Оновлення інвертованого індексу для {} нових/змінених документів", processor.new_or_updated_indices.len());
                
                // Детальний лог документів для відстеження
                for &idx in &processor.new_or_updated_indices {
                    if let Some(doc) = updated_doc_index.documents.get(idx) {
                        println!("   - Документ {}: {}", idx, doc.file_name);
                    } else {
                        println!("   - Документ {}: НЕ ЗНАЙДЕНО В DOCUMENT_INDEX!", idx);
                    }
                }
                
                // Критично важливо: передаємо правильний existing_inv_index
                let current_inv_index = existing_inv_index.clone();
                InvertedIndex::build_incremental(current_inv_index, &updated_doc_index, &processor.new_or_updated_indices)
            } else {
                // Якщо тільки перейменування, просто оновлюємо загальну кількість документів
                println!("📝 Тільки перейменування - оновлюємо лише кількість документів");
                let mut inv_index = existing_inv_index.unwrap_or_else(|| {
                    println!("⚠️  Створення нового порожнього інвертованого індексу");
                    let mut empty_idx = InvertedIndex::new();
                    empty_idx.total_documents = updated_doc_index.total_documents;
                    empty_idx
                });
                inv_index.total_documents = updated_doc_index.total_documents;
                println!("📊 Оновлено total_documents в інвертованому індексі: {}", inv_index.total_documents);
                inv_index
            };

            // Видаляємо записи про видалені файли з інвертованого індексу
            if !processor.deleted_file_paths.is_empty() {
                println!("🗑️  Очищення інвертованого індексу від {} видалених файлів", processor.deleted_file_paths.len());
                updated_inv_index.remove_deleted_documents_by_paths(&processor.deleted_file_paths, &updated_doc_index);
            }

            // ❌ ВИМКНЕНО: Повне перебудування занадто повільне і блокує файли
            // Замість цього використовуємо інкрементне оновлення
            // println!("🔄 Повне перебудування інвертованого індексу після сортування документів...");
            // updated_inv_index = InvertedIndex::rebuild_from_scratch(&updated_doc_index);

            // Очищуємо дублікати записів після оновлення
            let duplicates_removed = updated_inv_index.remove_duplicate_entries();
            if duplicates_removed > 0 {
                println!("🧹 Видалено {} дублікатів записів після оновлення індексу", duplicates_removed);
            }

            // Атомарно зберігаємо обидва індекси
            self.save_indices_atomically(&updated_doc_index, &updated_inv_index)?;
            
            let end_time: DateTime<Local> = Local::now();
            let end_time_str = end_time.format("%H:%M:%S").to_string();
            println!("✅ [{end_time_str}] Інкрементне оновлення завершено успішно!");
        } else {
            println!("ℹ️ Зміни не виявлено, індекси залишаються незмінними");
        }

        Ok(stats)
    }

    /// Збереження індексу документів в тимчасовий файл
    fn save_document_index_to_temp(&self, temp_path: &str, index: &DocumentIndex) -> Result<(), String> {
        use std::io::{BufWriter};

        let file = fs::File::create(temp_path)
            .map_err(|e| format!("Помилка створення тимчасового файлу індексу документів: {}", e))?;

        let writer = BufWriter::with_capacity(1024 * 1024, file); // 1MB буфер

        serde_json::to_writer_pretty(writer, index)
            .map_err(|e| {
                // Видаляємо пошкоджений тимчасовий файл
                let _ = fs::remove_file(temp_path);
                format!("Помилка серіалізації індексу документів: {}", e)
            })?;

        Ok(())
    }

    /// Збереження інвертованого індексу в тимчасовий файл
    fn save_inverted_index_to_temp(&self, temp_path: &str, index: &InvertedIndex) -> Result<(), String> {
        let json = serde_json::to_string(index)
            .map_err(|e| format!("Помилка серіалізації інвертованого індексу: {}", e))?;

        fs::write(temp_path, json)
            .map_err(|e| {
                // Видаляємо пошкоджений тимчасовий файл
                let _ = fs::remove_file(temp_path);
                format!("Помилка запису тимчасового файлу інвертованого індексу: {}", e)
            })?;

        Ok(())
    }

    /// Відновлення з резервних копій при помилках
    fn restore_from_backups(&self, backup_doc_path: &str, backup_inv_path: &str) {
        println!("🔄 Відновлення з резервних копій через помилку...");
        
        if Path::new(backup_doc_path).exists() {
            if let Err(e) = fs::rename(backup_doc_path, &self.documents_index_path) {
                println!("❌ Помилка відновлення індексу документів: {}", e);
            }
        }
        
        if Path::new(backup_inv_path).exists() {
            if let Err(e) = fs::rename(backup_inv_path, &self.inverted_index_path) {
                println!("❌ Помилка відновлення інвертованого індексу: {}", e);
            }
        }
        
        println!("✅ Відновлення завершено");
    }

    /// Перевірка цілісності індексів
    pub fn validate_indices(&self) -> Result<bool, String> {
        println!("🔍 Перевірка цілісності індексів...");

        // Перевіряємо існування файлів
        if !Path::new(&self.documents_index_path).exists() {
            return Err("Файл індексу документів не існує".to_string());
        }

        if !Path::new(&self.inverted_index_path).exists() {
            return Err("Файл інвертованого індексу не існує".to_string());
        }

        // Завантажуємо та перевіряємо індекси
        let doc_index = DocumentIndex::load_from_file(&self.documents_index_path)
            .map_err(|e| format!("Помилка завантаження індексу документів: {}", e))?;

        let mut inv_index = InvertedIndex::load_from_file(&self.inverted_index_path)
            .map_err(|e| format!("Помилка завантаження інвертованого індексу: {}", e))?;

        // Перевіряємо відповідність кількості документів
        let mut needs_repair = false;
        if doc_index.total_documents != inv_index.total_documents {
            println!("⚠️ Невідповідність кількості документів: doc_index={}, inv_index={}", 
                     doc_index.total_documents, inv_index.total_documents);
            inv_index.total_documents = doc_index.total_documents;
            needs_repair = true;
        }

        // Очищуємо дублікати та невалідні записи
        let duplicates_removed = inv_index.remove_duplicate_entries();
        if duplicates_removed > 0 {
            needs_repair = true;
        }
        
        let cleaned = inv_index.cleanup();
        if cleaned > 0 {
            needs_repair = true;
        }

        // Якщо потрібно виправлення, зберігаємо оновлений індекс
        if needs_repair {
            println!("🔧 Виправлення виявлених проблем інвертованого індексу...");
            if let Err(e) = inv_index.save_to_file(&self.inverted_index_path) {
                return Err(format!("Не вдалося зберегти виправлений індекс: {}", e));
            }
            println!("✅ Проблеми виправлено та збережено");
        }

        println!("✅ Індекси валідні та синхронізовані");
        Ok(true)
    }
    
    /// Метод для повного ребілду інвертованого індексу при критичних помилках
    pub fn rebuild_inverted_index_if_needed(&self) -> Result<bool, String> {
        println!("🔧 Перевірка необхідності перебудування інвертованого індексу...");
        
        // Завантажуємо індекс документів
        let doc_index = DocumentIndex::load_from_file(&self.documents_index_path)
            .map_err(|e| format!("Помилка завантаження індексу документів: {}", e))?;
            
        // Спробуємо завантажити інвертований індекс
        let inv_index_result = InvertedIndex::load_from_file(&self.inverted_index_path);
        
        let should_rebuild = match inv_index_result {
            Ok(inv_index) => {
                // Перевіряємо критичні невідповідності
                let docs_count_diff = (doc_index.total_documents as i32 - inv_index.total_documents as i32).abs();
                if docs_count_diff > 10 {
                    println!("⚠️ Критична невідповідність кількості документів: різниця {} документів", docs_count_diff);
                    true
                } else if inv_index.word_to_docs.is_empty() && doc_index.total_documents > 0 {
                    println!("⚠️ Інвертований індекс порожній при наявності документів");
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                println!("⚠️ Критична помилка інвертованого індексу: {}", e);
                true
            }
        };
        
        if should_rebuild {
            println!("🔄 Повне перебудування інвертованого індексу...");
            let new_inv_index = InvertedIndex::rebuild_from_scratch(&doc_index);
            
            // Зберігаємо новий індекс
            self.save_indices_atomically(&doc_index, &new_inv_index)?;
            
            println!("✅ Інвертований індекс успішно перебудовано");
            Ok(true)
        } else {
            println!("✅ Перебудування не потрібне");
            Ok(false)
        }
    }

    /// Очищення всіх тимчасових та резервних файлів
    pub fn cleanup_temp_files(&self) {
        let temp_files = vec![
            format!("{}.atomic_temp", self.documents_index_path),
            format!("{}.atomic_temp", self.inverted_index_path),
            format!("{}.atomic_backup", self.documents_index_path),
            format!("{}.atomic_backup", self.inverted_index_path),
            format!("{}.tmp", self.documents_index_path),
            format!("{}.tmp", self.inverted_index_path),
            format!("{}.backup", self.documents_index_path),
            format!("{}.backup", self.inverted_index_path),
        ];

        for temp_file in temp_files {
            if Path::new(&temp_file).exists() {
                if let Err(e) = fs::remove_file(&temp_file) {
                    println!("⚠️ Не вдалося видалити тимчасовий файл {}: {}", temp_file, e);
                } else {
                    println!("🧹 Видалено тимчасовий файл: {}", temp_file);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct UpdateStats {
    pub processed: usize,
    pub skipped: usize,
    pub deleted: usize,
    pub renamed: usize,
}

impl UpdateStats {
    pub fn has_changes(&self) -> bool {
        self.processed > 0 || self.deleted > 0 || self.renamed > 0
    }
}

impl std::fmt::Display for UpdateStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "оброблено: {}, пропущено: {}, видалено: {}, перейменовано: {}",
            self.processed, self.skipped, self.deleted, self.renamed
        )
    }
}