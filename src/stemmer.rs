/// Модуль для стемінгу (нормалізації) українських слів
/// Використовується як в пошуку, так і при створенні індексу

static UKRAINIAN_VOWELS: &str = "аеєиіїоуюяь";

/// Виконує стемінг слова (приведення до основи)
pub fn stem_word(word: &str) -> String {
    let word = word.to_lowercase();

    // Обробка слів з дефісом
    if word.contains('-') {
        let parts: Vec<String> = word
            .split('-')
            .map(|part| stem_word_part(part))
            .collect();
        return parts.join("-");
    }

    stem_word_part(&word)
}

/// Стемінг окремої частини слова (без дефісів)
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

    // Спеціальне правило ТІЛЬКИ для імені "Федір" та його відмінків
    // "федір" → "федір", "федора" → "федор", "федору" → "федор" → всі до "фед"
    if result.starts_with("фед") && (result.ends_with("ір") || result.ends_with("ор") || result.ends_with("і")) {
        result = "фед".to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stem_basic() {
        assert_eq!(stem_word("донецького"), "донецьк");
        assert_eq!(stem_word("лейтенанта"), "лейтенант");
        assert_eq!(stem_word("солдата"), "солдат");
    }

    #[test]
    fn test_stem_with_hyphen() {
        assert_eq!(stem_word("донецько-луганський"), "донецьк-луганськ");
    }

    #[test]
    fn test_stem_endings() {
        assert_eq!(stem_word("донецькому"), "донецьк");
        assert_eq!(stem_word("донець"), "дон");
    }

    #[test]
    fn test_stem_fedir() {
        // Спеціальне правило ТІЛЬКИ для імені "Федір"
        assert_eq!(stem_word("федір"), "фед");    // федір → фед
        assert_eq!(stem_word("федора"), "фед");   // федора → федор → фед
        assert_eq!(stem_word("федору"), "фед");   // федору → федор → фед

        // Інші імена НЕ обрізаються так само
        assert_eq!(stem_word("ігор"), "ігор");    // ігор → ігор (залишається)
        assert_eq!(stem_word("ігоря"), "ігор");   // ігоря → ігор
    }
}
