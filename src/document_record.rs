use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use std::time::SystemTime;
use std::io::{BufReader, BufWriter};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentRecord {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub last_modified: u64, // Unix timestamp
    pub created: u64,       // Unix timestamp
    pub content: Vec<String>,
    pub word_count: usize,
    pub paragraph_count: usize,
}

impl DocumentRecord {
    pub fn new(
        file_path: String,
        content: Vec<String>,
    ) -> Result<Self, String> {
        let path = Path::new(&file_path);

        let metadata = fs::metadata(&file_path)
            .map_err(|e| format!("Помилка отримання метаданих файлу {}: {}", file_path, e))?;

        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        let last_modified = metadata.modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let created = metadata.created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let word_count = content.iter()
            .map(|paragraph| paragraph.split_whitespace().count())
            .sum();

        let paragraph_count = content.len();

        Ok(DocumentRecord {
            file_path,
            file_name,
            file_size: metadata.len(),
            last_modified,
            created,
            content,
            word_count,
            paragraph_count,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentIndex {
    pub documents: Vec<DocumentRecord>,
    pub total_documents: usize,
    pub total_words: usize,
    pub indexed_at: u64, // Unix timestamp
}

impl DocumentIndex {
    pub fn new() -> Self {
        let indexed_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        DocumentIndex {
            documents: Vec::new(),
            total_documents: 0,
            total_words: 0,
            indexed_at,
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        println!("💾 Збереження індексу в файл: {}", path);

        // Атомарне збереження через тимчасовий файл
        let temp_path = format!("{}.tmp", path);
        let backup_path = format!("{}.backup", path);

        // Створюємо резервну копію існуючого файлу якщо він є
        if Path::new(path).exists() {
            fs::copy(path, &backup_path)
                .map_err(|e| format!("Помилка створення резервної копії: {}", e))?;
        }

        // Зберігаємо в тимчасовий файл
        {
            let file = std::fs::File::create(&temp_path)
                .map_err(|e| format!("Помилка створення тимчасового файлу: {}", e))?;

            let writer = BufWriter::with_capacity(1024 * 1024, file); // 1MB буфер

            serde_json::to_writer_pretty(writer, self)
                .map_err(|e| {
                    // Видаляємо пошкоджений тимчасовий файл
                    let _ = fs::remove_file(&temp_path);
                    format!("Помилка серіалізації JSON: {}", e)
                })?;
        } // writer закривається тут, дані записуються на диск

        // Атомарно переміщуємо тимчасовий файл на місце основного
        fs::rename(&temp_path, path)
            .map_err(|e| {
                // Якщо переміщення не вдалося, спробуємо відновити з резервної копії
                if Path::new(&backup_path).exists() {
                    let _ = fs::rename(&backup_path, path);
                }
                format!("Помилка переміщення тимчасового файлу: {}", e)
            })?;

        // Видаляємо резервну копію після успішного збереження
        if Path::new(&backup_path).exists() {
            let _ = fs::remove_file(&backup_path);
        }

        println!("✅ Індекс успішно збережено");
        Ok(())
    }

    pub fn load_from_file(file_path: &str) -> Result<Self, String> {
        println!("📂 Завантаження індексу з файлу: {}", file_path);

        let backup_path = format!("{}.backup", file_path);

        // Спочатку пробуємо завантажити основний файл
        let index = Self::try_load_file(file_path);
        
        match index {
            Ok(idx) => {
                // Перевіряємо цілісність індексу
                if Self::validate_index(&idx) {
                    println!("✅ Завантажено {} документів", idx.total_documents);
                    return Ok(idx);
                } else {
                    println!("⚠️  Основний індекс пошкоджений, спробуємо резервну копію...");
                }
            }
            Err(e) => {
                println!("⚠️  Помилка завантаження основного індексу: {}", e);
                println!("🔄 Спробуємо резервну копію...");
            }
        }

        // Якщо основний файл пошкоджений, пробуємо резервну копію
        if Path::new(&backup_path).exists() {
            match Self::try_load_file(&backup_path) {
                Ok(backup_idx) => {
                    if Self::validate_index(&backup_idx) {
                        println!("✅ Завантажено з резервної копії {} документів", backup_idx.total_documents);
                        // Відновлюємо основний файл з резервної копії
                        if let Err(e) = fs::copy(&backup_path, file_path) {
                            println!("⚠️  Не вдалося відновити основний файл: {}", e);
                        }
                        return Ok(backup_idx);
                    } else {
                        println!("❌ Резервна копія також пошкоджена");
                    }
                }
                Err(e) => {
                    println!("❌ Помилка завантаження резервної копії: {}", e);
                }
            }
        }

        Err("Не вдалося завантажити індекс: всі файли пошкоджені або відсутні".to_string())
    }

    fn try_load_file(file_path: &str) -> Result<Self, String> {
        let file = std::fs::File::open(file_path)
            .map_err(|e| format!("Помилка відкриття файлу: {}", e))?;

        let reader = BufReader::with_capacity(1024 * 1024, file); // 1MB буфер

        serde_json::from_reader(reader)
            .map_err(|e| format!("Помилка парсингу JSON: {}", e))
    }

    fn validate_index(index: &Self) -> bool {
        // Базові перевірки цілісності
        if index.documents.is_empty() && index.total_documents > 0 {
            println!("❌ Невідповідність: total_documents > 0, але documents порожній");
            return false;
        }

        if index.documents.len() != index.total_documents {
            println!("❌ Невідповідність: len(documents) != total_documents");
            return false;
        }

        // Перевіряємо, що кожен документ має валідні дані
        for (i, doc) in index.documents.iter().enumerate() {
            if doc.file_path.is_empty() {
                println!("❌ Документ {} має порожній file_path", i);
                return false;
            }
            
            if doc.content.is_empty() && doc.paragraph_count > 0 {
                println!("❌ Документ {} має paragraph_count > 0, але content порожній", i);
                return false;
            }

            if doc.content.len() != doc.paragraph_count {
                println!("❌ Документ {} має невідповідність len(content) != paragraph_count", i);
                return false;
            }
        }

        true
    }
}