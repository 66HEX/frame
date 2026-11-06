# Audyt nawigacji klawiaturą — 2026-11-05

Audyt obejmował natywne okno macOS oraz testy renderowanego UI GPUI. Do obsługi
okna używano Tab, Shift+Tab, strzałek, Home/End, Enter, Space i Escape. Scenariusze
uruchamiano w osobnej aplikacji testowej z danymi fixtures.

## Znalezione i poprawione problemy

| Miejsce | Przyczyna | Poprawka |
| --- | --- | --- |
| Inspector: tab → zawartość | Element grupy wyboru miał `tab_stop`, lecz jego jawny `FocusHandle` go nie miał. | Aktywna, dostępna opcja ustawia tab stop również na uchwycie. Grupa ma jeden punkt wejścia, opcje zmienia się strzałkami. |
| Inspector: zbędny przystanek na panelu | Kontener panelu był osobnym punktem w kolejności Tab. | Panel nie przyjmuje focusu; Tab przechodzi z aktywnej zakładki bezpośrednio do pierwszej kontrolki, a Shift+Tab wraca do zakładki. Panel bez kontrolek jest pomijany. Up/Down i PageUp/PageDown przewijają treść przy focusie na zakładce. |
| Długie panele i listy | Tab przenosił focus poza obszar przewijania bez odsłonięcia kontrolki. | GPUI odsłania nowy focus klawiaturowy w kolejnej klatce. Uwzględnia zagnieżdżone obszary, nie przewija sąsiednich paneli i nie odwraca ręcznego przewinięcia. |
| Kolejka plików | Brak klawiaturowego wyboru pliku niezależnego od checkboxa konwersji. | Up/Down/Home/End wybierają plik i przewijają listę. Space zmienia zaznaczenie do konwersji. Bieżący wiersz ma widoczny ring. |
| Zmiany akcji kolejki | Pause/Resume i usuwanie wiersza pozostawiały focus przy poprzedniej kontrolce. | Stałe uchwyty dla miejsc akcji. Zniknięcie akcji przenosi focus do następnej akcji lub listy; usunięcie ostatniego pliku do Open File. |
| Settings | Escape oddawał focus korzeniowi aplikacji. | Powrót do Settings; na pustym ekranie do Open File. |
| Okno aktualizacji | Wsteczne zawijanie mogło wychodzić poza dialog; release notes nie dało się przewijać klawiaturą. | Tab i Shift+Tab pozostają w dialogu. Treść ma własny ring i obsługę klawiszy przewijania. |
| Crop i overlay | Niemodalny toolbar zapętlał Tab i przejmował focus od uchwytów. Escape nie działał we wszystkich kontrolkach. | Focus początkowy tylko przy otwarciu; swobodne przejście między paskiem i uchwytami; Escape wraca do narzędzia. |
| Logs | Ring treści był przycinany; dalekie zakładki wychodziły poza widok. | Miejsce na ring oraz przewijanie paska do aktywnej zakładki. |

## Zakres kontroli

W natywnym oknie sprawdzono toolbar aplikacji, otwieranie i anulowanie systemowego
wyboru pliku/folderu, Settings i jego selecty, canvas, narzędzia preview, crop,
nakładkę, timeline, kolejkę, zakładki inspectora, grupy wyboru, pola tekstowe,
slidery, select fontu, color picker, presety, Logs oraz dialog aktualizacji.

Automatyczny test przechodzi pełny cykl Tab i dokładnie odwrotny cykl Shift+Tab
dla 17 widoków przy skali UI 100% i 200%: Source, Output, Video, Video AV1,
Video Filters, Audio, Audio Filters, Subtitles, Metadata, Presets, Images,
Settings, Logs, aktualizacja, crop, custom crop i pusty workspace. Sprawdza
nazwy, dostępność, widoczne granice focusu i obecność kluczowych kontrolek.

Dodatkowe regresje obejmują kolejkę 500 plików, zmianę statusu zadania, usunięcie
ostatniego pliku, 30 zakładek logów, przewijanie release notes, powrót z Settings
oraz edycję uchwytów crop i overlay. Test GPUI sprawdza zagnieżdżone przewijanie,
ponowne wejście focusem oraz rozróżnienie klawiatury i myszy.

Końcowa weryfikacja: 997 testów workspace przeszło, 50 pozostaje oznaczonych
`ignored`; 420 testów biblioteki GPUI przeszło. Build, Clippy z `-D warnings`
i kontrola formatowania zakończyły się poprawnie. Pierwszy przebieg workspace
miał timeouty lokalnego serwera w czterech testach updatera; osobne uruchomienie
updatera i ponowny pełny przebieg przeszły bez zmian w jego kodzie.

## Odtworzenie kontroli

```sh
CARGO_INCREMENTAL=0 cargo test --locked -p frame-app keyboard_tests
CARGO_INCREMENTAL=0 cargo test --locked --workspace
CARGO_INCREMENTAL=0 cargo clippy --locked --workspace --all-targets -- -D warnings
cd vendor/gpui-ce
CARGO_INCREMENTAL=0 cargo test --locked -p gpui-ce --lib
```

To audyt nawigacji klawiaturą. Nie obejmuje odsłuchu VoiceOver, natywnego działania
na Windows/Linux ani pobierania i instalowania aktualizacji. Stany konwersji
sprawdzano na kontrolowanych danych bez uruchamiania eksportu plików użytkownika.

## Inspector po przebudowie — 2026-11-06

Nawigacja ma trzy stałe zakładki: Metadata, Output i Presets. Testy dawnych
zakładek Video, Audio, Images, filtrów i napisów otwierają odpowiadające im
sekcje Output. Stan początkowy aplikacji pozostawia wszystkie sekcje zamknięte.

Nowe testy obejmują Tab i Shift+Tab między zakładką, nagłówkami Collapsible
i ich zawartością, zamykanie i ponowne otwieranie sekcji, Custom resolution,
automatyczną aktywację Tabs, selekcję i powrót fokusu w nowych selectach oraz
ich animacje wejścia i wyjścia. Test przechodzenia przez 17 widoków zachowuje
kontrolę obu kierunków przy skalach 100/200%.

W osobnym natywnym oknie macOS sprawdzono General, zmianę Processing mode,
wybór kontenera, powrót Shift+Tab do zakładek, Presets, Video/Custom resolution,
ponowne rozwijanie i wejście do Width oraz Subtitles/Burn-in. Panel nie dodaje
własnego przystanku. Kliknięcie nagłówka przejmuje fokus przed odmontowaniem
jego zawartości.

`cargo xtask ci`: 1003 zaliczone, 50 dotychczasowych testów ignored; formatowanie,
Clippy i pozostałe kontrole repozytorium przeszły. Referencyjna galeria React
z Collapsible przechodzi `npm run build`.
