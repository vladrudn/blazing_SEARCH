// Елементи інтерфейсу
const searchInput = document.getElementById('search-input');
const resultsContainer = document.getElementById('results-container');
const filesList = document.getElementById('files-list');
const documentPreview = document.getElementById('document-preview');
const searchStats = document.getElementById('search-stats');
const processingTime = document.getElementById('processing-time');
const infoPanel = document.getElementById('info-panel');
const loader = document.getElementById('loader');
const errorMessage = document.getElementById('error-message');

// Helper функція для отримання тексту параграфа (підтримка старого і нового формату)
function getParagraphText(paragraphData) {
    return typeof paragraphData === 'string' ? paragraphData : paragraphData.text;
}

// Клас для копіювання шляхів файлів у буфер обміну
class FilePathCopier {
    constructor() {
        this.initializeToastContainer();
    }

    initializeToastContainer() {
        if (!document.getElementById('toast-container')) {
            const container = document.createElement('div');
            container.id = 'toast-container';
            container.style.cssText = `
                position: fixed;
                top: 20px;
                right: 20px;
                z-index: 9999;
                max-width: 350px;
                pointer-events: none;
            `;
            document.body.appendChild(container);
        }
    }

    // Конвертує локальний шлях кешу в мережевий шлях
    convertToNetworkPath(localPath) {
        // Нормалізуємо шлях - замінюємо всі слеші на прямі
        const normalizedPath = localPath.replace(/\\/g, '/');

        // Перевіряємо чи це шлях до кешу
        if (normalizedPath.startsWith('./nakazi_cache/')) {
            const relativePath = normalizedPath.substring('./nakazi_cache/'.length);
            return '\\\\salem\\Documents\\Накази\\' + relativePath.replace(/\//g, '\\');
        }

        // Якщо це вже мережевий шлях або інший формат, повертаємо як є
        return localPath;
    }

    async copyFilePath(filePath) {
        try {
            const networkPath = this.convertToNetworkPath(filePath);
            if (navigator.clipboard && window.isSecureContext) {
                await navigator.clipboard.writeText(networkPath);
            } else {
                this.fallbackCopy(networkPath);
            }
            this.showToast('Шлях файлу скопійовано!', 'success');
            return true;
        } catch (error) {
            this.showToast('Помилка копіювання', 'error');
            console.error('Clipboard error:', error);
            return false;
        }
    }

    async copyFilePathWithInstruction(filePath) {
        try {
            const networkPath = this.convertToNetworkPath(filePath);
            if (navigator.clipboard && window.isSecureContext) {
                await navigator.clipboard.writeText(networkPath);
            } else {
                this.fallbackCopy(networkPath);
            }
            this.showExtendedToast();
            return true;
        } catch (error) {
            this.showToast('Помилка копіювання', 'error');
            console.error('Clipboard error:', error);
            return false;
        }
    }
    
    showExtendedToast() {
        const toast = document.createElement('div');
        toast.innerHTML = `
            <div style="font-weight: bold; margin-bottom: 8px;">✅ Шлях файлу скопійовано!</div>
            <div style="font-size: 13px; opacity: 0.9;">
                💡 Натисніть <strong>Win + R</strong>, потім <strong>Ctrl + V</strong><br>
                для відкриття файлу на вашому комп'ютері
            </div>
        `;
        
        toast.style.cssText = `
            background: #10b981;
            color: white;
            padding: 16px 20px;
            border-radius: 8px;
            margin-bottom: 8px;
            transform: translateX(100%);
            transition: transform 0.3s ease;
            font-size: 14px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            pointer-events: auto;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            line-height: 1.4;
        `;
        
        const container = document.getElementById('toast-container');
        container.appendChild(toast);
        
        // Анімація появи
        requestAnimationFrame(() => {
            toast.style.transform = 'translateX(0)';
        });
        
        // Видалення через 5 секунд (довше для читання інструкції)
        setTimeout(() => {
            toast.style.transform = 'translateX(100%)';
            setTimeout(() => {
                if (toast.parentNode) {
                    container.removeChild(toast);
                }
            }, 300);
        }, 5000);
    }
    
    fallbackCopy(text) {
        const textArea = document.createElement('textarea');
        textArea.value = text;
        textArea.style.position = 'fixed';
        textArea.style.left = '-9999px';
        textArea.style.opacity = '0';
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand('copy');
        document.body.removeChild(textArea);
    }
    
    showToast(message, type) {
        const toast = document.createElement('div');
        toast.textContent = message;
        toast.style.cssText = `
            background: ${type === 'success' ? '#10b981' : '#ef4444'};
            color: white;
            padding: 12px 16px;
            border-radius: 6px;
            margin-bottom: 8px;
            transform: translateX(100%);
            transition: transform 0.3s ease;
            font-size: 14px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            pointer-events: auto;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        `;
        
        const container = document.getElementById('toast-container');
        container.appendChild(toast);
        
        // Анімація появи
        requestAnimationFrame(() => {
            toast.style.transform = 'translateX(0)';
        });
        
        // Видалення через 3 секунди
        setTimeout(() => {
            toast.style.transform = 'translateX(100%)';
            setTimeout(() => {
                if (toast.parentNode) {
                    container.removeChild(toast);
                }
            }, 300);
        }, 3000);
    }
}

// Ініціалізація копіювача шляхів файлів
const filePathCopier = new FilePathCopier();

// Функція для показу контекстного меню файлу
function showFileContextMenu(event, file) {
    // Видаляємо існуюче меню якщо є
    const existingMenu = document.getElementById('file-context-menu');
    if (existingMenu) {
        existingMenu.remove();
    }

    const menu = document.createElement('div');
    menu.id = 'file-context-menu';
    menu.style.cssText = `
        position: fixed;
        top: ${event.pageY}px;
        left: ${event.pageX}px;
        background: white;
        border: 1px solid #ccc;
        border-radius: 6px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        z-index: 10000;
        padding: 8px 0;
        min-width: 200px;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        font-size: 14px;
    `;

    const menuItems = [
        {
            text: '📋 Копіювати шлях файлу',
            action: () => filePathCopier.copyFilePathWithInstruction(file.full_path)
        }
    ];

    menuItems.forEach(item => {
        const menuItem = document.createElement('div');
        menuItem.textContent = item.text;
        menuItem.style.cssText = `
            padding: 8px 16px;
            cursor: pointer;
            transition: background-color 0.2s;
        `;

        menuItem.addEventListener('mouseenter', () => {
            menuItem.style.backgroundColor = '#f5f5f5';
        });

        menuItem.addEventListener('mouseleave', () => {
            menuItem.style.backgroundColor = '';
        });

        menuItem.addEventListener('click', () => {
            item.action();
            menu.remove();
        });

        menu.appendChild(menuItem);
    });

    document.body.appendChild(menu);

    // Закриття меню при кліку поза ним
    const closeMenu = (e) => {
        if (!menu.contains(e.target)) {
            menu.remove();
            document.removeEventListener('click', closeMenu);
        }
    };

    setTimeout(() => {
        document.addEventListener('click', closeMenu);
    }, 100);
}


// Зберігання результатів пошуку
let searchResults = [];
let activeFileIndex = -1;
let displayedResults = [];
let currentQuery = ''; // Поточний запит пошуку
let totalCount = 0; // Загальна кількість результатів

// Українські голосні для стемінгу
const UKRAINIAN_VOWELS = 'аеєиіїоуюяь';

// Словник слів для припинення пошуку в файлах "особовий*"
const PERSONAL_FILE_STOP_WORDS = [
    'старш', 'молодш', 'солдат', 'сержант', 'штаб', 'лейтенант', 'майор', 'матрос', 'рекрут'
];

// Функція для перевірки чи ПОЧИНАЄТЬСЯ параграф з заборонених слів для особових файлів
function startsWithPersonalStopWords(paragraph) {
    const lowerParagraph = paragraph.toLowerCase().trim();
    return PERSONAL_FILE_STOP_WORDS.some(stopWord => lowerParagraph.startsWith(stopWord));
}

// Функція для перевірки чи ПОЧИНАЄТЬСЯ параграф з нумерації (наприклад "612.", "613.")
function startsWithNumbering(paragraph) {
    const trimmedParagraph = paragraph.trim();
    // Перевіряємо чи починається з цифр і закінчується крапкою
    return /^\d+\.\s/.test(trimmedParagraph);
}

// Комбінована функція для перевірки чи потрібно блокувати параграф в особових файлах
function shouldBlockParagraphInPersonalFile(paragraph) {
    return startsWithPersonalStopWords(paragraph) || startsWithNumbering(paragraph);
}



// Функції для стемінгу українських слів
function stripEndings(word) {
    word = word.toLowerCase();

    if (word.includes('-')) {
        const parts = word.split('-');
        const stemmedParts = parts.map(part => stripWordPart(part));
        return stemmedParts.join('-');
    }

    return stripWordPart(word);
}

function stripWordPart(word) {
    // Видаляємо закінчення -ець, -ця, -цю
    if (word.endsWith('ець')) {
        word = word.slice(0, -3);
    } else if (word.endsWith('ця')) {
        word = word.slice(0, -2);
    } else if (word.endsWith('цю')) {
        word = word.slice(0, -2);
    }

    // Видаляємо закінчення -ого
    if (word.endsWith('ого')) {
        word = word.slice(0, -3);
    }
    if (word.endsWith('ому')) {
        word = word.slice(0, -3);
    }

    // Видаляємо голосні та й/ь в кінці
    while (word && (UKRAINIAN_VOWELS.includes(word[word.length - 1]) || word[word.length - 1] === 'й')) {
        word = word.slice(0, -1);
    }

    return word;
}

function removeApostrophes(text) {
    return text.replace(/'/g, '');
}

function processSearchInput(input) {
    input = removeApostrophes(input);
    // Видаляємо коми в кінці слів
    input = input.replace(/,\s*$/, '').replace(/,(\s+)/g, '$1');
    const words = input.trim().split(/\s+/);
    const stemmedWords = words.map(word => stripEndings(word));
    return stemmedWords.join(' ');
}

function handleInputProcess(element) {
    element.value = processSearchInput(element.value);
}

// Обробник копіювання - копіює тільки чистий текст без форматування
document.addEventListener('copy', (event) => {
    const selection = window.getSelection();
    if (!selection || selection.toString().length === 0) {
        return;
    }

    // Перевіряємо чи копіюємо з document-preview
    const documentPreview = document.getElementById('document-preview');
    if (documentPreview && documentPreview.contains(selection.anchorNode)) {
        event.preventDefault();

        // Отримуємо виділений текст
        let plainText = selection.toString();

        // Поміщаємо в буфер тільки чистий текст
        event.clipboardData.setData('text/plain', plainText);

        // НЕ додаємо HTML форматування
        // event.clipboardData.setData('text/html', ...) - не викликаємо!
    }
});

// Ініціалізація при завантаженні сторінки
window.addEventListener('load', () => {
    // Обробник для Enter в полі пошуку
    searchInput.addEventListener('keyup', (event) => {
        if (event.key === 'Enter') {
            handleInputProcess(searchInput);
            performSearch();
        }
    });

    // Автовиділення тексту при фокусі
    searchInput.addEventListener('focus', function() {
        this.select();
    });

    // Обробка вставки тексту
    searchInput.addEventListener('paste', function(e) {
        e.preventDefault();
        const pastedText = e.clipboardData.getData('text');
        const processedText = processSearchInput(pastedText);

        const start = this.selectionStart;
        const end = this.selectionEnd;
        const text = this.value;
        const before = text.substring(0, start);
        const after = text.substring(end);

        this.value = before + processedText + after;
        this.selectionStart = this.selectionEnd = start + processedText.length;

        // Автоматично запускаємо пошук після обробки вставленого тексту
        setTimeout(() => {
            performSearch();
        }, 5); // Невелика затримка для завершення обробки DOM
    });

    // Обробник зміни режиму відображення
    document.querySelectorAll('input[name="view-mode"]').forEach(radio => {
        radio.addEventListener('change', () => {
            const currentQuery = searchInput.value.trim();
            
            if (displayedResults.length > 0 && currentQuery) {
                const newMode = getCurrentViewMode();
                
                if (newMode === 'fragments') {
                    // Переключились на режим Витяг - показуємо всі витяги
                    showAllExtracts(currentQuery);
                    
                    // Якщо ще не всі результати завантажені, запускаємо повний пошук
                    if (displayedResults.length < totalCount) {
                        performFullSearch(currentQuery);
                    }
                } else {
                    // Переключились на режим Повний документ - показуємо перший файл
                    if (activeFileIndex >= 0) {
                        selectFile(activeFileIndex, currentQuery);
                    } else {
                        selectFile(0, currentQuery);
                    }
                }
            }
        });
    });

    // Навігація клавіатурою
    document.addEventListener('keydown', (event) => {
        // Дозволяємо навігацію тільки коли активний елемент - не поле пошуку
        // і коли користувач не в процесі редагування тексту
        if (document.activeElement === searchInput ||
            document.activeElement.tagName === 'INPUT' ||
            document.activeElement.tagName === 'TEXTAREA') {
            return;
        }

        if (displayedResults.length === 0) {
            return;
        }

        if (event.key === 'ArrowDown' || event.key === 'z' || event.key === 'я') {
            const nextIndex = (activeFileIndex + 1) % displayedResults.length;
            selectFile(nextIndex, searchInput.value.trim());
            event.preventDefault();
        } else if (event.key === 'ArrowUp' || event.key === 'a' || event.key === 'ф') {
            const prevIndex = activeFileIndex <= 0 ? displayedResults.length - 1 : activeFileIndex - 1;
            selectFile(prevIndex, searchInput.value.trim());
            event.preventDefault();
        }
    });

    searchInput.focus();
});

// Основна функція пошуку
async function performSearch() {
    const query = searchInput.value.trim();

    if (!query) {
        showError('Введіть текст для пошуку');
        return;
    }

    // Захист від спаму - мінімум 3 символи
    if (query.length < 3) {
        showError('Введіть мінімум 3 символи для пошуку');
        return;
    }

    searchInput.value = removeApostrophes(query);
    showLoader();
    clearResults();
    hideError();

    try {
        // Зберігаємо поточний запит
        currentQuery = query;

        // Перший (швидкий) запит
        const viewMode = getCurrentViewMode();
        const response = await fetch('/api/search', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                query: query,
                full_search: false,
                view_mode: viewMode
            })
        });

