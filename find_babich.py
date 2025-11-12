import json
import sys
import io

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

print("Loading indices...")
with open('documents_index.json', 'r', encoding='utf-8') as f:
    doc_index = json.load(f)

with open('inverted_index.json', 'r', encoding='utf-8') as f:
    inv_index = json.load(f)

print(f"Documents: {len(doc_index['documents'])}")

# Find ALL documents with БАБИЧ
print("\nSearching for documents with 'BABICH'...")
babich_docs = []
for i, doc in enumerate(doc_index['documents']):
    text = ' '.join([p['text'] for p in doc['paragraphs']])
    if 'БАБИЧ' in text.upper():
        babich_docs.append((i, doc['file_name'], text))

print(f"Found {len(babich_docs)} documents with 'BABICH'")

# Show documents from 2025
print("\nDocuments from 2025:")
for i, name, text in babich_docs:
    if '2025' in name:
        print(f"  [{i}] {name}")
        # Check if indexed
        if 'бабич' in inv_index['word_to_docs']:
            found = any(dp['doc_index'] == i for dp in inv_index['word_to_docs']['бабич'])
            status = "[INDEXED]" if found else "[NOT INDEXED]"
            print(f"       {status}")

# Show last 10 documents with БАБИЧ
print("\nLast 10 documents with 'BABICH':")
for i, name, text in babich_docs[-10:]:
    print(f"  [{i}] {name}")
    if 'бабич' in inv_index['word_to_docs']:
        found = any(dp['doc_index'] == i for dp in inv_index['word_to_docs']['бабич'])
        status = "[INDEXED]" if found else "[NOT INDEXED]"
        print(f"       {status}")
