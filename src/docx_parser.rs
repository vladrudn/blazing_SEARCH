use quick_xml::events::{Event, BytesStart};
use quick_xml::Reader;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use zip::ZipArchive;
use once_cell::sync::Lazy;

// Глобальні компільовані регулярні вирази для кращої продуктивності
static NUMBERING_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*\d+(\.\d+)*\.\s+").unwrap());
static QUOTE_NUMBERING_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*«\s*\d+(\.\d+)*\.\s+").unwrap());
static BASIS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*Підстава:").unwrap());

#[derive(Debug, Clone)]
pub struct ParagraphInfo {
    pub text: String,
    #[allow(dead_code)]
    pub style: Option<String>,
    pub level: Option<usize>,
    pub has_numbering: bool,
    pub calculated_number: Option<String>,
    #[allow(dead_code)]
    pub original_text: String,
}

impl ParagraphInfo {
    fn new(text: String, style: Option<String>) -> Self {
        Self {
            original_text: text.clone(),
            text,
            style,
            level: None,
            has_numbering: false,
            calculated_number: None,
        }
    }

    fn with_numbering(
        text: String,
        style: Option<String>,
        level: usize,
        calculated_number: String,
    ) -> Self {
        Self {
            original_text: text.clone(),
            text,
            style,
            level: Some(level),
            has_numbering: true,
            calculated_number: Some(calculated_number),
        }
    }
}

#[derive(Debug)]
pub struct NumberingData {
    abstract_num_map: HashMap<String, HashMap<String, String>>,
    num_id_map: HashMap<String, HashMap<String, String>>,
}

impl Default for NumberingData {
    fn default() -> Self {
        Self {
            abstract_num_map: HashMap::new(),
            num_id_map: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct CurrentNumbering {
    level_1: usize,
    level_2: usize,
    level_3: usize,
    level_4: usize,
}

impl Default for CurrentNumbering {
    fn default() -> Self {
        Self {
            level_1: 0,
            level_2: 0,
            level_3: 0,
            level_4: 0,
        }
    }
}

pub struct DocxParser {
    doc_path: String,
    numbering_data: NumberingData,
}

impl DocxParser {
    // XML namespace константи
    #[allow(dead_code)]
    const W_NAMESPACE: &'static str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    // Стилі, що відповідають рівням нумерації
    const STYLE_LEVEL_MAP: &'static [(&'static str, usize)] = &[
        ("OiiSList1", 1), ("Oii_S_List_1", 1),
        ("OiiSList2", 2), ("Oii_S_List_2", 2),
        ("OiiSList3", 3), ("Oii_S_List_3", 3),
        ("OiiSList4", 4), ("Oii_S_List_4", 4),
    ];

    // Тексти для пропуску
    const SKIP_TEXTS: &'static [&'static str] = &["ПОГОДЖЕНО", "Документ підготовлено"];

    pub fn new(doc_path: String) -> Self {
        Self {
            doc_path,
            numbering_data: NumberingData::default(),
        }
    }

    pub fn parse(&mut self) -> Result<Vec<String>, String> {
        let paragraphs_info = self.extract_hierarchical_numbering()?;
        Ok(self.format_paragraphs(paragraphs_info))
    }

    fn open_docx(&mut self) -> Result<(String, Option<String>), String> {
        let file = File::open(&self.doc_path)
            .map_err(|e| format!("Помилка при відкритті документа: {}", e))?;

        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)
            .map_err(|e| format!("Помилка при відкритті ZIP архіву: {}", e))?;

        // Читання document.xml
        let doc_contents = {
            let mut doc_file = archive.by_name("word/document.xml")
                .map_err(|e| format!("Помилка при читанні document.xml: {}", e))?;

            let mut contents = String::new();
            doc_file.read_to_string(&mut contents)
                .map_err(|e| format!("Помилка при читанні вмісту документа: {}", e))?;
            contents
        };