        if (!response.ok) {
            throw new Error(`HTTP Error: ${response.status}`);
        }

        const result = await response.json();

        if (result.error) {
            showError(result.error);
            return;
        }

        // Оновлюємо глобальні змінні
        displayedResults = result.results;
        totalCount = result.total_count;

        displayResults(result, query);

        // Якщо є ще результати, запускаємо другий (повний) пошук
        console.log('🔍 Checking if need full search:', {
            displayedLength: displayedResults.length,
            totalCount: totalCount,
            needFullSearch: displayedResults.length < totalCount
        });
        
        if (displayedResults.length < totalCount) {
            console.log('🚀 Starting full search...');
            performFullSearch(query);
        } else {
            console.log('✅ No full search needed - all results found');
        }

    } catch (error) {
        showError(`Помилка під час пошуку: ${error.message || error}`);
    } finally {
        hideLoader();
        searchInput.focus();
        searchInput.select();
    }
}

async function performFullSearch(query) {
    console.log('🔍 performFullSearch called:', { query: query.substring(0, 20) });
    showLazyLoadingIndicator(); // Показуємо індикатор завантаження решти
    try {
        const viewMode = getCurrentViewMode();
        console.log('🔍 performFullSearch: sending request with full_search: true');
        const response = await fetch('/api/search', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                query: query,
                full_search: true,
                view_mode: viewMode
            })
        });

        if (!response.ok) {
            throw new Error(`HTTP Error: ${response.status}`);
        }

        const result = await response.json();
        console.log('✅ performFullSearch: received response:', {
            count: result.count,
            resultsLength: result.results ? result.results.length : 0,
            hasError: !!result.error
        });

        if (result.error) {
            // Не показуємо помилку, щоб не перекривати вже знайдені результати
            console.error("❌ Помилка повного пошуку:", result.error);
            return;
        }

        console.log('🎯 performFullSearch: calling appendResults');
        appendResults(result, query);

    } catch (error) {
        console.error(`Помилка під час повного пошуку: ${error.message || error}`);
    } finally {
        hideLazyLoadingIndicator();
    }
}

