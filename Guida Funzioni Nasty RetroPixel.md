# Guida Funzioni Nasty RetroPixel

Questa guida descrive in modo operativo tutte le funzioni principali del progetto `Nasty RetroPixel`, sia lato core, sia lato CLI, sia lato GUI/WASM.

Obiettivo del progetto:

- recuperare una griglia pixel leggibile da immagini AI o immagini degradate;
- ricostruire una palette piu' stabile e controllata;
- ridurre rumore, blending morbido e incoerenze geometriche;
- ottenere un output piu' vicino a vera pixel art utilizzabile.

## Regola Operativa

Questo documento va considerato una guida viva.

Quando viene aggiunta una nuova funzione:

- va aggiornata la GUI se la feature e' utente-facing;
- va mantenuta la parita' con la CLI quando la funzione esiste nel core;
- va aggiornato anche questo documento.

## Architettura

Il progetto e' composto da tre livelli:

- `src/main.rs`: core Rust condiviso.
- `src/bin/nasty-retropixel-cli.rs`: interfaccia CLI.
- `web/index.html` + `web/main.js`: GUI locale via WASM.

Flusso logico del core:

1. caricamento immagine;
2. eventuale prefilter;
3. quantizzazione per analisi profili;
4. stima pixel size / recovery della griglia;
5. resampling a celle o per pixel;
6. ricostruzione palette;
7. palette cleanup;
8. cleanup artefatti;
9. repair strutturale;
10. report debug.

## Stato GUI

La GUI copre oggi tutte le funzioni operative principali sviluppate nel core:

- processing singolo;
- processing batch;
- preset manuali;
- preset suggestion automatica;
- override pixel size;
- denoise;
- palette source;
- palette lock da immagine;
- palette cleanup;
- cell color mode;
- dithering;
- color space;
- cleanup artefatti;
- repair strutturale `off|basic|smart|ultra`;
- debug report JSON;
- debug overlay PNG;
- debug heatmap PNG;
- metriche di qualita' nel report debug;
- overlay griglia sull'input;
- preview palette;
- trim trasparenza;
- scala finale;
- confronto interattivo con slider e blink;
- confronto varianti con ranking automatico per qualita';
- download PNG e ZIP offline.

Nota attuale:

- CLI e GUI coprono entrambe debug JSON, overlay e heatmap;
- la CLI esporta su filesystem (`--debug-json`, `--debug-overlay`, `--debug-heatmap`, `--debug-dir`);
- la GUI esporta debug singolo via download e, in batch ZIP, include anche JSON + overlay + heatmap;
- il batch del core e della CLI e' ricorsivo e preserva la struttura delle sottocartelle in output/debug;
- la GUI batch preserva i path interni quando selezioni una cartella e scarichi lo ZIP.
- il confronto varianti della GUI usa la heatmap del core quando disponibile e ordina i risultati mettendo in evidenza la variante consigliata.

## Funzioni Core

### Grid Recovery

Il motore cerca di ricostruire il passo della griglia partendo dai profili orizzontali e verticali dell'immagine.

Componenti principali:

- stima automatica della dimensione pixel;
- fallback con autocorrelazione se i picchi sono deboli;
- walker per posizionare i cut;
- stabilizzazione incrociata tra asse X e asse Y;
- snapping uniforme quando una delle due direzioni risulta distorta.

Serve per correggere:

- pixel con spacing irregolare;
- drift di risoluzione;
- mismatch tra larghezza e altezza apparente delle celle.

### Prefilter / Denoise

Modalita':

- `off`
- `box3`

Descrizione:

- `box3` applica un box blur alpha-aware molto leggero prima dell'analisi;
- utile su output AI con rumore fine, anti-aliasing sporco o edge troppo tremolanti;
- va usato con cautela su sprite gia' molto puliti.

### Palette Source

Modalita':

- `pixels`
- `cells`

Descrizione:

- `pixels`: ricostruisce la palette ragionando piu' direttamente sui pixel quantizzati;
- `cells`: usa il contenuto delle celle ed e' in genere piu' stabile per output gia' orientati a sprite/tiles.

Scelta consigliata:

- `cells` come default per sprite e tileset;
- `pixels` se vuoi un comportamento piu' vicino al contenuto originale.

### Palette Lock

Permette di usare una immagine palette come vincolo cromatico.

Comportamento:

- la palette viene letta da un file immagine;
- i colori unici opachi vengono usati come palette forzata;
- l'output viene quantizzato verso quei colori;
- quando il lock e' attivo, alcuni cleanup palette vengono volutamente limitati per non rompere il vincolo.

Utile per:

- uniformare asset a una palette di gioco;
- imporre uno stile cromatico coerente;
- evitare che il rebuild palette generi colori indesiderati.

### Palette Cleanup

Modalita':

- `off`
- `basic`
- `strict`

Descrizione:

- unifica colori molto vicini tra loro;
- riduce doppioni quasi identici;
- compatta palette sporche generate da AI o da quantizzazione intermedia.

Scelta pratica:

- `basic`: piu' conservativo;
- `strict`: piu' aggressivo, utile su immagini molto sporche.

### Cell Color Mode

Modalita':

- `mean`
- `dominant`
- `medoid`

Descrizione:

- `mean`: media cromatica; puo' introdurre colori piu' morbidi;
- `dominant`: sceglie il colore piu' frequente della cella; spesso piu' netto;
- `medoid`: sceglie il colore reale piu' rappresentativo; di solito il migliore su input AI sporchi.

Scelta consigliata:

- `dominant` o `medoid` per pixel art piu' leggibile;
- `medoid` e' spesso la scelta piu' robusta sui casi difficili.

### Dithering

Modalita':

- `off`
- `fs`

Descrizione:

- `fs` usa Floyd-Steinberg a livello di celle;
- puo' aiutare a simulare transizioni con palette piccole;
- puo' anche introdurre texture non desiderata se vuoi un look molto pulito.

### Color Space

Modalita':

- `srgb`
- `linear`

Descrizione:

- influisce su come vengono confrontati e ricostruiti i colori;
- `linear` e' il default consigliato per quantizzazione piu' coerente;
- `srgb` puo' essere utile per un look piu' vicino al colore percepito originale.

### Cleanup Artefatti

Modalita':

- `off`
- `basic`

Descrizione:

- rimuove piccoli pixel incoerenti rispetto al vicinato;
- sostituisce outlier locali quando il contesto intorno e' chiaramente dominante;
- e' il primo stadio di pulizia spaziale dopo palette cleanup.

### Repair Strutturale

Modalita':

- `off`
- `basic`
- `smart`
- `ultra`

Descrizione:

- `basic`: corregge bridge semplici orizzontali/verticali;
- `smart`: chiude piccoli gap, ripara micro-buchi, rimuove micro-isole e consolida pattern diagonali;
- `ultra`: versione piu' aggressiva del repair, pensata per output AI molto degradati.

Quando usarli:

- `basic`: immagini quasi gia' corrette;
- `smart`: default generale consigliato;
- `ultra`: immagini AI molto sporche, outline tremolanti, checker diagonali e rumore strutturale.

## Preset Disponibili

### `ai-sprite`

Pensato per il caso generale di cleanup di sprite generati da AI.

Tende a:

- usare una palette abbastanza ricca;
- privilegiare `medoid`;
- applicare cleanup e repair abbastanza robusti.

### `strict-retro`

Pensato per un look piu' rigido e controllato.

Tende a:

- ridurre la palette;
- contenere rumore e blending;
- puntare a blocchi netti.

### `tileset-cleanup`

Pensato per tileset e texture grid-based.

Tende a:

- favorire coerenza tra celle;
- mantenere palette stabile;
- ridurre piccoli errori strutturali.

### `character-cleanup`

Pensato per sprite/personaggi.

Tende a:

- proteggere silhouette e leggibilita';
- usare impostazioni piu' adatte a contorni e volumi.

### `icon-cleanup`

Pensato per icone e asset UI.

Tende a:

- contenere la palette;
- aumentare chiarezza e contrasto.

### `ultra-cleanup`

Pensato per casi molto sporchi.

Tende a:

- usare prefilter;
- usare palette cleanup `strict`;
- usare `medoid`;
- attivare `repair ultra`.

### `auto`

Disponibile come workflow suggerito, non come preset statico puro.

Comportamento:

- analizza l'immagine;
- valuta densita', alpha, numero colori, edge density e dimensioni;
- propone il preset piu' adatto.

## Funzioni GUI

### Input e Workflow

La GUI consente:

- drag & drop;
- selezione file;
- modalita' batch multi-file;
- salvataggio automatico delle impostazioni in local storage.

### Preview

Include:

- anteprima input;
- anteprima output;
- confronto con slider;
- modalita' blink per controllare rapidamente le differenze;
- zoom;
- overlay griglia sull'immagine di input.

### Guida rapida in GUI

Nel drawer delle opzioni avanzate e' presente anche una guida rapida integrata con:

- workflow consigliati;
- spiegazione veloce di denoise, palette e repair;
- note su debug e output.

Serve come riferimento immediato durante l'uso senza aprire documentazione esterna.

Sono presenti anche tooltip contestuali sui controlli piu' importanti, in particolare:

- denoise;
- palette source;
- palette cleanup;
- colore per cella;
- color space;
- cleanup;
- repair.

La sezione preset in GUI usa anche card descrittive con:

- nome breve del preset;
- descrizione veloce del caso d'uso;
- evidenziazione del preset attivo.

Nel pannello di suggerimento preset sono presenti anche consigli automatici contestuali che combinano:

- preset suggerito;
- segnali letti dall'immagine come numero colori, edge density e trasparenza;
- suggerimenti pratici su repair, palette cleanup, denoise e trim.

La GUI mostra anche warning automatici nei casi potenzialmente problematici, per esempio:

- troppi colori in ingresso;
- griglia probabilmente instabile;
- contenuto opaco molto ridotto;
- uso di preset molto aggressivi come `Ultra`.

Nel pannello suggerimenti e' presente anche una difficolta' stimata dell'immagine:

- `facile`
- `media`
- `difficile`
- `molto difficile`

Questo punteggio viene derivato dai segnali letti sull'immagine e serve per capire in anticipo quanto il recovery potrebbe essere complesso.

La GUI mostra anche un esito previsto sintetico, per esempio:

- buona probabilita' di risultato pulito;
- risultato misto ma recuperabile;
- caso difficile con probabile bisogno di intervento manuale.

Questo aiuta a capire se il preset suggerito basta da solo oppure se conviene prepararsi a usare palette lock, cleanup piu' aggressivo o un confronto piu' attento con l'originale.

Nel pannello suggerimenti e' presente anche il pulsante `Applica Setup Consigliato`.

Questa funzione non applica solo il preset suggerito, ma usa la raccomandazione condivisa del core per impostare anche:

- denoise;
- palette source;
- palette cleanup;
- cell color;
- cleanup;
- repair;
- trim trasparenza.

Anche la CLI, quando usi `--preset auto`, applica lo stesso setup consigliato e stampa a terminale il riepilogo delle scelte automatiche.

Nella GUI il setup consigliato ha anche una conferma rapida con tre toggle:

- `Applica anche palette e colore cella`
- `Applica anche trim trasparenza`
- `Forza repair Ultra`

Questo permette di usare l'auto setup in modo piu' controllato senza dover entrare subito nei parametri avanzati.

Sono presenti anche tre profili rapidi sopra ai toggle:

- `Conservativo`
- `Bilanciato`
- `Aggressivo`

I profili impostano in un click la combinazione dei toggle rapidi. Se modifichi i toggle manualmente, la GUI passa automaticamente a stato `Personalizzato`.