        // Спроба читання numbering.xml
        let numbering_contents = match archive.by_name("word/numbering.xml") {
            Ok(mut numbering_file) => {
                let mut contents = String::new();
                match numbering_file.read_to_string(&mut contents) {
                    Ok(_) => Some(contents),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };

        Ok((doc_contents, numbering_contents))
    }

    fn process_numbering_xml(&mut self, numbering_xml: &str) -> Result<(), String> {
        let mut reader = Reader::from_str(numbering_xml);

        let mut buf = Vec::new();
        let mut current_abstract_num_id = None;
        let mut current_ilvl = None;
        let mut current_num_id = None;
        let mut current_abstract_num_id_ref = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"w:abstractNum" => {
                            if let Some(id) = self.get_attribute_value(e, "w:abstractNumId") {
                                current_abstract_num_id = Some(id);
                                self.numbering_data.abstract_num_map.insert(current_abstract_num_id.clone().unwrap(), HashMap::new());
                            }
                        }
                        b"w:lvl" => {
                            if let Some(ilvl) = self.get_attribute_value(e, "w:ilvl") {
                                current_ilvl = Some(ilvl);
                            }
                        }
                        b"w:num" => {
                            if let Some(num_id) = self.get_attribute_value(e, "w:numId") {
                                current_num_id = Some(num_id);
                            }
                        }
                        b"w:abstractNumId" => {
                            if let Some(val) = self.get_attribute_value(e, "w:val") {
                                current_abstract_num_id_ref = Some(val);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    match e.name().as_ref() {
                        b"w:lvlText" => {
                            if let (Some(abstract_num_id), Some(ilvl)) = (&current_abstract_num_id, &current_ilvl) {
                                if let Some(val) = self.get_attribute_value(e, "w:val") {
                                    if let Some(level_map) = self.numbering_data.abstract_num_map.get_mut(abstract_num_id) {
                                        level_map.insert(ilvl.clone(), val);
                                    }
                                }
                            }
                        }
                        b"w:abstractNumId" => {
                            if let Some(val) = self.get_attribute_value(e, "w:val") {
                                current_abstract_num_id_ref = Some(val);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    match e.name().as_ref() {
                        b"w:abstractNum" => {
                            current_abstract_num_id = None;
                        }
                        b"w:lvl" => {
                            current_ilvl = None;
                        }
                        b"w:num" => {
                            if let (Some(num_id), Some(abstract_num_id_ref)) = (&current_num_id, &current_abstract_num_id_ref) {
                                if let Some(abstract_data) = self.numbering_data.abstract_num_map.get(abstract_num_id_ref).cloned() {
                                    self.numbering_data.num_id_map.insert(num_id.clone(), abstract_data);
                                }
                            }
                            current_num_id = None;
                            current_abstract_num_id_ref = None;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Помилка парсингу numbering.xml: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    fn get_attribute_value(&self, element: &BytesStart, attr_name: &str) -> Option<String> {
        element.attributes()
            .find_map(|attr| {
                if let Ok(attr) = attr {
                    let key = std::str::from_utf8(attr.key.as_ref()).ok()?;
                    if key == attr_name {
                        return std::str::from_utf8(&attr.value).ok().map(|s| s.to_string());
                    }
                }
                None
            })
    }

    fn extract_hierarchical_numbering(&mut self) -> Result<Vec<ParagraphInfo>, String> {
        let (doc_xml, numbering_xml) = self.open_docx()?;

        // Обробка numbering.xml якщо існує
        if let Some(numbering_content) = numbering_xml {
            self.process_numbering_xml(&numbering_content)?;
        }

        let mut reader = Reader::from_str(&doc_xml);

        let mut buf = Vec::new();
        let mut result = Vec::new();
        let mut current_numbering = CurrentNumbering::default();
        let mut last_main_point = 0;

        // Змінні для обробки поточного параграфа
        let mut in_paragraph = false;
        let mut paragraph_text = String::new();
        let mut paragraph_style = None;
        let mut paragraph_num_pr = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"w:p" => {
                            in_paragraph = true;
                            paragraph_text.clear();
                            paragraph_style = None;
                            paragraph_num_pr = None;
                        }
                        b"w:pStyle" => {
                            if in_paragraph {
                                if let Some(val) = self.get_attribute_value(e, "w:val") {
                                    paragraph_style = Some(val);
                                }
                            }
                        }
                        b"w:numPr" => {
                            if in_paragraph {
                                paragraph_num_pr = Some(self.read_num_pr(&mut reader, &mut buf)?);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    match e.name().as_ref() {
                        b"w:pStyle" => {
                            if in_paragraph {
                                if let Some(val) = self.get_attribute_value(e, "w:val") {
                                    paragraph_style = Some(val);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_paragraph {
                        if let Ok(text) = e.unescape() {
                            paragraph_text.push_str(&text);
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == b"w:p" && in_paragraph {
                        in_paragraph = false;

                        let raw_text = paragraph_text.trim().to_string();
                        if raw_text.is_empty() || self.should_skip_text(&raw_text) {
                            continue;
                        }

                        let paragraph_info = self.process_paragraph(
                            raw_text,
                            paragraph_style.clone(),
                            paragraph_num_pr.clone(),
                            &mut current_numbering,
                            &mut last_main_point,
                        );

                        if let Some(info) = paragraph_info {
                            if info.level == Some(1) {
                                last_main_point = current_numbering.level_1;
                            }
                            result.push(info);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Помилка парсингу XML: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(result)
    }

    fn read_num_pr(&self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(Option<String>, Option<String>), String> {
        let mut ilvl = None;
        let mut num_id = None;

        loop {
            match reader.read_event_into(buf) {
                Ok(Event::Empty(ref e)) => {
                    match e.name().as_ref() {
                        b"w:ilvl" => {
                            ilvl = self.get_attribute_value(e, "w:val");
                        }
                        b"w:numId" => {
                            num_id = self.get_attribute_value(e, "w:val");
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == b"w:numPr" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Помилка читання numPr: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok((ilvl, num_id))
    }

    fn should_skip_text(&self, text: &str) -> bool {
        // Пропускаємо звичайні службові тексти
        if Self::SKIP_TEXTS.iter().any(|&prefix| text.starts_with(prefix)) {
            return true;
        }


        false
    }


    fn process_paragraph(
        &self,
        text: String,
        style: Option<String>,
        num_pr: Option<(Option<String>, Option<String>)>,
        current_numbering: &mut CurrentNumbering,
        last_main_point: &mut usize,
    ) -> Option<ParagraphInfo> {
        let has_text_numbering = NUMBERING_REGEX.is_match(&text);
        let has_quote_with_numbering = QUOTE_NUMBERING_REGEX.is_match(&text);
        let has_basis = BASIS_REGEX.is_match(&text);

        // Обробка за правилами з Python коду
        if has_basis {
            return Some(ParagraphInfo::new(text, style));
        }

        if has_text_numbering && !has_quote_with_numbering {
            return Some(ParagraphInfo::new(text, style));
        }

        if has_quote_with_numbering && num_pr.is_some() {
            let (ilvl, num_id) = num_pr.unwrap();
            if let Some(level) = self.get_numbering_level(&ilvl, &num_id) {
                self.update_numbering_for_level(level, current_numbering, *last_main_point);
                let calculated_number = self.format_numbering(level, current_numbering);
                return Some(ParagraphInfo::with_numbering(text, style, level, calculated_number));
            }
            return Some(ParagraphInfo::new(text, style));
        }

        if has_quote_with_numbering {
            return Some(ParagraphInfo::new(text, style));
        }

        if let Some((ilvl, num_id)) = num_pr {
            if let Some(level) = self.get_numbering_level(&ilvl, &num_id) {
                self.update_numbering_for_level(level, current_numbering, *last_main_point);
                let calculated_number = self.format_numbering(level, current_numbering);
                return Some(ParagraphInfo::with_numbering(text, style, level, calculated_number));
            }
        }

        if let Some(ref style_name) = style {
            if let Some(level) = self.get_style_level(style_name) {
                self.update_numbering_for_level(level, current_numbering, *last_main_point);
                let calculated_number = self.format_numbering(level, current_numbering);
                return Some(ParagraphInfo::with_numbering(text, style, level, calculated_number));
            }
        }

        Some(ParagraphInfo::new(text, style))
    }

    fn get_numbering_level(&self, ilvl: &Option<String>, num_id: &Option<String>) -> Option<usize> {
        if let (Some(ilvl), Some(_num_id)) = (ilvl, num_id) {
            if let Ok(level) = ilvl.parse::<usize>() {
                return Some(level + 1); // XML levels start from 0, our levels from 1
            }
        }
        None
    }

    fn get_style_level(&self, style_name: &str) -> Option<usize> {
        Self::STYLE_LEVEL_MAP.iter()
            .find(|(name, _)| *name == style_name)
            .map(|(_, level)| *level)
    }

    fn update_numbering_for_level(
        &self,
        level: usize,
        current_numbering: &mut CurrentNumbering,
        last_main_point: usize,
    ) {
        match level {
            1 => {
                current_numbering.level_1 = last_main_point + 1;
                current_numbering.level_2 = 0;
                current_numbering.level_3 = 0;
                current_numbering.level_4 = 0;
            }
            2 => {
                current_numbering.level_2 += 1;
                current_numbering.level_3 = 0;
                current_numbering.level_4 = 0;
            }
            3 => {
                current_numbering.level_3 += 1;
                current_numbering.level_4 = 0;
            }
            4 => {
                current_numbering.level_4 += 1;
            }
            _ => {}
        }
    }

    fn format_numbering(&self, level: usize, current_numbering: &CurrentNumbering) -> String {
        match level {
            1 => format!("{}. ", current_numbering.level_1),
            2 => format!("{}.{}. ", current_numbering.level_1, current_numbering.level_2),
            3 => format!("{}.{}.{}. ",
                         current_numbering.level_1,
                         current_numbering.level_2,
                         current_numbering.level_3
            ),
            4 => format!("{}.{}.{}.{}. ",
                         current_numbering.level_1,
                         current_numbering.level_2,
                         current_numbering.level_3,
                         current_numbering.level_4
            ),
            _ => String::new(),
        }
    }

    fn format_paragraphs(&self, paragraphs_info: Vec<ParagraphInfo>) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_section = String::new();

        for p_info in paragraphs_info {
            let formatted_text = if p_info.has_numbering {
                if let Some(calculated_number) = p_info.calculated_number {
                    format!("{}{}", calculated_number, p_info.text)
                } else {
                    p_info.text
                }
            } else {
                p_info.text
            };

            // Якщо це новий нумерований розділ (має numbering)
            if p_info.has_numbering {
                // Зберігаємо попередній розділ якщо він не порожній
                if !current_section.is_empty() {
                    result.push(current_section.trim().to_string());
                    current_section.clear();
                }

                // Починаємо новий розділ
                current_section = formatted_text;
            } else {
                // Це звичайний текст - додаємо до поточного розділу з переносом рядка
                if !current_section.is_empty() {
                    current_section.push('\n');
                }
                current_section.push_str(&formatted_text);
            }
        }

        // Додаємо останній розділ
        if !current_section.is_empty() {
            result.push(current_section.trim().to_string());
        }

        // Розділяємо параграфи що містять '\n' на окремі параграфи
        let mut final_result = Vec::new();
        for paragraph in result {
            if paragraph.contains('\n') {
                // Розділяємо по переносу рядка і додаємо кожну частину як окремий параграф
                for part in paragraph.split('\n') {
                    let trimmed_part = part.trim();
                    if !trimmed_part.is_empty() {
                        final_result.push(trimmed_part.to_string());
                    }
                }
            } else {
                final_result.push(paragraph);
            }
        }

        final_result
    }
}

// Публічна функція для парсингу
pub fn parse_docx(doc_path: &str) -> Result<Vec<String>, String> {
    let mut parser = DocxParser::new(doc_path.to_string());
    parser.parse()
}