// Відображення результатів
function displayResults(result, query) {
    const { count, processing_time_ms } = result;
    
    console.log('🔍 displayResults called:', { 
        count, 
        displayedResultsLength: displayedResults.length,
        totalCount,
        viewMode: getCurrentViewMode()
    });

    // Показуємо інформаційну панель
    infoPanel.style.display = 'flex';

    // Показуємо інформацію про кількість результатів
    updateResultsStats();
    processingTime.textContent = `Час пошуку: ${processing_time_ms}мс`;

    // Завжди показуємо контейнер результатів
    resultsContainer.classList.remove('hidden');

    // Очищуємо список і рендеримо поточні результати
    filesList.innerHTML = '';

    if (count === 0) {
        // Показуємо "Нічого не знайдено" тільки якщо це фінальний результат (немає більше файлів для пошуку)
        if (displayedResults.length >= totalCount) {
            filesList.innerHTML = '<div class="no-results">Нічого не знайдено</div>';
            documentPreview.innerHTML = '';
        }
        // НЕ робимо return тут! Дозволяємо коду продовжувати для підготовки до другої партії
    } else {
        // Рендеримо всі поточні результати
        const fragment = document.createDocumentFragment();
        displayedResults.forEach((file, index) => {
            const fileElement = createFileElement(file, index, query);
            if (fileElement) {
                fragment.appendChild(fileElement);
            }
        });
        filesList.appendChild(fragment);

        // Відображуємо результати тільки якщо є результати в першій партії
        if (getCurrentViewMode() === 'fragments') {
            // В режимі Витяг показуємо всі результати разом
            showAllExtracts(query);
        } else {
            // В режимі Повний документ показуємо перший файл
            selectFile(0, query);
        }
    }
}

function appendResults(result, query) {
    const newResults = result.results;
    console.log('🔍 appendResults called:', { 
        newResultsLength: newResults.length, 
        currentDisplayedLength: displayedResults.length,
        viewMode: getCurrentViewMode()
    });
    
    if (newResults.length === 0) {
        console.log('❌ appendResults: немає нових результатів');
        return;
    }

    const currentLength = displayedResults.length;
    displayedResults = displayedResults.concat(newResults);
    totalCount = displayedResults.length;
    
    console.log('✅ appendResults: оновлено displayedResults:', {
        oldLength: currentLength,
        newLength: displayedResults.length
    });

    // Завжди додаємо файли до списку для навігації
    const fragment = document.createDocumentFragment();
    newResults.forEach((file, index) => {
        const fileElement = createFileElement(file, currentLength + index, query);
        if (fileElement) {
            fragment.appendChild(fileElement);
        }
    });
    filesList.appendChild(fragment);

    // Переконуємося що контейнер результатів видимий
    resultsContainer.classList.remove('hidden');
    
    // В режимі Витяг також додаємо нові витяги до контенту
    if (getCurrentViewMode() === 'fragments') {
        if (currentLength === 0) {
            // Якщо це перші результати (перший етап був порожній), показуємо всі витяги
            console.log('🔍 appendResults: показуємо всі витяги (перший етап був порожній)');
            // Спочатку очищуємо documentPreview, щоб не було старого контенту
            documentPreview.innerHTML = '';
            showAllExtracts(query);
        } else {
            // Додаємо нові витяги до існуючого контенту
            console.log('🔍 appendResults: додаємо нові витяги до існуючого контенту');
            appendExtracts(newResults, query);
        }
    } else {
        // Режим Повний документ - якщо це перші результати, показуємо перший файл
        if (currentLength === 0) {
            console.log('🔍 appendResults: показуємо перший файл (режим повний документ)');
            selectFile(0, query);
        }
    }

    updateResultsStats();
}


// Функція для створення елемента файлу
function createFileElement(file, index, query) {
    if (!file || !file.file_name) {
        return null;
    }

    // Фільтрація вже відбулася на стороні сервера, тут просто перевіряємо чи є збіги
    if (file.matches && file.matches.length === 0) {
        return null;
    }

    const fileElement = document.createElement('div');
    fileElement.className = 'file-item';
    fileElement.dataset.index = index;
    fileElement.dataset.filepath = file.full_path;

    const fileName = document.createElement('div');
    fileName.className = 'file-name';
    fileName.textContent = file.file_name;
    fileName.style.cursor = 'pointer';
    fileName.title = 'Подвійний клік для копіювання шляху';

    if (file.matches && file.matches.length > 0) {
        const matchCount = document.createElement('span');
        matchCount.className = 'match-count';
        matchCount.textContent = file.matches.length;

        // Якщо більше одного результату, збільшуємо розмір шрифту
        if (file.matches.length > 1) {
            matchCount.style.fontSize = '18px'; // 12px * 1.5 = 18px
        }

        fileName.appendChild(matchCount);
    }

    fileElement.appendChild(fileName);

    fileElement.addEventListener('click', (event) => {
        // Якщо клікнули на назву файлу, не виконуємо вибір файлу
        if (event.target.className === 'file-name') {
            return;
        }
        selectFile(index, query);
    });

    // Обробник подвійного кліку на назві файлу для копіювання шляху
    fileName.addEventListener('dblclick', async (event) => {
        event.preventDefault();
        event.stopPropagation();
        await filePathCopier.copyFilePathWithInstruction(file.full_path);
    });

    // Обробник правого кліку для контекстного меню
    fileName.addEventListener('contextmenu', (event) => {
        event.preventDefault();
        showFileContextMenu(event, file);
    });

    // Обробник одинарного кліку на назві файлу для вибору файлу
    fileName.addEventListener('click', (event) => {
        event.preventDefault();
        event.stopPropagation();
        selectFile(index, query);
    });


    return fileElement;
}

