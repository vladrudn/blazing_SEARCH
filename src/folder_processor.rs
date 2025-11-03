use std::path::Path;
use walkdir::{WalkDir, DirEntry};
use regex::Regex;
use once_cell::sync::Lazy;
use crate::docx_parser::parse_docx_with_structure;
use crate::document_record::{DocumentRecord, DocumentIndex};

// Регулярний вираз для пошуку дати у форматі DD.MM.YYYY
static DATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{2})\.(\d{2})\.(\d{4})").unwrap()
});

pub struct FolderProcessor {
    pub processed_files: usize,
    pub skipped_files: usize,
    pub deleted_files: usize,
    pub errors: Vec<String>,
    pub new_or_updated_indices: Vec<usize>,
    pub deleted_file_paths: Vec<String>, // Змінено: зберігаємо шляхи файлів замість індексів
    pub renamed_indices: Vec<usize>, // Індекси перейменованих документів (не потребують переіндексації)
}

impl FolderProcessor {
    pub fn new() -> Self {
        Self {
            processed_files: 0,
            skipped_files: 0,
            deleted_files: 0,
            errors: Vec::new(),
            new_or_updated_indices: Vec::new(),
            deleted_file_paths: Vec::new(), // Змінено: зберігаємо шляхи файлів замість індексів
            renamed_indices: Vec::new(),
        }
    }

    // Парсинг дати з назви файлу у форматі DD.MM.YYYY
    fn extract_date_from_filename(&self, file_path: &str) -> Option<(u32, u32, u32)> {
        let filename = Path::new(file_path)
            .file_name()?
            .to_str()?;

        if let Some(captures) = DATE_REGEX.captures(filename) {
            let day: u32 = captures.get(1)?.as_str().parse().ok()?;
            let month: u32 = captures.get(2)?.as_str().parse().ok()?;
            let year: u32 = captures.get(3)?.as_str().parse().ok()?;

            // Перевірка валідності дати
            if day >= 1 && day <= 31 && month >= 1 && month <= 12 && year >= 1900 {
                Some((year, month, day))
            } else {
                None
            }
        } else {
            None
        }
    }

    // Порівняння дат для сортування (від нової до старої)
    fn compare_dates(&self, date1: Option<(u32, u32, u32)>, date2: Option<(u32, u32, u32)>) -> std::cmp::Ordering {
        match (date1, date2) {
            (Some((y1, m1, d1)), Some((y2, m2, d2))) => {
                // Для сортування від нових до старих: більша дата має йти першою
                // Спочатку порівнюємо роки (більший рік має йти першим)
                match y2.cmp(&y1) {
                    std::cmp::Ordering::Equal => {
                        // Якщо роки рівні, порівнюємо місяці (більший місяць має йти першим)
                        match m2.cmp(&m1) {
                            std::cmp::Ordering::Equal => {
                                // Якщо місяці рівні, порівнюємо дні (більший день має йти першим)
                                d2.cmp(&d1)
                            },
                            other => other
                        }
                    },
                    other => other
                }
            }
            (Some(_), None) => std::cmp::Ordering::Less,    // Файли з датою йдуть перед файлами без дати
            (None, Some(_)) => std::cmp::Ordering::Greater, // Файли без дати йдуть після файлів з датою
            (None, None) => std::cmp::Ordering::Equal      // Файли без дат сортуються як рівні
        }
    }

