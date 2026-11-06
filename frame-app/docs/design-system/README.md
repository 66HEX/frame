# Graphite we Frame

Frame używa motywu Graphite inspirowanego powierzchniami Sebastiano Guerriero: neutralnej, lekko chłodnej grafitowej palety, warstwowych cieni z highlightem i zagłębionych pól. Układ, odstępy, typografia, wymiary kontrolek i obsługa pozostają oparte na natywnym adapterze shadcn/ui Mira. Źródłem są pliki wygenerowane przez CLI `shadcn@4.23.0`, preset `b1D0eCA4`, baza Radix, kolor Neutral i Inter. Aplikacja używa ikon Nucleo UI 1.9.0, outline 18 px / stroke 1.5. To świadome odstępstwo od ikon oryginalnego presetu. Oryginały CLI znajdują się w [reference](reference).

Repozytorium React znajduje się obok Frame: `../frame-shadcn-mira`. Zawiera działającą galerię light/dark i komponenty zainstalowane przez CLI. Nie jest zależnością wykonawczą aplikacji Rust.

## Wygląd powierzchni

Referencje: [Lab01, eksperyment 3](https://lab01.dev/#ui-experiment-3), obrazy dostarczone do zadania i podana skala cieni 2xs–2xl. Dostęp do pliku Figma przez konektor był niedostępny; nie stanowił źródła odczytanych tokenów.

CSS definiuje paletę light/dark oraz bazowy promień 14 px. [Skala cieni](../ui-shadows/README.md) opisuje natywne warstwy: karty Sm, przyciski Xs, selecty 2xs, popovery Md, dialogi Xl. Pola tekstowe i tło tabs są zagłębione. Badge’e, nieaktywne ghost buttons, separatory i zwykłe wiersze pozostają płaskie. W dark mode hover i rozwinięte nagłówki Collapsible używają jasnej warstwy 8%, aby odróżniały się od rozświetlonej karty. Dekoracyjne obwódki zastępują cienie; istniejące szerokości przezroczystych borderów zachowują geometrię treści. Ringi fokusu i obrysy błędów zachowują swoją funkcję.

Promienie są współśrodkowe przy zachowaniu istniejącego paddingu: kontrolka 10 px, toolbar/menu 14 px przy 4 px insetu, tabs 13 px przy 3 px insetu. Statyczne powierzchnie mają subtelne światło od góry. Animowane tła pozostają jednolite, ponieważ GPUI interpoluje tylko jednolite kolory; relief tych kontrolek tworzą inset shadows.

## Zmiana motywu przez kopiuj–wklej

Edytuj wyłącznie [assets/themes/theme.css](../../assets/themes/theme.css). Zastąp bloki `:root { ... }` i `.dark { ... }` blokami z edytora motywów shadcn, zapisz plik i uruchom `cargo run -p frame-app`. Cargo automatycznie przeliczy kolory i przebuduje aplikację. Nie trzeba uruchamiać Node ani eksportować JSON.

Importer przyjmuje OKLCH, HSL, RGB, hex, kolory nazwane, alfa, aliasy `var(...)` i starszy zapis HSL bez funkcji. Można wkleić cały arkusz zawierający te bloki, również wewnątrz `@layer base`. `.dark` dziedziczy nieprzepisane wartości z `:root`. `--radius` przyjmuje `rem`, `px` lub `0`; promienie pochodne odejmują istniejący inset od bazowego promienia, z minimum 0 px. Błędny wymagany token zatrzymuje build z nazwą zmiennej zamiast wracać do Neutral.

To importer tokenów kolorów i promienia, nie silnik przeglądarkowy: selektory komponentów, fonty zewnętrzne, breakpointy i dowolne reguły CSS nie zmieniają natywnych komponentów. Promień musi być wspólny dla light/dark. [reference/mira-neutral.json](reference/mira-neutral.json) jest tylko niezmiennym wzorcem testowym, nie konfiguracją aplikacji.

## Timingi komponentów

Timingi natywnego adaptera, oparte na źródłach CLI i porównaniu z działającą galerią:

| Komponent | Czas i krzywa | Właściwości |
| --- | --- | --- |
| Select / Popover / color picker | 100 ms, CSS `ease` = cubic-bezier(.25,.1,.25,1) | opacity, skala .95–1; wejście od strony triggera o 8 px, wyjście w jego kierunku; strona i środek skalowania zależą od dostępnego miejsca |
| Dialog | 100 ms, CSS `ease` | opacity i skala .95–1, środek panelu |
| Tooltip | zwłoka hover 400 ms; wejście i wyjście 150 ms, CSS `ease` | opacity, skala .95–1 i przesunięcie 8 px od strony triggera; fokus klawiatury bez zwłoki |
| Sheet | 200 ms, cubic-bezier(.4,0,.2,1) | opacity i przesunięcie o 10% szerokości w obie strony |
| Overlay Dialog/Sheet | 100 ms, CSS `ease` | opacity; blur 4 px jest stały |
| Slider | 150 ms, cubic-bezier(.4,0,.2,1) | ring 0–2 px, kolor ring/30, hover/focus-visible/active; kursor domyślny |
| Button / Toggle / Tabs | 150 ms, cubic-bezier(.4,0,.2,1) | przejścia kolorów i ringu; przycisk dodatkowo przesunięcie nacisku 1 px |
| Input / Select trigger | 150 ms, cubic-bezier(.4,0,.2,1) | kolory; ring pojawia się bez animacji szerokości, zgodnie z `transition-colors` |
| Item / wiersz presetu lub ścieżki | 100 ms, cubic-bezier(.4,0,.2,1) | kolory; ring natychmiastowy (`transition-colors duration-100`) |
| Checkbox | 150 ms, cubic-bezier(.4,0,.2,1) | ring (`transition-shadow`); zaznaczenie bez fade |
| Progress | 150 ms, cubic-bezier(.4,0,.2,1) | zmiana długości wypełnienia |
| Spinner | 1 s, linear, powtarzanie | obrót |

Tooltip ma celowo dłuższą zwłokę niż oryginalny preset Mira. Szybkie przechodzenie między tooltipami zachowuje okno 300 ms bez ponownego oczekiwania; animacja wejścia działa także wtedy. Opuszczenie kontrolki przed upływem zwłoki anuluje oczekujący tooltip. Zamykanie zachowuje element do końca animacji. Ustawienie systemowe reduced motion natychmiast kończy przejścia, zachowując zwłokę hover. Natywna transformacja skaluje także glyphy, obrysy, cienie i obszary trafienia, bez zmiany układu tekstu.

### Focus i stany kontrolek

[Audyt klawiatury](KEYBOARD_AUDIT.md) opisuje poprawki przechodzenia Tab/Shift+Tab,
odsłaniania focusu, powrotów z dialogów i nawigacji w kolejce oraz preview.

Pomiar `getComputedStyle` w referencji CLI potwierdza różne ringi w samym Mira:
Button, Input, Select i Checkbox: **2 px, ring/30**; Toggle, Item i Tabs:
**3 px, ring/50**. Tabs ma dodatkowo **outline 1 px** przy offset 0.
To wartości CSS px, niezależne od skali `rem`; na Retina ring 2 px ma 4 piksele urządzenia.

Button, Toggle, Tabs i Select używają końcowych stylów interakcji GPUI. Nie nakładamy
na nie drugiej animacji hover obliczanej przy renderowaniu. Zmiana zaznaczenia, hover,
focus i przerwanie ruchu trafiają do tego samego mechanizmu przejść. Nacisk Button
przesuwa go o stały 1 px; Toggle nie ma tego przesunięcia. Wszystkie narzędzia preview korzystają z Button ghost. Flip, crop i overlay
zachowują stan `aria-pressed`, lecz nie zmieniają rodziny wizualnej ani ringu.

`control_motion_tests.rs` sprawdza scenę malowaną przez rzeczywiste konstruktory:
ringi co 15 ms, oba motywy, skale UI 100/200%, utratę focusu, hover przerwany
naciśnięciem i wyjazdem kursora, zmianę zaznaczenia i reduced motion.
Ring zachowuje ułamkową szerokość i geometrię podczas przejścia, bez zaokrąglania
do pikseli urządzenia. Obrys tabów jest współśrodkowy z ringiem i animuje kolor.
Test GPU `focus_rings` sprawdza również piksele 61 kolejnych klatek.

Wspólna interpolacja kolorów uwzględnia alfa przed mieszaniem kanałów, zgodnie
z [CSS Color 4](https://www.w3.org/TR/css-color-4/#interpolation-alpha).
Przejście białego tła do przezroczystości wygasza biel bez szarego błysku.
Test inspectora sprawdza malowane tła co 8 ms, zmianę tabów myszą i klawiaturą
oraz odwrócenie przejścia w trakcie animacji, w obu motywach i skalach UI.

## Inwentaryzacja

[INVENTORY.md](INVENTORY.md) zawiera wszystkie 24 rodzaje pól tekstowych, konstruktory UI, ich miejsca użycia i jawne identyfikatory. [inventory.json](inventory.json) zawiera ten sam indeks do dalszej analizy. Polecenie `python3 script/inventory-ui.py` aktualizuje oba pliki.

| Rodzina | Elementy we Frame | Wzorzec / adapter |
| --- | --- | --- |
| Button | Add source, Start, Cancel, Settings, aktualizacje, Apply, Close, Copy logs, reset filtrów, zapis/usuwanie presetów, import plików i napisów, akcje kolejki, narzędzia podglądu | `button.rs`, `frame_button_base`; default/primary, secondary, outline, ghost, destructive, link; xs/sm/default/lg i warianty ikonowe |
| Input | 24 typy: nazwy, bitrate, rozmiary, timecode, metadane, presety, opisy napisów, hex, liczba zadań | `input/element.rs`, `FrameTextInputSpec`; normal, focus, placeholder, disabled, invalid |
| Field / Label | Pola formularzy, podpisy, opisy, grupy i nagłówki sekcji we wszystkich kartach inspektora i ustawieniach | `patterns.rs`: `frame_field`, `frame_field_group`, `frame_label`, `frame_description`, `frame_section` |
| Checkbox | Zaznaczenie plików, select all / mixed, włączanie filtrów, opcje konwersji i napisów | `checkbox.rs`; checked, unchecked, mixed, disabled, focus |
| Collapsible | Sekcje Output | `frame_collapsible`; pełna szerokość nagłówka, `aria-expanded`, Enter/Space, domyślnie zamknięte, odmontowana treść |
| Select | Kontener, kodeki, dłuższe listy parametrów Output, Theme, UI scale, font napisów, rozmiar fontu, aspect ratio i jednostki crop | Jeden `frame_select_control`, wspólne trigger/item/popover, przewijanie, klawiatura, typeahead, odtwarzanie fokusu |
| Slider | CRF, quality, image quality/compression; 10 filtrów wideo; 9 filtrów audio; opacity nakładki | `slider.rs`, zakresy i etykiety w specyfikacjach domenowych; track 4, thumb 12 |
| Range | Trim start/end i bieżąca pozycja odtwarzania | `preview/timeline`; wspólny rail i przeliczanie współrzędnych, trzy wartości domenowe |
| Item | Wybór presetów, pliki w kolejce | `list_item.rs`; treść i akcje pozostają we współdzielonych konstruktorach wierszy |
| Tabs | Workspace/Logs, Metadata/Output/Presets, krótkie wybory i tryby Output, pliki logów | `frame_tabs_list`, `frame_tab_trigger`, `frame_option_tabs`, `TabFocusGroup`; aktywacja strzałkami, Home/End, jeden tab stop |
| Card | Preview, kolejka, inspektor, logi, welcome | `FrameSurface::card_surface`; card, radius-lg, shadow-sm |
| Badge | Wartości suwaków i kompaktowe wskaźniki wartości | `frame_badge`, wariant secondary, 20 px, tekst 10 px |
| Separator | Podziały nagłówków i toolbarów | Jeden piksel tokenu border, bez tłoczeń i dodatkowego highlightu |
| Tooltip | Przyciski narzędziowe, playback, zoom | `tooltip.rs`; delay 400 ms, popover/foreground, radius-md, strzałka, keyboard focus/Escape, kolizje z brzegiem okna i hoverable content |
| Popover | Selecty, color pickery, narzędzia crop/overlay | `popover_surface`, `frame_toolbar`; popover, shadow-md z krawędzią i highlightem |
| Dialog / AlertDialog | Dostępna aktualizacja, pobieranie i potwierdzenia instalacji | `updates/dialog.rs`; wspólna powierzchnia dialogu i kontrolki |
| Sheet | Ustawienia aplikacji | `preferences/view.rs`; prawy brzeg, 384 px maks., 75% wąskiego okna, przezroczysty border-left, shadow-lg, backdrop black/80 + blur4 |
| ScrollArea | Fonty, selecty, logi, treści z przewijaniem | `scrollbar.rs`; szerokość 10, thumb 7, minimalna długość 18; wariant `type="always"` |
| Progress / Spinner | Aktualizacja, konwersja, ładowanie | Wspólny rail 4 px i spinner Nucleo |
| Empty / Alert | Pusta kolejka, brak danych, błędy runtime i aktualizacji | `frame_empty`, opisy i semantyka Alert; kolory semantic foreground/destructive |
| ButtonGroup / InputGroup / Kbd | Grupy akcji, pola z przyciskiem Apply i pomoc klawiatury | Złożenia istniejących primitives; zainstalowane źródła referencyjne |

Crop, pan/zoom, overlay resize, HSV canvas, miniatury, timecode i strumień logów są komponentami domenowymi. Shadcn nie dostarcza odpowiadających im edytorów mediów. Ich obudowy i kontrolki korzystają ze wspólnych tokenów i primitives; logika przetwarzania mediów pozostaje we Frame.

## Tokeny i geometria

Każdy token kolorystyczny CSS light/dark jest dostępny pod oryginalną nazwą przez `ThemePalette::token`. Adapter konwertuje OKLCH do zmiennoprzecinkowego sRGB. Nie zaokrągla do 8-bitowego hex. Alfa `border` i `input` zostaje zachowana; modyfikatory typu `input/30` mnożą alfę koloru. Secondary hover jest obliczony w OKLCH, zgodnie z `color-mix` źródła.

| Wartość | Graphite przy rem=16 / scale=100% |
| --- | --- |
| spacing | 4 px; półkroki 2 px |
| radius | base 14; sm 6; md 10; lg 14; xl 18; 2xl 22; 3xl 26; 4xl 30 |
| Button | xs 20; sm 24; default 28; lg 32; icon = kwadrat odpowiedniej wysokości |
| Tekst kontrolek | Inter 12 px / 19.5 px, weight 400/500; xs/badge 10 px |
| Input / Select trigger | 28 px, padding-x 8, border 1 przezroczysty, radius 10 |
| Checkbox | 16 px, radius 4, check 14 |
| Slider / Progress | track 4 px; slider thumb 12 px, jasny, shadow-xs |
| Tabs | list 32 px, padding 3, radius13; trigger 25 px, px6, radius10 |
| Field / FieldGroup | gap8 / gap16 |
| Card / Popover / Dialog | padding16 / 10 / 16; radius14 / 14 / 18 |
| Focus | button/input/checkbox/select/slider 2 px ring/30; toggle/tab 3 px ring/50 |
| Disabled | opacity .5, wyłączona aktywacja i tab stop |

Skalowanie Frame mnoży wymiary rem przez wybraną skalę UI. Obrysy 1 px i focus rings są wymiarami CSS px. Podgląd mediów ma własną skalę i nie zmienia skali kontrolek.

Separatory pozostają płaskie. Wspólna [scale.css](../ui-shadows/scale.css) zawiera dokładne wartości Guerriero i jest sprawdzana testem adaptera cieni, łącznie z kolorem, alfą, kolejnością, spreadem i inset. Blur CSS jest dzielony przez 2 zgodnie z parametrem Gaussian sigma w GPUI.

## Wygładzanie tekstu

Na macOS Frame ustawia globalnie `TextRenderingMode::Antialiased`, odpowiednik
[klasy Tailwind `antialiased`](https://tailwindcss.com/docs/font-smoothing).
Antyaliasing pozostaje włączony, ale CoreGraphics nie pogrubia krawędzi glifów
zależnie od jasności tekstu. Ustawienie obejmuje także ręcznie malowane inputy,
tooltipy i popovery. Nie zmienia fontów Inter 400/500, rozmiarów, pozycji ani kolorów.

Windows i Linux zachowują `PlatformDefault`: właściwości CSS używane przez tę
klasę [działają tylko na macOS](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-smooth).
To zgodność mechanizmu wygładzania, nie gwarancja identycznych pikseli między
silnikami. GPUI i przeglądarki mogą inaczej pozycjonować i składać glify.

## Wspólne zachowanie

Kompaktowe listy opcji, ścieżek i presetów używają `frame_list_group` z odstępem 8 px. Padding i odstęp między treścią a akcjami należą do wspólnego wiersza. Dodawanie napisów w trybach Selectable i Burn-in używa jednego `settings_subtitle_file_button`, zawsze w wariancie Outline; tryb zmienia tylko podpis i wybór jednego lub wielu plików.

Selecty mają wspólny kod dla kliknięć, Enter/Space, strzałek bez zawijania listy, Home/End, Escape i wyszukiwania prefiksowego z timeoutem 1 s. Powtórzenie tej samej litery przechodzi do kolejnego dopasowania. Otwarty select blokuje Tab, a zamknięcie przywraca fokus triggera. Używana kompozycja odpowiada `SelectContent position="popper"` z limitem wysokości 192 px; domyślny Radix `item-aligned` jest odrębnym wariantem układu.

Podświetlenie opcji Select ma jedno źródło: fokus używa pary `accent` / `accent-foreground`, a checkmark dziedziczy kolor tekstu. Ruch kursora przenosi fokus na opcję, tak jak w Radix; osobny kolor hover nie nadpisuje tego stanu. Dzięki temu kolory `secondary` i `accent` mogą się różnić, a użycie strzałek przy nieruchomym kursorze pozostawia tylko jedną podświetloną opcję.

Scrollbar selecta jest absolutną nakładką: lista zachowuje padding 4 px z każdej strony, bez dodatkowej kolumny na scrollbar. Thumb jest przycinany do zaokrąglonego obrysu popovera również w skrajnych pozycjach przewijania.

Selecty i color pickery używają wspólnego `FloatingPopover`. Komponent mierzy trigger i zawartość przed pierwszym malowaniem, otwiera popup nad triggerem, jeśli pod nim brakuje miejsca, oraz ogranicza wysokość do większej dostępnej przestrzeni, gdy nie mieści się po żadnej stronie. Zawartość pozostaje przewijalna. Pozycja uwzględnia szerokość okna, zmianę jego rozmiaru i skalę UI. Odstęp od triggera wynosi 4 px; margines kolizji Select wynosi 10 px, a Popover 0 px. Zgodnie z wymaganiem Frame animacja zamykania również przesuwa popup ku triggerowi; to rozszerzenie klas wyjścia zapisanych w referencji CLI.

Przyciski dzielą obsługę hover/press i disabled. Zwykły przycisk przesuwa się o 1 px podczas nacisku; Toggle i trigger selecta nie mają tego przesunięcia. Suwaki zachowują domenowe zakresy, wartości i kroki; Home/End wybierają granice, PageUp zwiększa wartość, PageDown ją zmniejsza. Wartości są dostępne dla technologii asystujących.

Tooltip mierzy rzeczywistą treść przed pozycjonowaniem. Przy braku miejsca zmienia stronę, przesuwa się wzdłuż krawędzi okna i ustawia strzałkę względem triggera. Obszar przejścia kursora między przyciskiem a treścią odpowiada mechanizmowi hoverable content z Radix; kliknięcie i Escape zamykają dymek.

## Odtworzenie wzorca

W katalogu sąsiadującym z Frame:

```sh
npx shadcn@4.23.0 init --template vite --base radix --preset mira --name frame-shadcn-mira --no-monorepo --yes
cd frame-shadcn-mira
npx shadcn@4.23.0 add button input label field checkbox radio-group select slider toggle toggle-group tabs card badge separator tooltip popover dialog alert-dialog sheet scroll-area progress empty alert button-group input-group item spinner kbd collapsible --yes
npm install --save-dev culori@4.0.2
```

CLI doinstalował także `textarea` jako zależność. Łącznie snapshot zawiera 30 plików komponentów. `components.json`, lockfile, oryginały komponentów, CSS, licencja, SHA-256 i [provenance.json](reference/provenance.json) identyfikują źródło. Inter 4.1 jest osadzony jako instancje 400/500, opsz14.

Archiwalny snapshot CLI zachowuje oryginalne importy Hugeicons wyłącznie jako materiał porównawczy. Frame nie ładuje ich i nie ma już generatora tych assetów. Galeria React zachowuje poprawkę typów `spinner.tsx` (liczbowe `size` i `strokeWidth`); oryginał w snapshotcie pozostaje bez zmian.

## Weryfikacja i granice porównania

Graphite, 2026-11-06: `cargo xtask ci` zaliczył 1008 testów, przy 50 istniejących testach ignored, oraz formatowanie, Clippy, audyt kolorów i kontrolę workflow. Po końcowej korekcie neutralnej palety i stanów hover ponownie zaliczono 10 testów motywu, 6 testów rzeczywistych interakcji kontrolek i Clippy. Zbudowano aplikację oraz natywny renderer. Przejrzano świeże zrzuty głównych widoków w dark/light oraz selecty przy skali 200%. Porównanie układu wykonano z osobno skompilowanym HEAD `76d7ba4b`, a nie z wcześniej istniejącym plikiem wykonywalnym. Sprawdzenie diffu potwierdziło zachowanie layoutu, spacingu, fontów i ikon.

Testy obejmują komplet tokenów CSS i alf, geometrię cieni, fonty i ikony, disabled, fokus, klawiaturę, selecty ustawień aplikacji, napisów, crop i nowych pól Output, oba motywy, skale 100/200%, geometrię timeline i obsługę crop. Pełne sprawdzenie repo: `cargo xtask ci`. Galeria React: `npm run build`.

Pakiet poprawek 2026-11-05: `cargo xtask ci` — 982 testy zaliczone, 50 istniejących testów ignored; Clippy, formatowanie, audyt kolorów i workflow bez błędów. Osobny test GPUI sprawdza skalowanie i replay sceny; `cargo deny check licenses` również przechodzi. Testy obejmują animacje wszystkich selectów i pickerów, ring suwaka oraz przejście Tab/Shift+Tab przez panel inspectora. Sprawdzono natywny build z Nucleo.

Pierwotna migracja: 975 testów zaliczonych i 50 oznaczonych w repo jako ignored; brak błędów Clippy, formatowania, audytu kolorów i kontroli workflow. Build produkcyjny galerii również przeszedł. W desktopowym podglądzie przeglądarki potwierdzono dla Input wysokość 28, radius 8, padding 2×8 i Inter 12 / 19.5; dla SelectItem wysokość 28, radius 8 i padding 4×8.

Archiwalne podglądy Mira: [Frame dark](previews/frame-mira-dark.png), [galeria dark](previews/react-mira-dark.png), [galeria light](previews/react-mira-light.png). Są zapisem wybranych widoków, nie automatycznym porównaniem wszystkich stanów.

To port między DOM/Radix a GPUI. Zgodność tokenów i parametrów nie stanowi dowodu identyczności każdego piksela: rasteryzacja fontów, compositing, antyaliasing i backend dostępności zależą od silnika oraz systemu. Snapshot i testy umożliwiają sprawdzanie konkretnych stanów; nie deklarują automatycznie pełnej zgodności wizualnej wszystkich ekranów. Niestandardowe media widgets oraz wybrane przez Frame szerokości paneli, zakresy danych i kompozycje nie są komponentami dostarczanymi przez shadcn.


## Inspector: Metadata, Output, Presets

Metadata pokazuje informacje źródłowe z dawnej zakładki Source. Edycja tagów należy do sekcji Metadata w Output. Presets zachowuje bibliotekę presetów. Wszystkie trzy zakładki mają ikonę i tekst, bez tooltipa; tylko wybrana jest przystankiem Tab.

Output składa się z niezależnych, domyślnie zamkniętych Collapsible: General, Video / Images, Video filters, Audio, Audio filters, Subtitles i Metadata. Widoczność zależy od źródła, kontenera i trybu przetwarzania. Nagłówki oddzielają separatory na pełną szerokość. Cały prostokątny wiersz nagłówka reaguje na hover i zachowuje ten sam kolor po rozwinięciu. Ring klawiatury jest rysowany wewnątrz wiersza, aby nie obcinała go krawędź panelu. Treść ma równy padding `CARD_PADDING` (16 px) z każdej strony; boczny odstęp odpowiada Metadata i Presets. Zamknięta zawartość jest odmontowana. Nagłówki są przyciskami z `aria-expanded`, uruchamianymi również Enterem i spacją.

`frame_option_field` wybiera Tabs dla maksymalnie trzech opcji (w tym Processing mode, Subtitle type, jakość/bitrate i tryb metadanych). Rozdzielczość używa Tabs także dla pięciu opcji, ponieważ Custom odsłania pola wymiarów. Kodeki i kontener zawsze pozostają selectami, niezależnie od liczby dostępnych opcji. Pozostałe dłuższe wybory korzystają z tego samego Select co reszta aplikacji. Tabs mają automatyczną aktywację strzałkami, Home/End i pojedynczy przystanek Tab. `settings_field`, `settings_fields_grid` i `settings_track_choice` współdzielą układ pól, par i wyboru ścieżek. Pola zachowują rozmiary Mira; gęstość wynika z parowania pól i progressive disclosure.

Collapsible dodano przez `npx shadcn@4.23.0 add collapsible --yes` w repozytorium referencyjnym. Kopia `reference/collapsible.tsx` jest niezmieniona. Primitive nie zawiera animacji wysokości; natywny odpowiednik także jej nie dodaje. Wygląd nagłówka jest kompozycją Mira Button ghost i Separator.