// Функція для відображення всіх витягів
function showAllExtracts(query) {
    console.log('🔍 showAllExtracts called:', {
        displayedResultsLength: displayedResults.length,
        query: query.substring(0, 20)
    });
    
    documentPreview.innerHTML = '';
    
    const documentContent = document.createElement('div');
    documentContent.className = 'document-content';
    
    displayedResults.forEach((file) => {
        // Створюємо контейнер для файлу
        const fileContainer = document.createElement('div');
        fileContainer.className = 'file-container';
        fileContainer.style.marginBottom = '30px';
        
        // Додаємо назву файлу як заголовок один раз
        const fileHeader = document.createElement('div');
        fileHeader.style.fontSize = '1.2em';
        fileHeader.style.color = '#0066cc';
        fileHeader.style.marginBottom = '15px';
        fileHeader.style.paddingBottom = '8px';
        fileHeader.style.borderBottom = '2px solid #0066cc';
        fileHeader.style.fontWeight = 'bold';
        fileHeader.textContent = file.file_name;
        fileContainer.appendChild(fileHeader);
        
        // Додаємо всі витяги для цього файлу
        file.matches.forEach((match) => {
            const extractSection = document.createElement('div');
            extractSection.className = 'extract-section';
            extractSection.style.padding = '15px 0 15px 20px';
            extractSection.style.borderBottom = '1px solid #e0e0e0';
            extractSection.style.borderLeft = '4px solid #ffc107';
            extractSection.style.marginBottom = '15px';

            let content = '';
            
            // Отримуємо батьківські параграфи для контексту
            if (file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
                const isPersonalFile = file.file_name.toLowerCase().startsWith('особовий');
                let parentParagraphs = [];
                
                if (isPersonalFile) {
                    // Для файлів "особовий" шукаємо батьківський параграф з §
                    for (let i = match.position - 1; i >= 0; i--) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        if (paragraph.startsWith('§')) {
                            parentParagraphs = [paragraph];
                            break;
                        }
                    }
                } else {
                    // Стандартна логіка для звичайних файлів
                    parentParagraphs = getParentParagraphs(file.all_paragraphs, match.position);
                }
                
                // Додаємо батьківські параграфи
                if (parentParagraphs.length > 0) {
                    parentParagraphs.forEach((parentText) => {
                        let style = "color: #444; font-size: 0.95em; margin-bottom: 8px; font-weight: 500; line-height: 1.4;";
                        
                        if (isPersonalFile && parentText.startsWith('§')) {
                            // Стиль для § параграфів в особових файлах
                            style = "color: #0066cc; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                        } else if (parentText.match(/^\d+\.\s+/)) {
                            // Перевіряємо чи це параграф першого рівня з особливими фразами
                            if (parentText.includes("Вважати такими, що прибули")) {
                                style = "color: #cc0000; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                            } else if (parentText.includes("Вважати такими, що вибули")) {
                                style = "color: #009900; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                            }
                        }
                        
                        content += `<div style="${style}">${parentText}</div>`;
                    });
                }
            }
            
            // Основний текст збігу (знайдений фрагмент)
            content += `<div style="margin-bottom: 10px; line-height: 1.4;">${highlightText(match.context, query).replace(/\n/g, '<br>')}</div>`;
            
            // Додаємо додаткові параграфи до підстави або до § для файлів "особовий"
            if (file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
                let additionalParagraphs = [];
                let basisParagraph = null;
                
                // Перевіряємо чи це файл "особовий"
                const isPersonalFile = file.file_name.toLowerCase().startsWith('особовий');
                
                if (isPersonalFile) {
                    // Логіка для файлів "особовий": витягуємо до знака § (але § не показуємо)
                    // Якщо зустрічається заборонений параграф - блокуємо всі наступні в цьому розділі
                    let isBlocked = false;
                    for (let i = match.position + 1; i < file.all_paragraphs.length; i++) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        
                        if (paragraph.startsWith('§')) {
                            // Знайшли наступний § - зупиняємось
                            break;
                        } else if (paragraph.length > 0) {
                            if (!isBlocked) {
                                // Перевіряємо чи цей параграф потрібно блокувати (заборонені слова або нумерація)
                                if (shouldBlockParagraphInPersonalFile(paragraph)) {
                                    // Блокуємо цей та всі наступні параграфи в розділі
                                    isBlocked = true;
                                } else {
                                    // Додаємо параграф, якщо він не заборонений
                                    additionalParagraphs.push(paragraph);
                                }
                            }
                            // Якщо isBlocked = true, пропускаємо всі наступні параграфи
                        }
                    }
                } else {
                    // Стандартна логіка для звичайних файлів
                    const currentText = match.context.trim();
                    const sectionMatch = currentText.match(/^(\d+(\.\d+)*)\./);
                    let sectionPrefix = '';
                    
                    if (sectionMatch) {
                        sectionPrefix = sectionMatch[1].split('.')[0];
                    }
                    
                    for (let i = match.position + 1; i < file.all_paragraphs.length; i++) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        
                        if (paragraph.toLowerCase().startsWith('підстава')) {
                            basisParagraph = paragraph;
                            break;
                        } else if (paragraph.match(/^\d+\./)) {
                            const newSectionMatch = paragraph.match(/^(\d+)\./);
                            if (newSectionMatch && sectionPrefix && newSectionMatch[1] !== sectionPrefix) {
                                break;
                            } else if (paragraph.match(/^\d+(\.\d+)+\./)) {
                                continue;
                            }
                        } else if (paragraph.length > 0) {
                            additionalParagraphs.push(paragraph);
                        }
                    }
                }
                
                additionalParagraphs.forEach(p => {
                    content += '<div style="color: #555; line-height: 1.4; margin-top: 5px;">' + 
                              p.replace(/\n/g, '<br>') + '</div>';
                });
                
                if (basisParagraph && !isPersonalFile) {
                    // Показуємо basisParagraph тільки для звичайних файлів (не особових)
                    const basisStyle = 'font-style: italic; color: #666; line-height: 1.4;';
                    content += `<div style="${basisStyle}">` + 
                              basisParagraph.replace(/\n/g, '<br>') + '</div>';
                }
            }

            extractSection.innerHTML = content;
            fileContainer.appendChild(extractSection);
        });
        
        documentContent.appendChild(fileContainer);
    });
    
    documentPreview.appendChild(documentContent);
    
    // Прокручуємо до початку результатів
    setTimeout(() => {
        documentPreview.scrollTop = 0;
    }, 100);
}

