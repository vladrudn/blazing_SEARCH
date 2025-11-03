use crate::document_record::DocumentIndex;
use crate::inverted_index::InvertedIndex;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

static WORD_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b[\p{L}\p{N}]+\b").unwrap());

// Регулярний вираз для пошуку дати у форматі DD.MM.YYYY
static DATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{2})\.(\d{2})\.(\d{4})").unwrap()
});

static UKRAINIAN_VOWELS: &str = "аеєиіїоуюяь";

// Словник слів для припинення пошуку в файлах "особовий*"
static PERSONAL_FILE_STOP_WORDS: &[&str] = &[
    "старш", "молодш", "солдат", "сержант", "штаб", "лейтенант", "майор", "матрос"
];

#[derive(Debug, Clone)]
pub struct SearchEngineMatch {
    pub context: String,
    pub position: usize,
}

use crate::document_record::Paragraph;

#[derive(Debug, Clone)]
pub struct SearchEngineResult {
    pub file_name: String,
    pub file_path: String,
    pub matches: Vec<SearchEngineMatch>,
    pub all_paragraphs: Vec<Paragraph>,
    pub file_size: u64,
    pub last_modified: u64,
}

#[derive(Debug)]
pub enum SearchMode {
    Quick,
    Full,
    Remaining,
}

pub struct SearchEngine {
    data: Mutex<SearchEngineData>,
}

struct SearchEngineData {
    index: DocumentIndex,
    inverted_index: Option<InvertedIndex>,
}

// Функція для перевірки чи ПОЧИНАЄТЬСЯ параграф з заборонених слів для особових файлів
fn starts_with_personal_stop_words(paragraph: &str) -> bool {
    let binding = paragraph.to_lowercase();
    let lower_paragraph = binding.trim();
    PERSONAL_FILE_STOP_WORDS.iter().any(|stop_word| lower_paragraph.starts_with(stop_word))
}