La GUI include anche il pulsante `Confronta Varianti Rapide`, che genera tre anteprime guidate:

- `Bilanciato`
- `Aggressivo`
- `Ultra`

Ogni variante mostra una mini anteprima e un pulsante `Usa questa` per promuovere quel risultato al pannello principale senza dover rilanciare tutto manualmente.

Le stesse varianti vengono mostrate anche in una preview grande affiancata dentro la sezione anteprime, cosi' puoi confrontare meglio `Bilanciato`, `Aggressivo` e `Ultra` a dimensione piu' utile.

Nel confronto grande puoi anche attivare `Mostra diff visivo`, che sostituisce le anteprime con una heatmap delle differenze rispetto all'immagine di input per capire dove ogni variante interviene di piu'.

Ogni variante mostra anche uno score comparativo sintetico con tre letture automatiche:

- `Piu' fedele` o percentuale di fedelta'
- `Piu' pulita` o percentuale di pulizia
- `Piu' aggressiva` o percentuale di aggressivita'

Serve per capire piu' velocemente quale variante conserva meglio la struttura, quale semplifica di piu' e quale interviene in modo piu' forte.

La GUI mostra anche una `Scelta consigliata` finale, con motivazione sintetica e pulsante `Usa Consigliata`, per applicare subito la variante che offre il compromesso migliore tra fedelta', pulizia e controllo.

La CLI supporta anche `--recommend-variant` (solo file singolo) per stampare a terminale la variante consigliata con le metriche principali. La logica di raccomandazione e' nel core condiviso, quindi resta coerente tra GUI e CLI.

### Debug

La GUI espone:

- riepilogo pixel size stimato;
- dimensione griglia output;
- numero colori finali;
- modalita' effettive usate;
- download del report JSON;
- download dell'overlay PNG.

### Output

Funzioni disponibili:

- download PNG singolo;
- trim trasparenza;
- upscale nearest 1x / 2x / 4x / 8x;
- download batch singolo oppure ZIP offline.

## Funzioni CLI

La CLI e' utile per automazione, batch e pipeline locali.

Supporta:

- file singolo;
- directory input/output;
- preset manuali;
- preset `auto` su file singolo;
- palette lock;
- export debug JSON;
- export debug overlay;
- scelta della directory debug.

Opzioni principali:

- `--preset`
- `--pixel-size`
- `--denoise`
- `--palette-source`
- `--palette-lock`
- `--palette-cleanup`
- `--cell-color`
- `--dither`
- `--color-space`
- `--cleanup`
- `--repair`
- `--debug-json`
- `--debug-overlay`
- `--debug-dir`

## Workflow Consigliati

### Caso 1: Sprite AI standard

Usa:

- preset `ai-sprite`
- `repair smart`
- `cell-color medoid`
- `palette cleanup strict`

### Caso 2: Output AI molto sporco

Usa:

- preset `ultra-cleanup`
- `repair ultra`
- `palette lock` se hai una palette target

### Caso 3: Tileset

Usa:

- preset `tileset-cleanup`
- `palette-source cells`
- `repair smart`

### Caso 4: Icone o UI

Usa:

- preset `icon-cleanup`
- palette piccola;
- cleanup e repair non troppo aggressivi salvo casi estremi.

## Limitazioni Attuali

- il progetto non "inventa" vera pixel art da zero: ripulisce, riallinea e ricostruisce;
- input molto lontani dallo stile pixel art possono richiedere iterazioni manuali;
- la GUI batch oggi privilegia l'output finale piu' che l'export debug per-file;
- la qualita' finale dipende anche dalla coerenza del materiale sorgente.

## Manutenzione Documento

Quando aggiungiamo nuove funzioni bisogna aggiornare almeno queste sezioni:

- `Stato GUI`
- `Funzioni Core`
- `Preset Disponibili`
- `Funzioni GUI`
- `Funzioni CLI`
- `Workflow Consigliati`