// Функція для додавання нових витягів до існуючого контенту
function appendExtracts(newResults, query) {
    const documentContent = document.querySelector('.document-content');
    if (!documentContent) return;
    
    newResults.forEach((file) => {
        // Створюємо контейнер для файлу
        const fileContainer = document.createElement('div');
        fileContainer.className = 'file-container';
        fileContainer.style.marginBottom = '30px';
        
        // Додаємо назву файлу як заголовок один раз
        const fileHeader = document.createElement('div');
        fileHeader.style.fontSize = '1.2em';
        fileHeader.style.color = '#0066cc';
        fileHeader.style.marginBottom = '15px';
        fileHeader.style.paddingBottom = '8px';
        fileHeader.style.borderBottom = '2px solid #0066cc';
        fileHeader.style.fontWeight = 'bold';
        fileHeader.textContent = file.file_name;
        fileContainer.appendChild(fileHeader);
        
        // Додаємо всі витяги для цього файлу
        file.matches.forEach((match) => {
            const extractSection = document.createElement('div');
            extractSection.className = 'extract-section';
            extractSection.style.padding = '15px 0 15px 20px';
            extractSection.style.borderBottom = '1px solid #e0e0e0';
            extractSection.style.borderLeft = '4px solid #ffc107';
            extractSection.style.marginBottom = '15px';

            let content = '';
            
            // Отримуємо батьківські параграфи для контексту
            if (file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
                const isPersonalFile = file.file_name.toLowerCase().startsWith('особовий');
                let parentParagraphs = [];
                
                if (isPersonalFile) {
                    // Для файлів "особовий" шукаємо батьківський параграф з §
                    for (let i = match.position - 1; i >= 0; i--) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        if (paragraph.startsWith('§')) {
                            parentParagraphs = [paragraph];
                            break;
                        }
                    }
                } else {
                    // Стандартна логіка для звичайних файлів
                    parentParagraphs = getParentParagraphs(file.all_paragraphs, match.position);
                }
                
                // Додаємо батьківські параграфи
                if (parentParagraphs.length > 0) {
                    parentParagraphs.forEach((parentText) => {
                        let style = "color: #444; font-size: 0.95em; margin-bottom: 8px; font-weight: 500; line-height: 1.4;";
                        
                        if (isPersonalFile && parentText.startsWith('§')) {
                            // Стиль для § параграфів в особових файлах
                            style = "color: #0066cc; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                        } else if (parentText.match(/^\d+\.\s+/)) {
                            // Перевіряємо чи це параграф першого рівня з особливими фразами
                            if (parentText.includes("Вважати такими, що прибули")) {
                                style = "color: #cc0000; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                            } else if (parentText.includes("Вважати такими, що вибули")) {
                                style = "color: #009900; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                            }
                        }
                        
                        content += `<div style="${style}">${parentText}</div>`;
                    });
                }
            }
            
            // Основний текст збігу (знайдений фрагмент)
            content += `<div style="margin-bottom: 10px; line-height: 1.4;">${highlightText(match.context, query).replace(/\n/g, '<br>')}</div>`;
            
            // Додаємо додаткові параграфи до підстави або до § для файлів "особовий"
            if (file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
                let additionalParagraphs = [];
                let basisParagraph = null;
                
                // Перевіряємо чи це файл "особовий"
                const isPersonalFile = file.file_name.toLowerCase().startsWith('особовий');
                
                if (isPersonalFile) {
                    // Логіка для файлів "особовий": витягуємо до знака § (але § не показуємо)
                    // Якщо зустрічається заборонений параграф - блокуємо всі наступні в цьому розділі
                    let isBlocked = false;
                    for (let i = match.position + 1; i < file.all_paragraphs.length; i++) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        
                        if (paragraph.startsWith('§')) {
                            // Знайшли наступний § - зупиняємось
                            break;
                        } else if (paragraph.length > 0) {
                            if (!isBlocked) {
                                // Перевіряємо чи цей параграф потрібно блокувати (заборонені слова або нумерація)
                                if (shouldBlockParagraphInPersonalFile(paragraph)) {
                                    // Блокуємо цей та всі наступні параграфи в розділі
                                    isBlocked = true;
                                } else {
                                    // Додаємо параграф, якщо він не заборонений
                                    additionalParagraphs.push(paragraph);
                                }
                            }
                            // Якщо isBlocked = true, пропускаємо всі наступні параграфи
                        }
                    }
                } else {
                    // Стандартна логіка для звичайних файлів
                    const currentText = match.context.trim();
                    const sectionMatch = currentText.match(/^(\d+(\.\d+)*)\./);
                    let sectionPrefix = '';
                    
                    if (sectionMatch) {
                        sectionPrefix = sectionMatch[1].split('.')[0];
                    }
                    
                    for (let i = match.position + 1; i < file.all_paragraphs.length; i++) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        
                        if (paragraph.toLowerCase().startsWith('підстава')) {
                            basisParagraph = paragraph;
                            break;
                        } else if (paragraph.match(/^\d+\./)) {
                            const newSectionMatch = paragraph.match(/^(\d+)\./);
                            if (newSectionMatch && sectionPrefix && newSectionMatch[1] !== sectionPrefix) {
                                break;
                            } else if (paragraph.match(/^\d+(\.\d+)+\./)) {
                                continue;
                            }
                        } else if (paragraph.length > 0) {
                            additionalParagraphs.push(paragraph);
                        }
                    }
                }
                
                additionalParagraphs.forEach(p => {
                    content += '<div style="color: #555; line-height: 1.4; margin-top: 5px;">' + 
                              p.replace(/\n/g, '<br>') + '</div>';
                });
                
                if (basisParagraph && !isPersonalFile) {
                    // Показуємо basisParagraph тільки для звичайних файлів (не особових)
                    const basisStyle = 'font-style: italic; color: #666; line-height: 1.4;';
                    content += `<div style="${basisStyle}">` + 
                              basisParagraph.replace(/\n/g, '<br>') + '</div>';
                }
            }

            extractSection.innerHTML = content;
            fileContainer.appendChild(extractSection);
        });
        
        documentContent.appendChild(fileContainer);
    });
}

// Функція для прокрутки до файлу в витягах
function scrollToFileInExtracts(fileName) {
    // Шукаємо контейнер файлу з відповідною назвою
    const fileContainers = document.querySelectorAll('.file-container');
    
    for (const container of fileContainers) {
        const fileHeader = container.querySelector('div'); // Перший div це заголовок
        if (fileHeader && fileHeader.textContent.trim() === fileName) {
            // Прокручуємо до заголовку файлу
            fileHeader.scrollIntoView({ 
                behavior: 'smooth', 
                block: 'start' 
            });
            
            // Додаємо тимчасове виділення
            fileHeader.style.backgroundColor = '#e6f3ff';
            setTimeout(() => {
                fileHeader.style.backgroundColor = '';
            }, 2000);
            
            break;
        }
    }
}

