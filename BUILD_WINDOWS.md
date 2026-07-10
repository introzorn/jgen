# Сборка jGenesis на Windows (с поддержкой `archive.7z#rom.gen`)

## Что есть на этой машине (проверено)

| Компонент | Статус |
|-----------|--------|
| **Visual Studio Professional 2022** | ✅ `D:\Program Files\Microsoft Visual Studio\2022\Professional` |
| **MSVC (компилятор C++)** | ✅ `VC\Tools\MSVC\14.38.33130` |
| **Rust / Cargo** | ❌ не установлен |
| **Docker** | ❌ |
| **WSL** | ❌ |
| **Git** | ✅ 2.34.1 |

Отдельного «Build Tools» нет — но **полный VS Pro с C++ уже стоит**. Для локальной сборки не хватает только Rust.

---

## Сборка БЕЗ установки чего-либо локально (рекомендуется)

Проект уже умеет собирать Windows `.exe` **на GitHub Actions** (кросс-компиляция Linux → Windows). Добавлен ручной workflow:

**`.github/workflows/build-windows-cli-manual.yml`**

### Шаги

1. Создать репозиторий на GitHub (приватный или публичный).
2. Залить папку `JGENESIS_SOURCE` (с патчем `#`):
   ```powershell
   cd D:\EMULATORS\SEGA\JGENESIS_SOURCE
   git init
   git add .
   git commit -m "jGenesis with archive#rom CLI support"
   git remote add origin https://github.com/ВАШ_АККАУНТ/jgenesis-patched.git
   git push -u origin master
   ```
3. На GitHub: **Actions** → **Build Windows CLI (manual)** → **Run workflow**.
4. После завершения (~15–30 мин) скачать artifact **jgenesis-cli-windows-patched** (zip с `jgenesis-cli.exe`, `SDL3.dll`, `dxcompiler.dll`).
5. Распаковать в `D:\EMULATORS\SEGA\` или в `GGCenter\Consoles\Sega\`.

Ничего ставить на ПК не нужно — только git (уже есть) и аккаунт GitHub.

### Альтернатива без своего репозитория

Попросить кого-то с GitHub Actions собрать из вашего архива, или использовать [GitHub Codespaces](https://github.com/codespaces) (бесплатные минуты) — там Rust уже есть, но для Windows `.exe` всё равно удобнее Actions workflow выше.

---

## Синтаксис запуска ROM из архива

```powershell
jgenesis-cli.exe -f "E:\GAMES\Sonic.7z#Sonic the Hedgehog (U) [!].gen"
jgenesis-cli.exe -f "E:\GAMES\Sonic.7z"   # без # — первый ROM в архиве (как раньше)
```

Разделитель `#` (как в RetroArch): слева путь к `.zip`/`.7z`, справа имя файла внутри архива.

---

## Можно ли собрать без Cargo?

**Нет** — jGenesis написан на Rust. Cargo — это стандартный менеджер сборки Rust-проектов, без него из исходников не собрать.

### Варианты

| Способ | Нужен Cargo? | Комментарий |
|--------|--------------|-------------|
| **Скачать готовый релиз** | Нет | [github.com/jsgroth/jgenesis/releases](https://github.com/jsgroth/jgenesis/releases) — но там **нет** патча `#`, только после своей сборки |
| **Установить Rust + собрать** | Да (один раз) | Рекомендуется для этого форка |
| **Попросить кого-то собрать** | Нет у вас | Получить готовый `jgenesis-cli.exe` |

Cargo ставится вместе с Rust через [rustup](https://rustup.rs/) — это один установщик, не отдельная «тяжёлая» IDE.

---

## Установка Rust (один раз)

1. Скачать и запустить: https://rustup.rs/ → `rustup-init.exe`
2. Выбрать **default host triple** (x86_64-pc-windows-msvc)
3. Нужны **Visual Studio Build Tools** с компонентом **Desktop development with C++** (для SDL3 и нативных зависимостей)

Проверка:

```powershell
rustc --version
cargo --version
```

---

## Сборка jgenesis-cli

```powershell
cd D:\EMULATORS\SEGA\JGENESIS_SOURCE

# Первая сборка долгая (SDL3 компилируется из исходников)
cargo build --profile release-lto -p jgenesis-cli
```

Готовый exe:

```
D:\EMULATORS\SEGA\JGENESIS_SOURCE\target\release-lto\jgenesis-cli.exe
```

Быстрая отладочная сборка (быстрее, но медленнее в рантайме):

```powershell
cargo build -p jgenesis-cli
# → target\debug\jgenesis-cli.exe
```

---

## Запуск из PowerShell

Кавычки обязательны из‑за пробелов и `!` в именах GoodSet:

```powershell
.\target\release-lto\jgenesis-cli.exe -f "E:\EMULATORS\GAMES\SEGA\Sonic.7z#Sonic (U) [!].gen" --fullscreen
```

---

## Зависимости при запуске

- **SDL3** — при сборке с дефолтными фичами кладётся рядом автоматически, либо должна быть в PATH
- **dxcompiler.dll** — только если используется DX12 backend wgpu; положить в папку с exe

---

## Тесты парсера пути (без полной сборки эмулятора)

```powershell
cargo test -p jgenesis-native-driver parse_archive_path
```

---

## Интеграция с GGCenter

```vb
' Пример аргументов для SegaConsole
start_info.Arguments = "-f """ & game.Path & "#" & game.Versions.First() & """ --fullscreen"
```
