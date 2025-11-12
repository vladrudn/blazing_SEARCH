use std::env;

mod document_record;
mod inverted_index;

use document_record::DocumentIndex;
use inverted_index::InvertedIndex;

fn main() {
    println!("🔄 Перебудова інвертованого індексу...");

    // Завантажуємо індекс документів
    let doc_index = match DocumentIndex::load_from_file("documents_index.json") {
        Ok(index) => {
            println!("✅ Завантажено {} документів", index.documents.len());
            index
        }
        Err(e) => {
            eprintln!("❌ Помилка завантаження documents_index.json: {}", e);
            return;
        }
    };

    // Перебудовуємо інвертований індекс
    let inv_index = InvertedIndex::rebuild_from_scratch(&doc_index);

    // Зберігаємо
    match inv_index.save_to_file("inverted_index.json") {
        Ok(_) => {
            println!("✅ Інвертований індекс успішно перебудовано і збережено!");
            let (docs, words) = inv_index.get_stats();
            println!("📊 Документів: {}, Слів: {}", docs, words);
        }
        Err(e) => {
            eprintln!("❌ Помилка збереження: {}", e);
        }
    }
}