// Функція для оновлення статистики результатів
function updateResultsStats() {
    searchStats.textContent = `Знайдено: ${totalCount} файл(ів)`;
}

// Отримати поточний режим відображення
function getCurrentViewMode() {
    const checkedMode = document.querySelector('input[name="view-mode"]:checked');
    return checkedMode ? checkedMode.value : 'fragments';
}

// Вибір файлу та відображення вмісту
function selectFile(fileIndex, query) {
    const allFileItems = document.querySelectorAll('.file-item');
    allFileItems.forEach(item => item.classList.remove('active'));

    const selectedFileItem = document.querySelector(`.file-item[data-index="${fileIndex}"]`);
    if (selectedFileItem) {
        selectedFileItem.classList.add('active');
        selectedFileItem.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }

    activeFileIndex = fileIndex;
    const file = displayedResults[fileIndex];
    const viewMode = getCurrentViewMode();
    
    // В режимі Витяг просто прокручуємо до файлу
    if (viewMode === 'fragments') {
        scrollToFileInExtracts(file.file_name);
        return;
    }

    // Режим Повний документ - показуємо один файл
    documentPreview.innerHTML = '';

    const documentContent = document.createElement('div');
    documentContent.className = 'document-content';

    let firstMatchElement = null;
    let paragraphCount = 0;
    
    // Діагностика для перевірки даних
    console.log('🔍 Debug selectFile:', {
        viewMode,
        fileName: file.file_name,
        hasAllParagraphs: !!file.all_paragraphs,
        allParagraphsLength: file.all_paragraphs ? file.all_paragraphs.length : 0,
        matchesLength: file.matches ? file.matches.length : 0
    });

    if (viewMode === 'full-document' && file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
        file.all_paragraphs.forEach((paragraphData, index) => {
            // Підтримка старого і нового формату
            const text = typeof paragraphData === 'string' ? paragraphData : paragraphData.text;
            const lineBreaksAfter = typeof paragraphData === 'object' ? (paragraphData.line_breaks_after || 0) : 0;

            if (!text.trim()) {
                // Додаємо порожній рядок як переніс
                const emptyLine = document.createElement('div');
                emptyLine.style.height = '1.2em';
                documentContent.appendChild(emptyLine);
                return;
            }

            const paragraph = document.createElement('div');
            paragraph.className = 'paragraph';
            paragraph.style.marginBottom = '0'; // Базовий відступ - 0
            paragraph.style.whiteSpace = 'pre-wrap'; // Зберігаємо переноси рядків
            paragraph.style.lineHeight = '1.4';

            let isMatch = false;
            for (const match of file.matches) {
                if (match.context.trim() === text.trim()) {
                    isMatch = true;
                    paragraph.className += ' found-text';
                    paragraph.innerHTML = highlightText(text, query).replace(/\n/g, '<br>');

                    if (!firstMatchElement) {
                        firstMatchElement = paragraph;
                    }
                    break;
                }
            }

            if (!isMatch) {
                paragraph.innerHTML = text.replace(/\n/g, '<br>');
            }

            documentContent.appendChild(paragraph);
            paragraphCount++;

            // Додаємо розриви рядків після параграфа якщо вони є
            for (let i = 0; i < lineBreaksAfter; i++) {
                const emptyLine = document.createElement('div');
                emptyLine.style.height = '1.2em';
                documentContent.appendChild(emptyLine);
            }
        });
    } else {
        // Режим фрагментів - показуємо знайдені фрагменти з батьківськими параграфами
        file.matches.forEach((match) => {
            const extractSection = document.createElement('div');
            extractSection.className = 'extract-section';
            extractSection.style.padding = '15px 0 15px 20px';
            extractSection.style.borderBottom = '2px solid #e0e0e0';
            extractSection.style.borderLeft = '4px solid #ffc107';
            extractSection.style.marginBottom = '20px';

            // Додаємо назву файлу як заголовок витягу
            let content = `<div style="font-size: 1.2em; color: #0066cc; margin-bottom: 15px; padding-bottom: 8px; border-bottom: 1px solid #ddd; font-weight: bold;">${file.file_name}</div>`;
            
            // Отримуємо батьківські параграфи для контексту
            
            if (file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
                const isPersonalFile = file.file_name.toLowerCase().startsWith('особовий');
                let parentParagraphs = [];
                
                if (isPersonalFile) {
                    // Для файлів "особовий" шукаємо батьківський параграф з §
                    for (let i = match.position - 1; i >= 0; i--) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        if (paragraph.startsWith('§')) {
                            parentParagraphs = [paragraph];
                            break;
                        }
                    }
                } else {
                    // Стандартна логіка для звичайних файлів
                    parentParagraphs = getParentParagraphs(file.all_paragraphs, match.position);
                }
                
                // Додаємо батьківські параграфи
                if (parentParagraphs.length > 0) {
                    parentParagraphs.forEach((parentText) => {
                        let style = "color: #444; font-size: 0.95em; margin-bottom: 8px; font-weight: 500; line-height: 1.4;";
                        
                        if (isPersonalFile && parentText.startsWith('§')) {
                            // Стиль для § параграфів в особових файлах
                            style = "color: #0066cc; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                        } else if (parentText.match(/^\d+\.\s+/)) {
                            // Перевіряємо чи це параграф першого рівня з особливими фразами
                            if (parentText.includes("Вважати такими, що прибули")) {
                                style = "color: #cc0000; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                            } else if (parentText.includes("Вважати такими, що вибули")) {
                                style = "color: #009900; font-size: 0.95em; margin-bottom: 8px; font-weight: bold; line-height: 1.4;";
                            }
                        }
                        
                        content += `<div style="${style}">${parentText}</div>`;
                    });
                }
            }
            
            // Основний текст збігу (знайдений фрагмент)
            content += `<div style="margin-bottom: 10px; line-height: 1.4;">${highlightText(match.context, query).replace(/\n/g, '<br>')}</div>`;
            
            // Додаємо додаткові параграфи до підстави або до § для файлів "особовий"
            if (file.all_paragraphs && Array.isArray(file.all_paragraphs)) {
                let additionalParagraphs = [];
                let basisParagraph = null;
                
                // Перевіряємо чи це файл "особовий"
                const isPersonalFile = file.file_name.toLowerCase().startsWith('особовий');
                
                if (isPersonalFile) {
                    // Логіка для файлів "особовий": витягуємо до знака § (але § не показуємо)
                    // Якщо зустрічається заборонений параграф - блокуємо всі наступні в цьому розділі
                    let isBlocked = false;
                    for (let i = match.position + 1; i < file.all_paragraphs.length; i++) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        
                        if (paragraph.startsWith('§')) {
                            // Знайшли наступний § - зупиняємось
                            break;
                        } else if (paragraph.length > 0) {
                            if (!isBlocked) {
                                // Перевіряємо чи цей параграф потрібно блокувати (заборонені слова або нумерація)
                                if (shouldBlockParagraphInPersonalFile(paragraph)) {
                                    // Блокуємо цей та всі наступні параграфи в розділі
                                    isBlocked = true;
                                } else {
                                    // Додаємо параграф, якщо він не заборонений
                                    additionalParagraphs.push(paragraph);
                                }
                            }
                            // Якщо isBlocked = true, пропускаємо всі наступні параграфи
                        }
                    }
                } else {
                    // Стандартна логіка для звичайних файлів
                    const currentText = match.context.trim();
                    const sectionMatch = currentText.match(/^(\d+(\.\d+)*)\./);
                    let sectionPrefix = '';
                    
                    if (sectionMatch) {
                        sectionPrefix = sectionMatch[1].split('.')[0]; // Беремо перший номер (наприклад "25" з "25.1.194")
                    }
                    
                    // Шукаємо параграфи після основного тексту
                    for (let i = match.position + 1; i < file.all_paragraphs.length; i++) {
                        const paragraph = getParagraphText(file.all_paragraphs[i]).trim();
                        
                        if (paragraph.toLowerCase().startsWith('підстава')) {
                            // Знайшли підставу
                            basisParagraph = paragraph;
                            break;
                        } else if (paragraph.match(/^\d+\./)) {
                            // Це новий розділ першого рівня
                            const newSectionMatch = paragraph.match(/^(\d+)\./);
                            if (newSectionMatch && sectionPrefix && newSectionMatch[1] !== sectionPrefix) {
                                // Новий розділ з іншим номером - зупиняємось
                                break;
                            } else if (paragraph.match(/^\d+(\.\d+)+\./)) {
                                // Це підрозділ того ж розділу - пропускаємо (не додаємо)
                                continue;
                            }
                        } else if (paragraph.length > 0) {
                            // Це звичайний текст - додаємо
                            additionalParagraphs.push(paragraph);
                        }
                    }
                }
                
                // Додаємо всі знайдені додаткові параграфи
                additionalParagraphs.forEach(p => {
                    content += '<div style="color: #555; line-height: 1.4; margin-top: 5px;">' + 
                              p.replace(/\n/g, '<br>') + '</div>';
                });
                
                // Додаємо підставу якщо знайшли (тільки для звичайних файлів, не особових)
                if (basisParagraph && !isPersonalFile) {
                    content += '<div style="font-style: italic; color: #666; line-height: 1.4;">' + 
                              basisParagraph.replace(/\n/g, '<br>') + '</div>';
                }
            }

            extractSection.innerHTML = content;

            if (!firstMatchElement) {
                firstMatchElement = extractSection;
            }

            documentContent.appendChild(extractSection);
            paragraphCount++;
        });
    }


    documentPreview.appendChild(documentContent);

    // Прокрутити до першого збігу
    if (firstMatchElement) {
        setTimeout(() => {
            firstMatchElement.scrollIntoView({
                behavior: 'smooth',
                block: 'center'
            });
        }, 100);
    }

    setTimeout(() => {
        addScrollMarkers();
    }, 300);
}