    pub fn process_folder_incremental(&mut self, folder_path: &str, existing_index: Option<DocumentIndex>) -> Result<DocumentIndex, String> {
        let folder = Path::new(folder_path);

        if !folder.exists() {
            return Err(format!("Папка не існує: {}", folder_path));
        }

        if !folder.is_dir() {
            return Err(format!("Шлях не є папкою: {}", folder_path));
        }

        let mut index = existing_index.unwrap_or_else(|| DocumentIndex::new());

        // Папки виключення
        let excluded_folders = vec![".git", "ЕРДР (не виключені)"];

        // Створюємо мапу існуючих документів для швидкого пошуку
        let mut existing_docs_map = index.documents.iter()
            .enumerate()
            .map(|(i, doc)| (doc.file_path.clone(), (i, doc.last_modified)))
            .collect::<std::collections::HashMap<String, (usize, u64)>>();
            
        // Створюємо мапу для виявлення потенційних перейменувань
        // Ключ: (розмір_файлу, час_модифікації), значення: (індекс, шлях)
        let mut size_time_to_doc = std::collections::HashMap::new();
        for (i, doc) in index.documents.iter().enumerate() {
            if let Ok(metadata) = std::fs::metadata(&doc.file_path) {
                let size = metadata.len();
                let modified = metadata.modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                size_time_to_doc.insert((size, modified), (i, doc.file_path.clone()));
            }
        }

        // Створюємо сет існуючих файлів для виявлення видалених
        let mut found_files = std::collections::HashSet::new();

        println!("🔍 Пошук DOCX файлів у папці: {}", folder_path);

        for entry in WalkDir::new(folder_path)
            .follow_links(false)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Перевіряємо чи потрібно пропустити цей запис
            if Self::should_skip_entry_static(&entry, &excluded_folders) {
                continue;
            }

            // Перевіряємо чи це DOCX файл
            if path.is_file() && self.is_docx_file(path) {
                let file_path = path.to_string_lossy().to_string();
                found_files.insert(file_path.clone());

                // Отримуємо метадані файлу
                match std::fs::metadata(&file_path) {
                    Ok(metadata) => {
                        let file_last_modified = metadata.modified()
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        // Перевіряємо чи потрібно оновлювати файл
                        let should_process = if let Some((doc_index, existing_modified)) = existing_docs_map.get(&file_path) {
                            if file_last_modified > *existing_modified {
                                // Файл змінився, видаляємо старий запис
                                index.total_words -= index.documents[*doc_index].word_count;
                                println!("🔄 Оновлення файлу: {}", path.file_name().unwrap_or_default().to_string_lossy());
                                true
                            } else {
                                // Файл не змінився
                                self.skipped_files += 1;
                                false
                            }
                        } else {
                            // Перевіряємо чи це може бути перейменований файл
                            let file_size = metadata.len();
                            if let Some((old_doc_index, old_path)) = size_time_to_doc.get(&(file_size, file_last_modified)) {
                                if old_path != &file_path {
                                    // Знайдено потенційне перейменування
                                    println!("🔄 Виявлено перейменування: {} -> {}", 
                                             std::path::Path::new(old_path).file_name().unwrap_or_default().to_string_lossy(),
                                             path.file_name().unwrap_or_default().to_string_lossy());
                                    
                                    // Оновлюємо шлях в існуючому документі
                                    index.documents[*old_doc_index].file_path = file_path.clone();
                                    
                                    // Видаляємо зі старої мапи та додаємо в нову
                                    existing_docs_map.remove(old_path);
                                    existing_docs_map.insert(file_path.clone(), (*old_doc_index, file_last_modified));
                                    
                                    // Позначаємо як перейменований (не потребує переіндексації інвертованого індексу)
                                    self.renamed_indices.push(*old_doc_index);
                                    
                                    false // Не потребує повторної обробки
                                } else {
                                    // Новий файл
                                    true
                                }
                            } else {
                                // Новий файл
                                true
                            }
                        };

                        if should_process {
                            match self.process_docx_file(&file_path) {
                                Ok(new_document) => {
                                    let doc_index = if let Some((doc_index, _)) = existing_docs_map.remove(&file_path) {
                                        // Замінюємо існуючий документ на місці
                                        index.documents[doc_index] = new_document;
                                        doc_index
                                    } else {
                                        // Додаємо новий документ
                                        index.documents.push(new_document);
                                        index.documents.len() - 1
                                    };

                                    // Оновлюємо загальну статистику
                                    index.total_words += index.documents[doc_index].word_count;
                                    index.total_documents = index.documents.len();

                                    // Записуємо індекс нового/оновленого документа
                                    self.new_or_updated_indices.push(doc_index);
                                    self.processed_files += 1;
                                    println!("✅ Оброблено: {} ({} слів)",
                                             path.file_name().unwrap_or_default().to_string_lossy(),
                                             index.documents[doc_index].word_count
                                    );
                                }
                                Err(error) => {
                                    let error_msg = format!("Помилка обробки {}: {}", file_path, error);
                                    self.errors.push(error_msg.clone());
                                    println!("❌ {}", error_msg);
                                }
                            }
                        }
                    }
                    Err(error) => {
                        let error_msg = format!("Помилка отримання метаданих {}: {}", file_path, error);
                        self.errors.push(error_msg.clone());
                        println!("❌ {}", error_msg);
                    }
                }
            }
        }

        // Видаляємо документи для файлів, які більше не існують
        let mut files_to_remove = Vec::new();
        for (i, doc) in index.documents.iter().enumerate() {
            if !found_files.contains(&doc.file_path) {
                files_to_remove.push((i, doc.file_path.clone()));
            }
        }

        // Зберігаємо шляхи файлів, які будуть видалені, ДО видалення
        for (_pos, file_path) in &files_to_remove {
            self.deleted_file_paths.push(file_path.clone());
        }

        // Сортуємо індекси в зворотному порядку, щоб видаляти з кінця
        files_to_remove.sort_by(|a, b| b.0.cmp(&a.0));

        for (pos, file_path) in files_to_remove {
            let removed_doc = index.documents.remove(pos);
            index.total_words -= removed_doc.word_count;
            self.deleted_files += 1;
            println!("🗑️  Видалено: {}", std::path::Path::new(&file_path).file_name().unwrap_or_default().to_string_lossy());
        }

