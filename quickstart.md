# Blazing Search - Посібник зі швидкого запуску

Цей посібник допоможе вам швидко розгорнути та запустити додаток Blazing Search за допомогою Docker.

## Передумови

- Docker встановлений у вашій системі
- Docker Compose встановлений (зазвичай входить до складу Docker Desktop)
- Доступ до мережевого ресурсу SMB (сервер salem)

## Інструкції з налаштування

### 1. Клонуйте репозиторій

```bash
git clone <repository-url>
cd blazing_SEARCH
```

### 2. Налаштуйте доступ до спільного ресурсу SMB

Перед запуском контейнера потрібно переконатися, що спільний ресурс SMB доступний з хост-системи:

#### Варіант А: Монтування спільного ресурсу SMB на хості (Рекомендовано)

На вашій хост-системі змонтуйте спільний ресурс SMB:

```bash
# Створіть точку монтування
sudo mkdir -p /mnt/salem

# Змонтуйте спільний ресурс SMB (замініть своїми обліковими даними)
sudo mount -t cifs //salem/Documents /mnt/salem -o username=your_username,password=your_password,uid=$(id -u),gid=$(id -g)
```

Для постійного монтування додайте до `/etc/fstab`:
```
//salem/Documents /mnt/salem cifs username=your_username,password=your_password,uid=1000,gid=1000,file_mode=0644,dir_mode=0755 0 0
```

#### Варіант Б: Використання GVFS (якщо доступно)

Якщо ваша система використовує GVFS (поширено в середовищах робочого столу Ubuntu/Mint), спільний ресурс SMB може бути доступний за адресою:
```
/run/user/$(id -u)/gvfs/smb-share:server=salem,share=documents
```

### 3. Налаштуйте додаток

Скопіюйте приклад конфігурації та відредагуйте шляхи за потреби:

```bash
cp config.example.toml config.toml
```

Відредагуйте `config.toml`, щоб відповідав шляхам до вашого спільного ресурсу SMB:

```toml
[paths]
# Оновіть ці шляхи, щоб відповідали місцю монтування вашого спільного ресурсу SMB
remote_folder_path = "/mnt/salem/Накази"  # Або "/run/user/1000/gvfs/smb-share:server=salem,share=documents/Накази"
photo_folder_path = "/mnt/salem/ФОТО ВК"  # Або "/run/user/1000/gvfs/smb-share:server=salem,share=documents/ФОТО ВК"
local_cache_path = "./nakazi_cache"
documents_index_path = "documents_index.json"
inverted_index_path = "inverted_index.json"
```

### 4. Оновіть docker-compose.yml

Змініть файл `docker-compose.yml`, щоб змонтувати ваш спільний ресурс SMB. Оновіть розділ томів:

```yaml
services:
  blazing-search:
    build: .
    ports:
      - "8080:8080"
    volumes:
      # Змонтуйте ваш спільний ресурс SMB - відрегулюйте шлях за потреби
      - /mnt/salem:/mnt/salem:ro  # Або - /run/user/1000/gvfs/smb-share:server=salem,share=documents:/mnt/salem:ro
      - ./nakazi_cache:/app/nakazi_cache
      - ./documents_index.json:/app/documents_index.json
      - ./inverted_index.json:/app/inverted_index.json
      - ./config.toml:/app/config.toml
    environment:
      - RUST_BACKTRACE=1
    restart: unless-stopped
```

### 5. Збирайте та запускайте контейнер

```bash
# Зберіть та запустіть контейнер
docker-compose up --build -d

# Перевірте журнали, щоб переконатися, що все працює належним чином
docker-compose logs -f
```

### 6. Доступ до додатку

Після запуску контейнера отримайте доступ до додатку за адресою:
- Веб-інтерфейс: http://localhost:8080

## Альтернатива: Запуск без збирання

Якщо ви надаєте перевагу витягуванню готового образу (коли доступно):

```bash
# Витягніть та запустіть образ безпосередньо
docker run -d \
  --name blazing-search \
  -p 8080:8080 \
  -v /mnt/salem:/mnt/salem:ro \
  -v ./nakazi_cache:/app/nakazi_cache \
  -v ./config.toml:/app/config.toml \
  your-dockerhub-username/blazing-search:latest
```

## Усунення несправностей

### Загальні проблеми:

1. **Відмовлено у доступі до спільного ресурсу SMB**:
   - Переконайтеся, що спільний ресурс SMB правильно змонтований на хості
   - Перевірте, чи може контейнер Docker отримати доступ до змонтованого шляху

2. **Додаток не може знайти файли**:
   - Переконайтеся, що шляхи в `config.toml` відповідають шляхам усередині контейнера
   - Перевірте, чи правильно налаштовано монтування томів у `docker-compose.yml`

3. **Контейнер не вдається запустити**:
   - Виконайте `docker-compose logs`, щоб переглянути повідомлення про помилки
   - Переконайтеся, що всі необхідні томи доступні

### Корисні команди:

```bash
# Перегляньте журнали контейнера
docker-compose logs -f

# Перезапустіть сервіс
docker-compose restart

# Зупиніть сервіс
docker-compose down

# Увійдіть у контейнер для налагодження
docker-compose exec blazing-search bash
```

## Оновлення додатку

Щоб оновити до останньої версії:

```bash
# Витягніть останній код
git pull origin main

# Перезберіть та перезапустіть контейнер
docker-compose up --build -d
```

## Примітки

- Перший запуск триватиме довше, оскільки індекс будується з нуля
- Додаток зберігає кешовані файли в каталозі `nakazi_cache`
- Файли індексу (`documents_index.json` та `inverted_index.json`) зберігаються в корені проєкту
- Додаток автоматично оновлює індекс періодично