// Виділення знайденого тексту
function highlightText(text, query) {

    const queryParts = query.toLowerCase().trim().split(/\s+/);
    const isNameSearch = queryParts.length >= 2 && queryParts.length <= 3 &&
        queryParts.every(part => part.length >= 2);

    let highlightedText = text;
    let hasHighlight = false;

    if (isNameSearch) {
        // Пошук повної послідовності
        const fullQueryRegex = new RegExp(
            `(${queryParts.map(part => `${escapeRegExp(part)}[а-яіїєґ]*`).join('\\s+')})`,
            'gi'
        );

        let fullMatches = [];
        let match;

        while ((match = fullQueryRegex.exec(text)) !== null) {
            fullMatches.push({
                index: match.index,
                text: match[0]
            });
            hasHighlight = true;
        }

        if (fullMatches.length > 0) {
            for (let i = fullMatches.length - 1; i >= 0; i--) {
                const matchData = fullMatches[i];
                const prefix = highlightedText.substring(0, matchData.index);
                const suffix = highlightedText.substring(matchData.index + matchData.text.length);
                highlightedText = prefix + `<span class="highlight">${matchData.text}</span>` + suffix;
            }
            return highlightedText;
        }

        // Пошук патерну ПРІЗВИЩЕ Ім'я
        const capsNameRegex = /\b([A-ZА-ЯІЇЄҐ]{2,})\s+([A-ZА-ЯІЇЄҐ][a-zа-яіїєґ]+)\b/g;
        const capsMatches = [];

        while ((match = capsNameRegex.exec(text)) !== null) {
            const fullName = match[0].toLowerCase();
            if (queryParts.every(part => fullName.includes(part))) {
                capsMatches.push({
                    index: match.index,
                    text: match[0]
                });
                hasHighlight = true;
            }
        }

        for (let i = capsMatches.length - 1; i >= 0; i--) {
            const matchData = capsMatches[i];
            const prefix = highlightedText.substring(0, matchData.index);
            const suffix = highlightedText.substring(matchData.index + matchData.text.length);
            highlightedText = prefix + `<span class="highlight">${matchData.text}</span>` + suffix;
        }

        if (hasHighlight) {
            return highlightedText;
        }
    }

    // Звичайне виділення для кожного слова окремо
    queryParts.forEach(queryPart => {
        const regex = new RegExp(escapeRegExp(queryPart), 'gi');
        if (regex.test(text)) {
            hasHighlight = true;
            highlightedText = highlightedText.replace(regex, match => `<span class="highlight">${match}</span>`);
        }
    });


    return highlightedText;
}

function escapeRegExp(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// Відкриття файлу з паролем
function openFile(filePath) {
    const savedPassword = localStorage.getItem('fileOpenPassword');
    const correctPassword = '4053@115';

    if (savedPassword === correctPassword) {
        // Пароль збережений і правильний - відкриваємо файл
        openFileDirectly(filePath);
        return;
    }

    // Запитуємо пароль
    const enteredPassword = prompt('Введіть пароль для відкриття файлу:');

    if (enteredPassword === null) {
        // Користувач натиснув Cancel
        return;
    }

    if (enteredPassword === correctPassword) {
        // Пароль правильний - зберігаємо в localStorage і відкриваємо файл
        localStorage.setItem('fileOpenPassword', correctPassword);
        openFileDirectly(filePath);
    } else {
        // Неправильний пароль
        alert('Неправильний пароль!');
    }
}

// Функція для безпосереднього відкриття файлу
async function openFileDirectly(filePath) {
    try {
        const savedPassword = localStorage.getItem('fileOpenPassword');
        const correctPassword = '4053@115';

        // Використовуємо збережений пароль або той що був перевірений раніше
        const passwordToUse = savedPassword || correctPassword;

        const response = await fetch('/api/open-file', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                file_path: filePath,
                password: passwordToUse
            })
        });

        const result = await response.json();

        if (response.ok) {
            console.log('Файл успішно відкрито:', filePath);
            // Показуємо повідомлення про успіх (опціонально)
            // alert('Файл відкрито!');
        } else {
            throw new Error(result.error || 'Невідома помилка');
        }
    } catch (error) {
        console.error('Помилка при відкритті файлу:', error);
        alert(`Помилка при відкритті файлу: ${error.message}\nШлях: ${filePath}`);
    }
}

