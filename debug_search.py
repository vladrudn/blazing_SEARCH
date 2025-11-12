import json
import re
import sys
import io

# Set UTF-8 output for Windows console
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

print("Zavantazhennya indeksiv...")

# Завантаження documents_index
with open('documents_index.json', 'r', encoding='utf-8') as f:
    doc_index = json.load(f)

# Завантаження inverted_index
with open('inverted_index.json', 'r', encoding='utf-8') as f:
    inv_index = json.load(f)

print(f"Dokumentiv: {len(doc_index['documents'])}")
print(f"Sliv v invertovanomu indeksi: {len(inv_index['word_to_docs'])}")

# Шукаємо ВСІХ документів які містять "БАБИЧ"
print("\nShukayemo VSI dokumenty z 'BABICH'...")
babich_docs = []
for i, doc in enumerate(doc_index['documents']):
    text = ' '.join([p['text'] for p in doc['paragraphs']])
    if 'БАБИЧ' in text.upper():
        babich_docs.append((i, doc, text))

print(f"Znayideno {len(babich_docs)} dokumentiv z 'BABICH'")

# Шукаємо той що з 2025 року та містить "Веселе"
target_doc_idx = None
for i, doc, text in babich_docs:
    if '2025' in doc['file_name'] and ('Веселе' in text or '336' in text):
        target_doc_idx = i
        print(f"\n[OK] Znayideno dokument 2025 roku z 'BABICH':")
        print(f"   Indeks: {i}")
        print(f"   Imya: {doc['file_name']}")
        print(f"   Shlyakh: {doc['file_path']}")
        print(f"   Sliv: {doc['word_count']}")

        # Показуємо фрагмент з БАБИЧ
        idx = text.upper().find('БАБИЧ')
        print(f"\n   Fragment tekstu z 'BABICH':")
        print(f"   ...{text[max(0,idx-50):idx+150]}...")
        break

if target_doc_idx is None and babich_docs:
    # Показуємо останній документ з БАБИЧ
    i, doc, text = babich_docs[-1]
    target_doc_idx = i
    print(f"\n[OK] Ostanniy dokument z 'BABICH':")
    print(f"   Indeks: {i}")
    print(f"   Imya: {doc['file_name']}")
    print(f"   Shlyakh: {doc['file_path']}")
    if '13.11.2025' in text and 'Веселе' in text and '336' in text:
        target_doc_idx = i
        print(f"\n[OK] Znayideno dokument:")
        print(f"   Indeks: {i}")
        print(f"   Imya: {doc['file_name']}")
        print(f"   Shlyakh: {doc['file_path']}")
        print(f"   Sliv: {doc['word_count']}")

        # Перевіряємо чи є "БАБИЧ" в тексті
        if 'БАБИЧ' in text.upper():
            print(f"   [OK] Slovo 'BABICH' znayideno v teksti dokumenta")
            # Показуємо фрагмент з БАБИЧ
            idx = text.upper().find('БАБИЧ')
            print(f"\n   Fragment tekstu z 'BABICH':")
            print(f"   ...{text[max(0,idx-50):idx+100]}...")
        else:
            print(f"   [ERROR] Slovo 'BABICH' NE znayideno v teksti dokumenta")
            # Показуємо фрагмент тексту для діагностики
            print(f"\n   Pershykh 500 symvoliv tekstu:")
            print(f"   {text[:500]}")
        break

if target_doc_idx is None:
    print("[ERROR] Dokument ne znayideno!")
    exit(1)

# Тепер перевіряємо інвертований індекс
print(f"\nPerevirka invertovanogo indeksu dlya slova 'babich'...")

# Стеммінг слова як робить Rust код
def stem_word(word):
    word = word.lower()
    # Видаляємо закінчення -ець, -ця, -цю
    if word.endswith('ець'):
        word = word[:-3]
    elif word.endswith('ця'):
        word = word[:-2]
    elif word.endswith('цю'):
        word = word[:-2]
    # Видаляємо закінчення -ого, -ому
    if word.endswith('ого'):
        word = word[:-3]
    if word.endswith('ому'):
        word = word[:-3]
    # Видаляємо голосні в кінці
    ukrainian_vowels = "аеєиіїоуюяь"
    while word and word[-1] in ukrainian_vowels or (word and word[-1] == 'й'):
        word = word[:-1]
    return word

stem = stem_word("БАБИЧ")
print(f"   Stem slova 'BABICH': '{stem}'")

if stem in inv_index['word_to_docs']:
    doc_positions = inv_index['word_to_docs'][stem]
    print(f"   [OK] Slovo '{stem}' znayideno v invertovanomu indeksi")
    print(f"   Dokumentiv z tsym slovom: {len(doc_positions)}")

    # Перевіряємо чи наш документ є в списку
    found_our_doc = False
    for dp in doc_positions:
        if dp['doc_index'] == target_doc_idx:
            found_our_doc = True
            print(f"   [OK] Dokument {target_doc_idx} znayideno v invertovanomu indeksi!")
            print(f"   Pozitsii paragrafiv: {dp['paragraph_positions']}")
            break

    if not found_our_doc:
        print(f"   [ERROR] Dokument {target_doc_idx} NE znayideno v invertovanomu indeksi dlya slova '{stem}'")
        print(f"   Dokumenty de znayideno '{stem}':")
        for dp in doc_positions[:10]:  # Показуємо перші 10
            doc = doc_index['documents'][dp['doc_index']]
            print(f"      - Dokument {dp['doc_index']}: {doc['file_name']}")
else:
    print(f"   [ERROR] Slovo '{stem}' NE znayideno v invertovanomu indeksi!")

# Перевіряємо інші варіанти стему
print(f"\nPerevirka inshykh mozhlyvykh stemiv:")
for variant in ["бабич", "бабіч", "бабича", "бабичем"]:
    stem = stem_word(variant)
    if stem in inv_index['word_to_docs']:
        print(f"   [OK] '{variant}' -> '{stem}' znayideno ({len(inv_index['word_to_docs'][stem])} dokumentiv)")
    else:
        print(f"   [ERROR] '{variant}' -> '{stem}' NE znayideno")

# Перевіряємо максимальний індекс документа в інвертованому індексі
print(f"\nPerevirka maksymalnogo indeksu dokumenta:")
max_doc_idx = 0
for word, doc_positions in inv_index['word_to_docs'].items():
    for dp in doc_positions:
        if dp['doc_index'] > max_doc_idx:
            max_doc_idx = dp['doc_index']

print(f"   Maksymalnyy indeks v invertovanomu indeksi: {max_doc_idx}")
print(f"   Zagalna kilkist dokumentiv: {len(doc_index['documents'])}")
print(f"   Neproindeksovano: {len(doc_index['documents']) - max_doc_idx - 1}")

if max_doc_idx < len(doc_index['documents']) - 1:
    print(f"\n   [PROBLEMA] Dokumenty pislia indeksu {max_doc_idx} NE proindeksovani!")
    print(f"   Pershykh 10 neproindeksovanykh:")
    for i in range(max_doc_idx + 1, min(max_doc_idx + 11, len(doc_index['documents']))):
        print(f"      - [{i}] {doc_index['documents'][i]['file_name']}")