impl SearchEngine {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(SearchEngineData {
                index: DocumentIndex::new(),
                inverted_index: None,
            }),
        }
    }

    /// Витягує дату з назви файлу у форматі DD.MM.YYYY
    fn extract_date_from_filename(file_path: &str) -> Option<(u32, u32, u32)> {
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

    /// Порівняння дат для сортування (від нової до старої)
    fn compare_dates(date1: Option<(u32, u32, u32)>, date2: Option<(u32, u32, u32)>) -> std::cmp::Ordering {
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
                            other => other,
                        }
                    },
                    other => other,
                }
            },
            (Some(_), None) => std::cmp::Ordering::Less, // Файли з датою йдуть перед файлами без дати
            (None, Some(_)) => std::cmp::Ordering::Greater, // Файли без дати йдуть після файлів з датою
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    pub fn load_from_file(&mut self, index_path: &str) -> Result<(), String> {
        let content = fs::read_to_string(index_path)
            .map_err(|e| format!("Помилка читання індексу: {}", e))?;

        let index: DocumentIndex =
            serde_json::from_str(&content).map_err(|e| format!("Помилка парсингу JSON: {}", e))?;

        // ❌ НЕ сортуємо документи тут, бо це зламає інвертований індекс!
        // Замість цього сортуємо РЕЗУЛЬТАТИ ПОШУКУ в методі search()

        // Спробуємо завантажити інвертований індекс
        let inverted_path = "inverted_index.json";
        let inverted_index = if std::path::Path::new(inverted_path).exists() {
            InvertedIndex::load_from_file(inverted_path).ok()
        } else {
            None
        };

        // Оновлюємо дані з блокуванням
        let mut data = self.data.lock()
            .map_err(|e| format!("Помилка блокування даних: {}", e))?;
        data.index = index;
        data.inverted_index = inverted_index;

        Ok(())
    }

    pub fn reload(&self, index_path: &str) -> Result<(), String> {
        let content = fs::read_to_string(index_path)
            .map_err(|e| format!("Помилка читання індексу: {}", e))?;

        let index: DocumentIndex =
            serde_json::from_str(&content).map_err(|e| format!("Помилка парсингу JSON: {}", e))?;

        // ❌ НЕ сортуємо документи тут, бо це зламає інвертований індекс!
        // Замість цього сортуємо РЕЗУЛЬТАТИ ПОШУКУ в методі search()

        // Спробуємо завантажити інвертований індекс
        let inverted_path = "inverted_index.json";
        let inverted_index = if std::path::Path::new(inverted_path).exists() {
            InvertedIndex::load_from_file(inverted_path).ok()
        } else {
            None
        };

        // Оновлюємо дані з блокуванням
        let mut data = self.data.lock()
            .map_err(|e| format!("Помилка блокування даних: {}", e))?;
        data.index = index;
        data.inverted_index = inverted_index;

        Ok(())
    }

    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        view_mode: Option<&str>,
    ) -> Result<Vec<SearchEngineResult>, String> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Спробуємо автоматично перезавантажити індекси якщо потрібно
        self.try_reload_indices_if_needed();

        let processed_query = self.process_search_query(query);
        let query_words = self.extract_search_words(&processed_query);

        if query_words.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        // Отримуємо доступ до даних
        let data = self.data.lock()
            .map_err(|e| format!("Помилка блокування даних: {}", e))?;

        // Використовуємо інвертований індекс якщо доступний
        if let Some(ref inverted_index) = data.inverted_index {
            println!("🔍 Пошук через інвертований індекс для слів: {:?}", query_words);
            let (inv_docs, inv_words) = inverted_index.get_stats();
            println!("📊 Інвертований індекс: {} документів, {} унікальних слів", inv_docs, inv_words);

            // Отримуємо кандидатів документів з інвертованого індексу
            let candidates = inverted_index.search_fast(&query_words, &data.index, &mode);
            println!("🎯 Знайдено {} кандидатів документів", candidates.len());

            for (doc_idx, paragraph_positions) in candidates {
                if doc_idx < data.index.documents.len() {
                    let document = &data.index.documents[doc_idx];
                    let paragraphs = document.get_paragraphs();
                    let mut document_matches = Vec::new();

                    // Перевіряємо тільки ті параграфи, які є в позиціях
                    for &pos in &paragraph_positions {
                        if pos < paragraphs.len() {
                            let paragraph = &paragraphs[pos];
                            let paragraph_lower = paragraph.text.to_lowercase();

                            // Пропускаємо параграфи які починаються з "Підстава" тільки в режимі "Витяг"
                            if view_mode == Some("fragments")
                                && paragraph_lower.trim().starts_with("підстава")
                            {
                                continue;
                            }

                            // Нормалізуємо параграф для пошуку (видаляємо апострофи)
                            let normalized_paragraph = paragraph_lower.replace('\'', "");

                            // Перевіряємо чи всі слова дійсно є в цьому нормалізованому параграфі
                            let has_all_words = query_words
                                .iter()
                                .all(|word| normalized_paragraph.contains(word));

                            if has_all_words {
                                // Перевіряємо близькість для ПІБ
                                let is_name_search =
                                    query_words.len() >= 2 && query_words.len() <= 3;

                                let proximity_check = !is_name_search
                                    || self
                                        .check_words_proximity(&normalized_paragraph, &query_words);

                                if proximity_check {
                                    // Знайдений параграф з персоною завжди додаємо (фільтрація наступних параграфів буде в JS)
                                    document_matches.push(SearchEngineMatch {
                                        context: paragraph.text.clone(),
                                        position: pos,
                                    });
                                }
                            }
                        }
                    }

                    if !document_matches.is_empty() {
                        results.push(SearchEngineResult {
                            file_name: document.file_name.clone(),
                            file_path: document.file_path.clone(),
                            matches: document_matches,
                            all_paragraphs: paragraphs,
                            file_size: document.file_size,
                            last_modified: document.last_modified,
                        });
                    }
                }
            }
        } else {
            println!("⚠️  Інвертований індекс не доступний, використовуємо звичайний пошук");
            // Звичайний пошук як резервний варіант
            for document in data.index.documents.iter() {
                let paragraphs = document.get_paragraphs();
                let mut document_matches = Vec::new();
                let mut has_any_match = false;

                for (pos, paragraph) in paragraphs.iter().enumerate() {
                    let paragraph_lower = paragraph.text.to_lowercase();

                    // Пропускаємо параграфи які починаються з "Підстава" тільки в режимі "Витяг"
                    if view_mode == Some("fragments")
                        && paragraph_lower.trim().starts_with("підстава")
                    {
                        continue;
                    }

                    // Нормалізуємо параграф для пошуку (видаляємо апострофи)
                    let normalized_paragraph = paragraph_lower.replace('\'', "");

                    let has_all_words = query_words
                        .iter()
                        .all(|word| normalized_paragraph.contains(word));

                    if has_all_words {
                        let is_name_search = query_words.len() >= 2 && query_words.len() <= 3;

                        let proximity_check = !is_name_search
                            || self.check_words_proximity(&normalized_paragraph, &query_words);

                        if proximity_check {
                            // Знайдений параграф з персоною завжди додаємо (фільтрація наступних параграфів буде в JS)
                            document_matches.push(SearchEngineMatch {
                                context: paragraph.text.clone(),
                                position: pos,
                            });
                            has_any_match = true;
                        }
                    }
                }

                if has_any_match {
                    results.push(SearchEngineResult {
                        file_name: document.file_name.clone(),
                        file_path: document.file_path.clone(),
                        matches: document_matches,
                        all_paragraphs: paragraphs,
                        file_size: document.file_size,
                        last_modified: document.last_modified,
                    });
                }
            }
        }

        // Сортуємо за датою з назви файлу (від нових до старих), потім за кількістю збігів
        results.sort_by(|a, b| {
            // Витягуємо дати з назв файлів
            let date_a = Self::extract_date_from_filename(&a.file_path);
            let date_b = Self::extract_date_from_filename(&b.file_path);

            // Порівнюємо за датою
            match Self::compare_dates(date_a, date_b) {
                std::cmp::Ordering::Equal => {
                    // Якщо дати однакові, сортуємо за кількістю збігів
                    b.matches.len().cmp(&a.matches.len())
                }
                other => other,
            }
        });

        Ok(results)
    }

    fn process_search_query(&self, query: &str) -> String {
        // Видаляємо апострофи
        let without_apostrophes = query.replace('\'', "");

        // Розбиваємо на слова та обробляємо стемінг
        let words: Vec<String> = without_apostrophes
            .split_whitespace()
            .map(|word| self.stem_word(word))
            .collect();

        words.join(" ")
    }

    fn extract_search_words(&self, query: &str) -> Vec<String> {
        WORD_REGEX
            .find_iter(query)
            .map(|m| m.as_str().to_lowercase())
            .collect()
    }

    fn check_words_proximity(&self, paragraph: &str, query_words: &[String]) -> bool {
        if query_words.len() < 2 {
            return true;
        }

        // Нормалізуємо параграф для пошуку (видаляємо апострофи)
        let normalized_paragraph = paragraph.to_lowercase().replace('\'', "");

        // Перевіряємо чи всі слова йдуть у правильному порядку з розумною відстанню
        let mut last_position = 0;

        for (i, word) in query_words.iter().enumerate() {
            if let Some(word_pos) = normalized_paragraph[last_position..].find(word) {
                let absolute_pos = last_position + word_pos;

                // Для першого слова встановлюємо початкову позицію
                if i == 0 {
                    last_position = absolute_pos + word.len();
                    continue;
                }

                // Для наступних слів перевіряємо відстань
                let distance = absolute_pos - last_position;

                // Дозволяємо до 15 символів між словами (для урахування відмінків і розділових знаків)
                // Це дозволить знайти "ДОНА Анатолія" при пошуку "дон анатол"
                if distance > 15 {
                    return false;
                }

                // Оновлюємо позицію для пошуку наступного слова
                last_position = absolute_pos + word.len();
            } else {
                // Слово не знайдено після попередніх - порядок порушено
                return false;
            }
        }

        true
    }

    fn stem_word(&self, word: &str) -> String {
        let word = word.to_lowercase();

        // Обробка слів з дефісом
        if word.contains('-') {
            let parts: Vec<String> = word
                .split('-')
                .map(|part| self.stem_word_part(part))
                .collect();
            return parts.join("-");
        }

        self.stem_word_part(&word)
    }

    fn stem_word_part(&self, word: &str) -> String {
        let mut result = word.to_string();

        // Видаляємо закінчення -ець
        if result.ends_with("ець") {
            result = result[..result.len() - "ець".len()].to_string();
        } else if result.ends_with("ця") {
            result = result[..result.len() - "ця".len()].to_string();
        } else if result.ends_with("цю") {
            result = result[..result.len() - "цю".len()].to_string();
        }

        // Видаляємо закінчення -ого
        if result.ends_with("ого") {
            result = result[..result.len() - "ого".len()].to_string();
        }
        if result.ends_with("ому") {
            result = result[..result.len() - "ому".len()].to_string();
        }

        // Видаляємо голосні в кінці
        while !result.is_empty() {
            if let Some(last_char) = result.chars().last() {
                if UKRAINIAN_VOWELS.contains(last_char) || last_char == 'й' {
                    result.pop();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    pub fn get_stats(&self) -> (usize, usize) {
        let data = self.data.lock()
            .expect("Критична помилка блокування даних при отриманні статистики");
        (data.index.total_documents, data.index.total_words)
    }

    fn try_reload_indices_if_needed(&self) {
        let documents_path = "documents_index.json";
        let inverted_path = "inverted_index.json";

        // Перевіряємо чи існують файли індексів і чи вони новіші за поточні
        if std::path::Path::new(documents_path).exists() && std::path::Path::new(inverted_path).exists() {
            let should_reload = {
                if let Ok(data) = self.data.lock() {
                    // Якщо інвертований індекс відсутній, перезавантажуємо
                    data.inverted_index.is_none() || data.index.documents.is_empty()
                } else {
                    return; // Не можемо отримати блокування, пропускаємо
                }
            };

            if should_reload {
                println!("🔄 Автоматичне перезавантаження індексів...");
                if let Err(e) = self.reload(documents_path) {
                    println!("⚠️  Помилка автоматичного перезавантаження індексів: {}", e);
                } else {
                    println!("✅ Індекси автоматично перезавантажено");
                }
            }
        }
    }
}