// Функція для отримання батьківських параграфів з ієрархією (до 4 рівнів)
function getParentParagraphs(allParagraphs, matchPosition) {
    if (!allParagraphs || matchPosition < 0 || matchPosition >= allParagraphs.length) {
        return [];
    }

    const parentParagraphs = [];
    const currentParagraph = getParagraphText(allParagraphs[matchPosition]).trim();

    // Визначаємо рівень поточного параграфу
    let currentLevel = 999; // Якщо це не заголовок, шукаємо всі батьківські заголовки
    if (isHierarchyHeader(currentParagraph)) {
        currentLevel = getHierarchyLevel(currentParagraph);
    }

    // Шукаємо батьківські параграфи, йдучи назад від знайденої позиції
    for (let i = matchPosition - 1; i >= 0; i--) {
        const paragraph = getParagraphText(allParagraphs[i]).trim();
        
        // Перевіряємо чи це заголовок розділу (починається з цифр та крапки)
        if (isHierarchyHeader(paragraph)) {
            const level = getHierarchyLevel(paragraph);
            
            // Для звичайного тексту (currentLevel = 999) додаємо всі батьківські заголовки
            // Для заголовків додаємо тільки вищі рівні (менший номер = вищий рівень)
            if (level > 0 && level < currentLevel) {
                parentParagraphs.unshift(paragraph);
                currentLevel = level; // Оновлюємо поточний рівень для пошуку батьків
                
                // Якщо це заголовок першого рівня (наприклад "25. "), зупиняємось
                if (level === 1) {
                    break;
                }
                
                // Обмежуємо до 4 рівнів ієрархії
                if (parentParagraphs.length >= 4) {
                    break;
                }
            }
        }
    }
    
    return parentParagraphs;
}

// Перевіряє чи є параграф заголовком ієрархії
function isHierarchyHeader(paragraph) {
    // Шаблони для заголовків: "1.", "1.1.", "1.1.1.", "1.1.1.1."
    const hierarchyPattern = /^\d+(\.\d+){0,3}\.\s+/;
    return hierarchyPattern.test(paragraph);
}

// Визначає рівень ієрархії заголовку
function getHierarchyLevel(paragraph) {
    const match = paragraph.match(/^(\d+(\.\d+)*)\.\s+/);
    if (!match) return 0;
    
    const numbers = match[1];
    // Рахуємо кількість крапок + 1 = рівень ієрархії
    return (numbers.match(/\./g) || []).length + 1;
}

// Допоміжні функції
function showLoader() {
    loader.classList.remove('hidden');
}

function hideLoader() {
    loader.classList.add('hidden');
}

function clearResults() {
    filesList.innerHTML = '';
    documentPreview.innerHTML = '';
    resultsContainer.classList.add('hidden');
    infoPanel.style.display = 'none';
    searchResults = [];
    displayedResults = [];
    activeFileIndex = -1;
    currentQuery = '';
    totalCount = 0;
}

function showError(message) {
    errorMessage.textContent = message;
    errorMessage.classList.remove('hidden');
}

function hideError() {
    errorMessage.classList.add('hidden');
}

// Додавання маркерів на скрол-барі
function addScrollMarkers() {
    const existingMarkers = document.querySelectorAll('.marker-indicator');
    existingMarkers.forEach(marker => marker.remove());

    const existingPanels = document.querySelectorAll('.marker-panel');
    existingPanels.forEach(panel => panel.remove());

    // Додаємо маркери тільки для параграфів які ДІЙСНО містять підсвічений текст
    const foundTextElements = document.querySelectorAll('.paragraph.found-text .highlight');
    const parentElements = [...new Set([...foundTextElements].map(el => el.closest('.paragraph.found-text')))];


    if (parentElements.length === 0) return;

    const preview = document.getElementById('document-preview');
    const previewHeight = preview.clientHeight;
    const scrollHeight = preview.scrollHeight;

    const markerPanel = document.createElement('div');
    markerPanel.className = 'marker-panel';
    const previewPanel = document.querySelector('.content-preview-panel');
    previewPanel.appendChild(markerPanel);

    const scrollHandle = document.createElement('div');
    scrollHandle.className = 'scroll-handle';
    markerPanel.appendChild(scrollHandle);

    function updateScrollHandlePosition() {
        const scrollPosition = preview.scrollTop / (scrollHeight - previewHeight);
        const handlePosition = scrollPosition * previewHeight;
        scrollHandle.style.top = `${handlePosition}px`;
    }

    let isDragging = false;
    scrollHandle.addEventListener('mousedown', (e) => {
        isDragging = true;
        e.preventDefault();
    });

    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;

        const markerPanelRect = markerPanel.getBoundingClientRect();
        const relativePosition = (e.clientY - markerPanelRect.top) / previewHeight;
        const clampedPosition = Math.max(0, Math.min(1, relativePosition));

        preview.scrollTop = clampedPosition * (scrollHeight - previewHeight);
    });

    document.addEventListener('mouseup', () => {
        isDragging = false;
    });

    updateScrollHandlePosition();
    preview.addEventListener('scroll', updateScrollHandlePosition);

    parentElements.forEach((element, index) => {
        const elementTop = element.offsetTop;
        const relativePosition = elementTop / scrollHeight;
        const markerPosition = relativePosition * previewHeight;

        const marker = document.createElement('div');
        marker.className = 'marker-indicator';
        marker.title = `Збіг ${index + 1} з ${parentElements.length}`;
        marker.style.top = `${markerPosition}px`;
        marker.dataset.index = index;

        marker.addEventListener('click', (e) => {
            e.stopPropagation();
            element.scrollIntoView({ block: 'center' });
        });

        markerPanel.appendChild(marker);
    });

    preview.addEventListener('scroll', updateScrollMarkersVisibility);
    window.addEventListener('resize', addScrollMarkers);
}

function updateScrollMarkersVisibility() {
    const preview = document.getElementById('document-preview');
    const markers = document.querySelectorAll('.marker-indicator');

    if (markers.length === 0) return;

    // Використовуємо тільки параграфи що дійсно мають підсвітку
    const foundTextElements = document.querySelectorAll('.paragraph.found-text .highlight');
    const parentElements = [...new Set([...foundTextElements].map(el => el.closest('.paragraph.found-text')))];

    markers.forEach((marker, index) => {
        if (index >= parentElements.length) return;

        const textElement = parentElements[parseInt(marker.dataset.index) || index];
        if (!textElement) return;

        const elementRect = textElement.getBoundingClientRect();
        const previewRect = preview.getBoundingClientRect();

        const isVisible =
            (elementRect.top >= previewRect.top && elementRect.top <= previewRect.bottom) ||
            (elementRect.bottom >= previewRect.top && elementRect.bottom <= previewRect.bottom) ||
            (elementRect.top <= previewRect.top && elementRect.bottom >= previewRect.bottom);

        if (isVisible) {
            marker.classList.add('visible');
        } else {
            marker.classList.remove('visible');
        }
    });
}

function showLazyLoadingIndicator() {
    const indicator = document.getElementById('lazy-loading-indicator');
    if (indicator) {
        indicator.classList.remove('hidden');
    }
}

function hideLazyLoadingIndicator() {
    const indicator = document.getElementById('lazy-loading-indicator');
    if (indicator) {
        indicator.classList.add('hidden');
    }
}