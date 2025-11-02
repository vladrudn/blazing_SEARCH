use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use crate::document_record::{DocumentRecord, DocumentIndex};
use crate::search_engine::SearchMode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvertedIndex {
    // Мапа: слово -> список документів з позиціями
    pub word_to_docs: HashMap<String, Vec<DocPosition>>,
    pub total_documents: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocPosition {
    pub doc_index: usize,
    pub paragraph_positions: Vec<usize>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            word_to_docs: HashMap::new(),
            total_documents: 0,
        }
    }

    pub fn update_incremental(&mut self, document_index: &DocumentIndex, changed_doc_indices: &[usize]) {
        println!("🚀 Інкрементне оновлення інвертованого індексу...");
        println!("📄 Оновлюємо {} документів", changed_doc_indices.len());

        if changed_doc_indices.is_empty() {
            println!("ℹ️  Немає документів для оновлення інвертованого індексу");
            return;
        }

        // Видаляємо старі записи для змінених документів тільки якщо вони дійсно існують
        let mut actually_removed = 0;
        for &doc_idx in changed_doc_indices {
            let removed_count = self.remove_document_from_index_with_count(doc_idx);
            actually_removed += removed_count;
        }

        // Додаємо нові записи
        let mut actually_added = 0;
        for &doc_idx in changed_doc_indices {
            if let Some(document) = document_index.documents.get(doc_idx) {
                let added_count = self.add_document_to_index_with_count(doc_idx, document);
                actually_added += added_count;
                println!("📝 Додано {} записів для документа {}", added_count, doc_idx);
            } else {
                println!("⚠️  Документ з індексом {} не знайдено в document_index", doc_idx);
            }
        }

        // Оновлюємо загальну кількість документів
        self.total_documents = document_index.documents.len();

        println!("✅ Інкрементне оновлення завершено: видалено {} записів, додано {}", actually_removed, actually_added);
    }

    pub fn build_incremental(existing_index: Option<Self>, document_index: &DocumentIndex, new_or_changed_docs: &[usize]) -> Self {
        let mut inverted_index = existing_index.unwrap_or_else(|| InvertedIndex::new());

        if new_or_changed_docs.is_empty() {
            println!("🚀 Немає нових або змінених документів, індекс залишається незмінним");
            inverted_index.total_documents = document_index.documents.len();
            return inverted_index;
        }

        println!("🔧 build_incremental: обробка {} документів для оновлення індексу", new_or_changed_docs.len());

        // Якщо це перший раз (існуючий індекс порожній), додаємо всі документи
        if inverted_index.word_to_docs.is_empty() {
            println!("📝 Створення нового індексу з нуля...");
            for &doc_idx in new_or_changed_docs {
                if let Some(document) = document_index.documents.get(doc_idx) {
                    let added_count = inverted_index.add_document_to_index_with_count(doc_idx, document);
                    println!("➕ Додано {} записів для документа {} (новий індекс)", added_count, doc_idx);
                }
            }
        } else {
            // Інкрементне оновлення існуючого індексу
            inverted_index.update_incremental(document_index, new_or_changed_docs);
        }

        inverted_index.total_documents = document_index.documents.len();
        inverted_index
    }

    pub fn remove_deleted_documents_by_paths(&mut self, deleted_file_paths: &[String], document_index: &DocumentIndex) {
        if deleted_file_paths.is_empty() {
            return;
        }

        println!("🗑️  Видалення {} документів з інвертованого індексу...", deleted_file_paths.len());

        // Знаходимо поточні індекси для видалених файлів в оновленому документному індексі
        let mut deleted_indices = Vec::new();
        for deleted_path in deleted_file_paths {
            // Шукаємо чи є цей файл ще в документному індексі
            // Якщо так, то знаходимо його індекс для видалення з інвертованого індексу
            for (doc_idx, document) in document_index.documents.iter().enumerate() {
                if document.file_path == *deleted_path {
                    deleted_indices.push(doc_idx);
                    break;
                }
            }
        }

        // Видаляємо записи з інвертованого індексу використовуючи поточні індекси
        for &doc_idx in &deleted_indices {
            self.remove_document_from_index(doc_idx);
        }

        println!("✅ Видалення з інвертованого індексу завершено");
    }

    // Залишаємо старий метод для зворотної сумісності, але позначаємо як deprecated
    #[deprecated(note = "Use remove_deleted_documents_by_paths instead to avoid index mismatch issues")]
    pub fn remove_deleted_documents(&mut self, deleted_indices: &[usize]) {
        if deleted_indices.is_empty() {
            return;
        }

        println!("🗑️  Видалення {} документів з інвертованого індексу...", deleted_indices.len());

        // Видаляємо записи для кожного видаленого документа
        for &doc_idx in deleted_indices {
            self.remove_document_from_index(doc_idx);
        }

        // Після видалення документів потрібно оновити індекси у всіх записах
        // оскільки видалення зміщує індекси документів
        self.reindex_after_deletions(deleted_indices);

        println!("✅ Видалення з інвертованого індексу завершено");
    }

    fn reindex_after_deletions(&mut self, deleted_indices: &[usize]) {
        // Сортуємо індекси видалених документів у зворотному порядку
        let mut sorted_deleted: Vec<usize> = deleted_indices.to_vec();
        sorted_deleted.sort_by(|a, b| b.cmp(a));

        // Оновлюємо індекси для всіх документів
        for doc_positions in self.word_to_docs.values_mut() {
            for doc_pos in doc_positions.iter_mut() {
                let original_idx = doc_pos.doc_index;
                let mut new_idx = original_idx;

                // Рахуємо скільки документів було видалено перед цим індексом
                for &deleted_idx in &sorted_deleted {
                    if deleted_idx < original_idx {
                        new_idx -= 1;
                    }
                }

                doc_pos.doc_index = new_idx;
            }
        }
    }

    fn remove_document_from_index(&mut self, doc_idx: usize) {
        self.remove_document_from_index_with_count(doc_idx);
    }

    fn remove_document_from_index_with_count(&mut self, doc_idx: usize) -> usize {
        // Проходимо по всіх словах і видаляємо посилання на цей документ
        let mut words_to_remove = Vec::new();
        let mut removed_entries = 0;

        for (word, doc_positions) in self.word_to_docs.iter_mut() {
            let original_len = doc_positions.len();
            doc_positions.retain(|dp| dp.doc_index != doc_idx);
            let removed_count = original_len - doc_positions.len();

            if removed_count > 0 {
                removed_entries += removed_count;
            }

            // Якщо слово більше ні в яких документах не зустрічається, позначаємо для видалення
            if doc_positions.is_empty() {
                words_to_remove.push(word.clone());
            }
        }

        // Видаляємо слова, які більше не зустрічаються
        for word in words_to_remove {
            self.word_to_docs.remove(&word);
        }

        if removed_entries > 0 {
            println!("🧹 Видалено {} записів документа {} з інвертованого індексу", removed_entries, doc_idx);
        }

        removed_entries
    }

    fn add_document_to_index(&mut self, doc_idx: usize, document: &DocumentRecord) {
        self.add_document_to_index_with_count(doc_idx, document);
    }

    fn add_document_to_index_with_count(&mut self, doc_idx: usize, document: &DocumentRecord) -> usize {
        let mut added_entries = 0;

        for (para_idx, paragraph) in document.content.iter().enumerate() {
            let words = Self::extract_words(paragraph);

            for word in words {
                let entry = self.word_to_docs
                    .entry(word)
                    .or_insert_with(Vec::new);

                // Перевіряємо чи є вже цей документ
                if let Some(doc_pos) = entry.iter_mut().find(|dp| dp.doc_index == doc_idx) {
                    // Документ вже є, додаємо позицію параграфа
                    if !doc_pos.paragraph_positions.contains(&para_idx) {
                        doc_pos.paragraph_positions.push(para_idx);
                        added_entries += 1;
                    }
                } else {
                    // Новий документ для цього слова
                    entry.push(DocPosition {
                        doc_index: doc_idx,
                        paragraph_positions: vec![para_idx],
                    });
                    added_entries += 1;
                }
            }
        }

        added_entries
    }

    pub fn search_fast(&self, query_words: &[String], document_index: &DocumentIndex, mode: &SearchMode) -> Vec<(usize, Vec<usize>)> {
        if query_words.is_empty() {
            return Vec::new();
        }

        let total_docs = document_index.documents.len();
        let (start_index, end_index) = match mode {
            SearchMode::Quick => {
                let end = if total_docs > 170 { 170 } else { total_docs };
                (0, end)
            },
            SearchMode::Remaining => {
                let start = if total_docs > 170 { 170 } else { 0 };
                (start, total_docs)
            },
            SearchMode::Full => (0, total_docs),
        };

        // ОПТИМІЗАЦІЯ 1: Знаходимо слово з найменшою кількістю документів для першого фільтру
        let mut min_word_count = usize::MAX;
        let mut best_first_word_idx = 0;

        for (idx, word) in query_words.iter().enumerate() {
            if let Some(doc_positions) = self.word_to_docs.get(word) {
                let filtered_count = doc_positions.iter()
                    .filter(|dp| dp.doc_index >= start_index && dp.doc_index < end_index)
                    .count();
                if filtered_count < min_word_count {
                    min_word_count = filtered_count;
                    best_first_word_idx = idx;
                }
            } else {
                return Vec::new(); // Якщо якесь слово відсутнє, результат порожній
            }
        }

        // Починаємо з найрідшого слова
        let first_word = &query_words[best_first_word_idx];
        let mut candidate_docs: HashMap<usize, HashSet<usize>> = HashMap::new();

        if let Some(doc_positions) = self.word_to_docs.get(first_word) {
            for doc_pos in doc_positions.iter().filter(|dp| dp.doc_index >= start_index && dp.doc_index < end_index) {
                candidate_docs.insert(doc_pos.doc_index, doc_pos.paragraph_positions.iter().cloned().collect());
            }
        }

        if candidate_docs.is_empty() {
            return Vec::new();
        }

        // ОПТИМІЗАЦІЯ 2: Обробляємо інші слова в порядку зростання кількості документів
        let mut other_words: Vec<_> = query_words.iter().enumerate()
            .filter(|(idx, _)| *idx != best_first_word_idx)
            .map(|(_, word)| word)
            .collect();

        other_words.sort_by_key(|word| {
            self.word_to_docs.get(*word).map_or(0, |docs|
                docs.iter().filter(|dp| dp.doc_index >= start_index && dp.doc_index < end_index).count()
            )
        });

        // ОПТИМІЗАЦІЯ 3: Використовуємо HashSet для швидшого пересічення
        for word in other_words {
            if let Some(doc_positions) = self.word_to_docs.get(word) {
                let docs_with_current_word: HashMap<usize, HashSet<usize>> = doc_positions.iter()
                    .filter(|dp| dp.doc_index >= start_index && dp.doc_index < end_index)
                    .map(|dp| (dp.doc_index, dp.paragraph_positions.iter().cloned().collect()))
                    .collect();

                // ОПТИМІЗАЦІЯ 4: Ранній вихід якщо перетину немає
                candidate_docs.retain(|doc_idx, positions| {
                    if let Some(current_positions) = docs_with_current_word.get(doc_idx) {
                        // Об'єднуємо позиції параграфів (Union)
                        positions.extend(current_positions);
                        true
                    } else {
                        false
                    }
                });

                if candidate_docs.is_empty() {
                    return Vec::new(); // Ранній вихід якщо немає кандидатів
                }
            } else {
                return Vec::new();
            }
        }

        // Конвертуємо назад у Vec і сортуємо
        let final_results: Vec<(usize, Vec<usize>)> = candidate_docs.into_iter()
            .map(|(doc_idx, positions)| {
                let mut pos_vec: Vec<usize> = positions.into_iter().collect();
                pos_vec.sort_unstable();
                (doc_idx, pos_vec)
            })
            .collect();

        final_results
    }

    fn extract_words(text: &str) -> Vec<String> {
        use regex::Regex;
        use once_cell::sync::Lazy;

        static WORD_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[\p{L}\p{N}']+").unwrap()
        });

        WORD_REGEX
            .find_iter(text)
            .map(|m| {
                let word_without_apostrophe = m.as_str().replace('\'', "");
                Self::stem_word(&word_without_apostrophe)
            })
            .filter(|word| !word.is_empty() && word.len() >= 2) // Фільтруємо порожні та занадто короткі слова
            .collect()
    }

    fn stem_word(word: &str) -> String {
        let word = word.to_lowercase();

        // Обробка слів з дефісом
        if word.contains('-') {
            let parts: Vec<String> = word.split('-')
                .map(|part| Self::stem_word_part(part))
                .collect();
            return parts.join("-");
        }

        Self::stem_word_part(&word)
    }

    fn stem_word_part(word: &str) -> String {
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
        static UKRAINIAN_VOWELS: &str = "аеєиіїоуюяь";
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

    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        use std::path::Path;
        use std::fs;

        // Атомарне збереження через тимчасовий файл
        let temp_path = format!("{}.tmp", path);
        let backup_path = format!("{}.backup", path);

        // Створюємо резервну копію існуючого файлу якщо він є
        if Path::new(path).exists() {
            fs::copy(path, &backup_path)
                .map_err(|e| format!("Помилка створення резервної копії інвертованого індексу: {}", e))?;
        }

        // Зберігаємо в тимчасовий файл
        let json = serde_json::to_string(self)
            .map_err(|e| format!("Помилка серіалізації інвертованого індексу: {}", e))?;

        fs::write(&temp_path, json)
            .map_err(|e| {
                // Видаляємо пошкоджений тимчасовий файл
                let _ = fs::remove_file(&temp_path);
                format!("Помилка запису тимчасового файлу інвертованого індексу: {}", e)
            })?;

        // Атомарно переміщуємо тимчасовий файл на місце основного
        fs::rename(&temp_path, path)
            .map_err(|e| {
                // Якщо переміщення не вдалося, спробуємо відновити з резервної копії
                if Path::new(&backup_path).exists() {
                    let _ = fs::rename(&backup_path, path);
                }
                format!("Помилка переміщення тимчасового файлу інвертованого індексу: {}", e)
            })?;

        // Видаляємо резервну копію після успішного збереження
        if Path::new(&backup_path).exists() {
            let _ = fs::remove_file(&backup_path);
        }

        Ok(())
    }

    pub fn get_stats(&self) -> (usize, usize) {
        (self.total_documents, self.word_to_docs.len())
    }

    pub fn load_from_file(path: &str) -> Result<Self, String> {
        use std::path::Path;
        use std::fs;

        let backup_path = format!("{}.backup", path);

        // Спочатку пробуємо завантажити основний файл
        let index = Self::try_load_file(path);

        match index {
            Ok(idx) => {
                // Перевіряємо цілісність індексу
                if Self::validate_index(&idx) {
                    return Ok(idx);
                } else {
                    println!("⚠️  Основний інвертований індекс пошкоджений, спробуємо резервну копію...");
                }
            }
            Err(e) => {
                println!("⚠️  Помилка завантаження основного інвертованого індексу: {}", e);
                println!("🔄 Спробуємо резервну копію...");
            }
        }

        // Якщо основний файл пошкоджений, пробуємо резервну копію
        if Path::new(&backup_path).exists() {
            match Self::try_load_file(&backup_path) {
                Ok(backup_idx) => {
                    if Self::validate_index(&backup_idx) {
                        println!("✅ Завантажено інвертований індекс з резервної копії");
                        // Відновлюємо основний файл з резервної копії
                        if let Err(e) = fs::copy(&backup_path, path) {
                            println!("⚠️  Не вдалося відновити основний файл інвертованого індексу: {}", e);
                        }
                        return Ok(backup_idx);
                    } else {
                        println!("❌ Резервна копія інвертованого індексу також пошкоджена");
                    }
                }
                Err(e) => {
                    println!("❌ Помилка завантаження резервної копії інвертованого індексу: {}", e);
                }
            }
        }

        Err("Не вдалося завантажити інвертований індекс: всі файли пошкоджені або відсутні".to_string())
    }

    fn try_load_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Помилка читання файлу: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Помилка десеріалізації: {}", e))
    }

    fn validate_index(index: &Self) -> bool {
        // Базові перевірки цілісності (м'якіші)
        if index.word_to_docs.is_empty() && index.total_documents > 100 {
            println!("❌ Невідповідність інвертованого індексу: total_documents > 100, але word_to_docs порожній");
            return false;
        }

        let mut invalid_words = Vec::new();
        let mut empty_doc_lists = Vec::new();
        let mut empty_positions = Vec::new();

        // Збираємо проблемні записи
        for (word, doc_positions) in &index.word_to_docs {
            if word.is_empty() || word.len() < 2 {
                invalid_words.push(word.clone());
                continue;
            }

            if doc_positions.is_empty() {
                empty_doc_lists.push(word.clone());
                continue;
            }

            for doc_pos in doc_positions {
                if doc_pos.paragraph_positions.is_empty() {
                    empty_positions.push((word.clone(), doc_pos.doc_index));
                }
            }
        }

        // Репортуємо проблеми, але не блокуємо збереження
        if !invalid_words.is_empty() {
            println!("⚠️  Знайдено {} невалідних слів в інвертованому індексі (будуть виправлені)", invalid_words.len());
        }

        if !empty_doc_lists.is_empty() {
            println!("⚠️  Знайдено {} слів з порожніми списками документів", empty_doc_lists.len());
        }

        if !empty_positions.is_empty() {
            println!("⚠️  Знайдено {} записів з порожніми позиціями", empty_positions.len());
        }

        // Дозволяємо збереження, навіть якщо є проблеми
        true
    }

    // Функція для очищення індексу від невалідних записів
    pub fn cleanup(&mut self) -> usize {
        let mut removed_count = 0;

        // Видаляємо невалідні слова та порожні записи
        self.word_to_docs.retain(|word, doc_positions| {
            // Видаляємо порожні або занадто короткі слова
            if word.is_empty() || word.len() < 2 {
                removed_count += 1;
                return false;
            }

            // Очищуємо порожні позиції в документах
            doc_positions.retain(|doc_pos| !doc_pos.paragraph_positions.is_empty());

            // Видаляємо слова з порожніми списками документів
            if doc_positions.is_empty() {
                removed_count += 1;
                return false;
            }

            true
        });

        if removed_count > 0 {
            println!("🧹 Очищено {} невалідних записів з інвертованого індексу", removed_count);
        }

        removed_count
    }

    // Функція для виявлення та очистки дублікатів записів
    pub fn remove_duplicate_entries(&mut self) -> usize {
        let mut duplicates_removed = 0;

        for (_word, doc_positions) in self.word_to_docs.iter_mut() {
            let original_len = doc_positions.len();

            // Сортуємо для групування дублікатів
            doc_positions.sort_by_key(|dp| dp.doc_index);

            // Видаляємо дублікати з одним індексом документа
            let mut unique_positions = Vec::new();
            let mut current_doc_idx = None;
            let mut current_paragraphs = Vec::new();

            for doc_pos in doc_positions.drain(..) {
                if current_doc_idx == Some(doc_pos.doc_index) {
                    // Об'єднуємо параграфи для одного документа
                    for para in doc_pos.paragraph_positions {
                        if !current_paragraphs.contains(&para) {
                            current_paragraphs.push(para);
                        }
                    }
                } else {
                    // Зберігаємо попередній документ якщо він був
                    if let Some(doc_idx) = current_doc_idx {
                        if !current_paragraphs.is_empty() {
                            current_paragraphs.sort_unstable();
                            unique_positions.push(DocPosition {
                                doc_index: doc_idx,
                                paragraph_positions: current_paragraphs.clone(),
                            });
                        }
                    }

                    // Початок нового документа
                    current_doc_idx = Some(doc_pos.doc_index);
                    current_paragraphs = doc_pos.paragraph_positions;
                }
            }

            // Додаємо останній документ
            if let Some(doc_idx) = current_doc_idx {
                if !current_paragraphs.is_empty() {
                    current_paragraphs.sort_unstable();
                    unique_positions.push(DocPosition {
                        doc_index: doc_idx,
                        paragraph_positions: current_paragraphs,
                    });
                }
            }

            let removed = original_len - unique_positions.len();
            duplicates_removed += removed;

            *doc_positions = unique_positions;
        }

        if duplicates_removed > 0 {
            println!("🧹 Видалено {} дублікатів записів з інвертованого індексу", duplicates_removed);
        }

        duplicates_removed
    }

    // Функція для повного перебудування індексу
    pub fn rebuild_from_scratch(document_index: &DocumentIndex) -> Self {
        println!("🔄 Повне перебудування інвертованого індексу...");

        let mut inverted_index = InvertedIndex::new();
        inverted_index.total_documents = document_index.documents.len();

        for (doc_idx, document) in document_index.documents.iter().enumerate() {
            inverted_index.add_document_to_index(doc_idx, document);
        }

        // Очищуємо невалідні записи та дублікати
        inverted_index.cleanup();
        inverted_index.remove_duplicate_entries();

        let (docs, words) = inverted_index.get_stats();
        println!("✅ Перебудування завершено: {} документів, {} слів", docs, words);

        inverted_index
    }
}
