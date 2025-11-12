import json
import sys
import io

# Set UTF-8 output for Windows console
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

print("Rebuilding inverted index from documents_index.json...")

# Load documents_index
with open('documents_index.json', 'r', encoding='utf-8') as f:
    doc_index = json.load(f)

print(f"Loaded {len(doc_index['documents'])} documents")

# Create new inverted index
inverted_index = {
    'word_to_docs': {},
    'total_documents': len(doc_index['documents'])
}

# Stemming function (same as Rust code)
def stem_word(word):
    word = word.lower()
    # Remove word parts with dash
    if '-' in word:
        parts = [stem_word_part(p) for p in word.split('-')]
        return '-'.join(parts)
    return stem_word_part(word)

def stem_word_part(word):
    # Remove endings -ець, -ця, -цю
    if word.endswith('ець'):
        word = word[:-3]
    elif word.endswith('ця'):
        word = word[:-2]
    elif word.endswith('цю'):
        word = word[:-2]
    # Remove endings -ого, -ому
    if word.endswith('ого'):
        word = word[:-3]
    if word.endswith('ому'):
        word = word[:-3]
    # Remove vowels at the end
    ukrainian_vowels = "аеєиіїоуюяь"
    while word and (word[-1] in ukrainian_vowels or word[-1] == 'й'):
        word = word[:-1]
    return word

# Extract words (same as Rust code)
import re
WORD_REGEX = re.compile(r"[\w']+", re.UNICODE)

def extract_words(text):
    words = []
    for match in WORD_REGEX.finditer(text):
        word = match.group(0).replace("'", "")
        stemmed = stem_word(word)
        if stemmed and len(stemmed) >= 2:
            words.append(stemmed)
    return words

# Build inverted index
print("Building inverted index...")
for doc_idx, doc in enumerate(doc_index['documents']):
    if doc_idx % 100 == 0:
        print(f"  Processing document {doc_idx}/{len(doc_index['documents'])}")

    for para_idx, paragraph in enumerate(doc['paragraphs']):
        text = paragraph['text']
        words = extract_words(text)

        for word in words:
            if word not in inverted_index['word_to_docs']:
                inverted_index['word_to_docs'][word] = []

            # Find or create doc_position for this document
            doc_positions = inverted_index['word_to_docs'][word]
            found = False
            for dp in doc_positions:
                if dp['doc_index'] == doc_idx:
                    if para_idx not in dp['paragraph_positions']:
                        dp['paragraph_positions'].append(para_idx)
                    found = True
                    break

            if not found:
                doc_positions.append({
                    'doc_index': doc_idx,
                    'paragraph_positions': [para_idx]
                })

print(f"Built inverted index with {len(inverted_index['word_to_docs'])} unique words")

# Save inverted index
print("Saving inverted_index.json...")
with open('inverted_index.json', 'w', encoding='utf-8') as f:
    json.dump(inverted_index, f, ensure_ascii=False)

print("Done! Inverted index rebuilt successfully.")
print(f"Total documents: {inverted_index['total_documents']}")
print(f"Unique words: {len(inverted_index['word_to_docs'])}")

# Test search for БАБИЧ
if 'бабич' in inverted_index['word_to_docs']:
    count = len(inverted_index['word_to_docs']['бабич'])
    print(f"\nTest: Found 'бабич' in {count} documents")

    # Check if document 2252 is in the list
    for dp in inverted_index['word_to_docs']['бабич']:
        if dp['doc_index'] == 2252:
            print(f"  [OK] Document 2252 is now indexed for 'бабич'!")
            break
else:
    print("\n[ERROR] 'бабич' not found in inverted index!")