        // Створюємо мапу старих індексів для оновлення після сортування
        let old_to_new_index_map: std::collections::HashMap<usize, usize> = if !self.new_or_updated_indices.is_empty() || !self.renamed_indices.is_empty() {
            // Створюємо мапу файлових шляхів до індексів перед сортуванням
            let file_path_to_old_index: std::collections::HashMap<String, usize> =
                index.documents.iter().enumerate()
                    .map(|(i, doc)| (doc.file_path.clone(), i))
                    .collect();

            // ❌ ВИМКНЕНО: Сортування змінює індекси документів,
            // що вимагає повного перебудування інвертованого індексу (занадто повільно)
            // Сортуємо документи за датою з назви файлу (від нових до старих)
            // index.documents.sort_by(|a, b| {
            //     let date_a = self.extract_date_from_filename(&a.file_path);
            //     let date_b = self.extract_date_from_filename(&b.file_path);
            //     self.compare_dates(date_a, date_b)
            // });

            // Створюємо мапу нових індексів
            let file_path_to_new_index: std::collections::HashMap<String, usize> =
                index.documents.iter().enumerate()
                    .map(|(i, doc)| (doc.file_path.clone(), i))
                    .collect();

            // Створюємо мапу переходу зі старих індексів на нові
            file_path_to_old_index.iter()
                .filter_map(|(file_path, &old_idx)| {
                    file_path_to_new_index.get(file_path)
                        .map(|&new_idx| (old_idx, new_idx))
                })
                .collect()
        } else {
            // ❌ ВИМКНЕНО: Сортування змінює індекси документів,
            // що вимагає повного перебудування інвертованого індексу (занадто повільно)
            // Сортуємо документи за датою з назви файлу (від нових до старих)
            // index.documents.sort_by(|a, b| {
            //     let date_a = self.extract_date_from_filename(&a.file_path);
            //     let date_b = self.extract_date_from_filename(&b.file_path);
            //     self.compare_dates(date_a, date_b)
            // });
            std::collections::HashMap::new()
        };

        // Оновлюємо індекси нових/оновлених документів після сортування
        self.new_or_updated_indices = self.new_or_updated_indices.iter()
            .filter_map(|&old_idx| old_to_new_index_map.get(&old_idx).copied())
            .collect();

        // Оновлюємо індекси перейменованих документів після сортування  
        self.renamed_indices = self.renamed_indices.iter()
            .filter_map(|&old_idx| old_to_new_index_map.get(&old_idx).copied())
            .collect();

        // Оновлюємо загальну кількість документів
        index.total_documents = index.documents.len();

        // Оновлюємо timestamp індексації
        index.indexed_at = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        println!("\n📊 Результати інкрементної індексації:");
        println!("   - Оброблено файлів: {}", self.processed_files);
        println!("   - Пропущено незмінених: {}", self.skipped_files);
        println!("   - Перейменовано файлів: {}", self.renamed_indices.len());
        println!("   - Видалено файлів: {}", self.deleted_files);
        println!("   - Помилок: {}", self.errors.len());
        println!("   - Загальна кількість слів: {}", index.total_words);

        if !self.errors.is_empty() {
            println!("\n⚠️  ПОМИЛКИ:");
            for error in &self.errors {
                println!("{}", error);
            }
        }

        Ok(index)
    }

    fn is_docx_file(&self, path: &Path) -> bool {
        // Пропускаємо тимчасові файли Office (~$)
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with("~$") {
                return false;
            }
        }

        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "docx")
            .unwrap_or(false)
    }

    fn process_docx_file(&self, file_path: &str) -> Result<DocumentRecord, String> {
        // Використовуємо новий парсер зі збереженням структури
        let paragraphs = parse_docx_with_structure(file_path)?;
        DocumentRecord::new_with_paragraphs(file_path.to_string(), paragraphs)
    }

    fn should_skip_entry_static(entry: &DirEntry, excluded_folders: &[&str]) -> bool {
        let path = entry.path();
        let path_str = path.to_string_lossy().to_lowercase();

        // Перевіряємо чи це одна з виключених папок
        if entry.file_type().is_dir() {
            if let Some(folder_name) = path.file_name() {
                let folder_name_str = folder_name.to_string_lossy().to_lowercase();
                for excluded in excluded_folders {
                    if folder_name_str == excluded.to_lowercase() {
                        return true;
                    }
                }
            }
        }

        // Перевіряємо чи знаходиться у виключеній папці
        for excluded in excluded_folders {
            let excluded_lower = excluded.to_lowercase();
            if path_str.contains(&format!("\\{}", excluded_lower)) ||
                path_str.contains(&format!("/{}", excluded_lower)) {
                return true;
            }
        }

        false
    }
}