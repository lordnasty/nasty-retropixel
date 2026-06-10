import init, {
  get_preset_config,
  recommend_variant_from_metrics_json,
  suggest_setup,
  process_image,
  process_image_debug,
  process_image_debug_with_palette_image,
  process_image_with_palette_image,
} from "./pkg/nasty_retropixel.js";

const els = {
  dropZone: document.getElementById("dropZone"),
  dropTitle: document.querySelector("#dropZone .dropTitle"),
  dropSub: document.querySelector("#dropZone .dropSub"),
  fileInput: document.getElementById("fileInput"),
  kColors: document.getElementById("kColors"),
  pixelOverrideEnabled: document.getElementById("pixelOverrideEnabled"),
  pixelSize: document.getElementById("pixelSize"),
  zoom: document.getElementById("zoom"),
  processBtn: document.getElementById("processBtn"),
  downloadBtn: document.getElementById("downloadBtn"),
  downloadDebugBtn: document.getElementById("downloadDebugBtn"),
  downloadOverlayBtn: document.getElementById("downloadOverlayBtn"),
  downloadAllBtn: document.getElementById("downloadAllBtn"),
  status: document.getElementById("status"),
  presetSuggestion: document.getElementById("presetSuggestion"),
  presetDifficulty: document.getElementById("presetDifficulty"),
  presetForecast: document.getElementById("presetForecast"),
  presetWarnings: document.getElementById("presetWarnings"),
  presetAdvice: document.getElementById("presetAdvice"),
  presetSetupSummary: document.getElementById("presetSetupSummary"),
  setupProfileBar: document.getElementById("setupProfileBar"),
  setupProfileStatus: document.getElementById("setupProfileStatus"),
  setupQuickOptions: document.getElementById("setupQuickOptions"),
  setupApplyPalette: document.getElementById("setupApplyPalette"),
  setupApplyTrim: document.getElementById("setupApplyTrim"),
  setupForceUltra: document.getElementById("setupForceUltra"),
  applySuggestedSetupBtn: document.getElementById("applySuggestedSetupBtn"),
  applySuggestedPresetBtn: document.getElementById("applySuggestedPresetBtn"),
  runVariantCompareBtn: document.getElementById("runVariantCompareBtn"),
  variantCompareStatus: document.getElementById("variantCompareStatus"),
  variantRecommendation: document.getElementById("variantRecommendation"),
  variantRecommendationLabel: document.getElementById("variantRecommendationLabel"),
  variantRecommendationReason: document.getElementById("variantRecommendationReason"),
  applyRecommendedVariantBtn: document.getElementById("applyRecommendedVariantBtn"),
  variantComparePanel: document.getElementById("variantComparePanel"),
  variantCompareGrid: document.getElementById("variantCompareGrid"),
  variantPreviewPanel: document.getElementById("variantPreviewPanel"),
  variantPreviewStatus: document.getElementById("variantPreviewStatus"),
  variantPreviewGrid: document.getElementById("variantPreviewGrid"),
  variantShowDiff: document.getElementById("variantShowDiff"),
  debugPixelSize: document.getElementById("debugPixelSize"),
  debugGridSize: document.getElementById("debugGridSize"),
  debugPaletteCount: document.getElementById("debugPaletteCount"),
  debugModes: document.getElementById("debugModes"),
  inputPreview: document.getElementById("inputPreview"),
  outputPreview: document.getElementById("outputPreview"),
  compareFrame: document.getElementById("compareFrame"),
  compareOutput: document.getElementById("compareOutput"),
  compareInputWrap: document.getElementById("compareInputWrap"),
  compareInput: document.getElementById("compareInput"),
  compareSlider: document.getElementById("compareSlider"),
  compareLabel: document.getElementById("compareLabel"),
  compareLine: document.getElementById("compareLine"),
  compareHandle: document.getElementById("compareHandle"),
  compareReset: document.getElementById("compareReset"),
  compareBlink: document.getElementById("compareBlink"),
  openOptions: document.getElementById("openOptions"),
  closeOptions: document.getElementById("closeOptions"),
  presetStatus: document.getElementById("presetStatus"),
  drawer: document.getElementById("drawer"),
  drawerOverlay: document.getElementById("drawerOverlay"),
  batchMode: document.getElementById("batchMode"),
  denoiseMode: document.getElementById("denoiseMode"),
  paletteSource: document.getElementById("paletteSource"),
  paletteLockEnabled: document.getElementById("paletteLockEnabled"),
  paletteLockFile: document.getElementById("paletteLockFile"),
  paletteCleanupMode: document.getElementById("paletteCleanupMode"),
  cellColorMode: document.getElementById("cellColorMode"),
  ditherMode: document.getElementById("ditherMode"),
  colorSpace: document.getElementById("colorSpace"),
  cleanupMode: document.getElementById("cleanupMode"),
  repairMode: document.getElementById("repairMode"),
  trimTransparent: document.getElementById("trimTransparent"),
  scaleFactor: document.getElementById("scaleFactor"),
  showPalette: document.getElementById("showPalette"),
  zipStatus: document.getElementById("zipStatus"),
  batchList: document.getElementById("batchList"),
  paletteSwatches: document.getElementById("paletteSwatches"),
  showGrid: document.getElementById("showGrid"),
  inputGridOverlay: document.getElementById("inputGridOverlay"),
};

const REQUIRED_KEYS = [
  "dropZone",
  "dropTitle",
  "dropSub",
  "fileInput",
  "kColors",
  "pixelOverrideEnabled",
  "pixelSize",
  "zoom",
  "processBtn",
  "downloadBtn",
  "downloadDebugBtn",
  "downloadOverlayBtn",
  "downloadAllBtn",
  "status",
  "presetSuggestion",
  "presetDifficulty",
  "presetForecast",
  "presetWarnings",
  "presetAdvice",
  "presetSetupSummary",
  "setupProfileBar",
  "setupProfileStatus",
  "setupQuickOptions",
  "setupApplyPalette",
  "setupApplyTrim",
  "setupForceUltra",
  "applySuggestedSetupBtn",
  "applySuggestedPresetBtn",
  "runVariantCompareBtn",
  "variantCompareStatus",
  "variantRecommendation",
  "variantRecommendationLabel",
  "variantRecommendationReason",
  "applyRecommendedVariantBtn",
  "variantComparePanel",
  "variantCompareGrid",
  "variantPreviewPanel",
  "variantPreviewStatus",
  "variantPreviewGrid",
  "variantShowDiff",
  "debugPixelSize",
  "debugGridSize",
  "debugPaletteCount",
  "debugModes",
  "inputPreview",
  "outputPreview",
  "compareInputWrap",
  "compareInput",
  "compareOutput",
  "compareSlider",
  "compareLabel",
  "compareLine",
  "compareHandle",
  "compareReset",
  "compareBlink",
  "openOptions",
  "closeOptions",
  "presetStatus",
  "drawer",
  "drawerOverlay",
  "batchMode",
  "denoiseMode",
  "paletteSource",
  "paletteLockEnabled",
  "paletteLockFile",
  "paletteCleanupMode",
  "cellColorMode",
  "ditherMode",
  "colorSpace",
  "cleanupMode",
  "repairMode",
  "trimTransparent",
  "scaleFactor",
  "showPalette",
  "showGrid",
  "zipStatus",
  "batchList",
  "paletteSwatches",
  "inputGridOverlay",
];

const missingKeys = REQUIRED_KEYS.filter((k) => !els[k]);
if (missingKeys.length) {
  const msg = `UI non inizializzata: mancano elementi DOM (${missingKeys.join(", ")}). Ricarica la pagina (Ctrl+F5).`;
  console.error(msg);
  document.body.textContent = msg;
  throw new Error(msg);
}

let wasmReady = false;
let selectedFiles = [];
let inputUrl = null;
let outputUrl = null;
let debugUrl = null;
let overlayUrl = null;
let batchResults = [];
let lastDebug = null;
let compareDragging = false;
let compareBlinkActive = false;
let compareBlinkPrev = 50;
let activePresetKey = null;
let suggestedPresetKey = null;
let suggestedSetup = null;
let activeSetupProfile = "balanced";
let variantCompareResults = [];
let recommendedVariantKey = null;

const STORAGE_KEY = "nasty-retropixel-settings-v1";
const SETUP_PROFILES = {
  conservative: {
    label: "Conservativo",
    applyPalette: false,
    applyTrim: false,
    forceUltra: false,
  },
  balanced: {
    label: "Bilanciato",
    applyPalette: true,
    applyTrim: true,
    forceUltra: false,
  },
  aggressive: {
    label: "Aggressivo",
    applyPalette: true,
    applyTrim: true,
    forceUltra: true,
  },
};

function setStatus(text, isError = false) {
  els.status.textContent = text;
  els.status.classList.toggle("error", Boolean(isError));
}

function setProcessEnabled(enabled) {
  els.processBtn.disabled = !enabled;
}

function setLoading(isLoading) {
  els.processBtn.classList.toggle("loading", Boolean(isLoading));
}

function revokeUrl(url) {
  if (!url) return;
  URL.revokeObjectURL(url);
}

function revokeBatchUrls() {
  for (const r of batchResults) revokeUrl(r.url);
  batchResults = [];
}

function revokeVariantCompareUrls() {
  for (const r of variantCompareResults) {
    revokeUrl(r.url);
    revokeUrl(r.diffUrl);
  }
  variantCompareResults = [];
}

function setDownloadEnabled(enabled) {
  els.downloadBtn.classList.toggle("disabled", !enabled);
  if (!enabled) els.downloadBtn.removeAttribute("href");
}

function setDebugDownloadEnabled(enabled) {
  els.downloadDebugBtn.classList.toggle("disabled", !enabled);
  if (!enabled) els.downloadDebugBtn.removeAttribute("href");
}

function setOverlayDownloadEnabled(enabled) {
  els.downloadOverlayBtn.classList.toggle("disabled", !enabled);
  if (!enabled) els.downloadOverlayBtn.removeAttribute("href");
}

function setVariantCompareStatus(text, isError = false) {
  els.variantCompareStatus.textContent = text;
  els.variantCompareStatus.classList.toggle("hidden", !text);
  els.variantCompareStatus.classList.toggle("error", Boolean(isError));
}

function setDownloadAllEnabled(enabled) {
  els.downloadAllBtn.classList.toggle("disabled", !enabled);
  els.downloadAllBtn.disabled = !enabled;
}

function colorEntryToCss(entry) {
  if (!entry) return "transparent";
  if (typeof entry === "string") return entry;
  const a = Number(entry.a ?? 255) / 255;
  return `rgba(${entry.r ?? 0} ${entry.g ?? 0} ${entry.b ?? 0} / ${a.toFixed(3)})`;
}

function clearDebugSummary() {
  els.debugPixelSize.textContent = "-";
  els.debugGridSize.textContent = "-";
  els.debugPaletteCount.textContent = "-";
  els.debugModes.textContent = "Nessun report disponibile.";
}

function clearPresetSuggestion() {
  suggestedPresetKey = null;
  suggestedSetup = null;
  els.presetSuggestion.textContent = "Seleziona un'immagine per ottenere un preset consigliato.";
  els.presetDifficulty.textContent = "";
  els.presetDifficulty.className = "difficultyBadge hidden";
  els.presetForecast.textContent = "";
  els.presetForecast.className = "forecastBox hidden";
  els.presetWarnings.textContent = "";
  els.presetWarnings.classList.add("hidden");
  els.presetAdvice.textContent = "";
  els.presetAdvice.classList.add("hidden");
  els.presetSetupSummary.textContent = "";
  els.presetSetupSummary.classList.add("hidden");
  els.setupProfileBar.classList.add("hidden");
  els.setupProfileStatus.textContent = "";
  els.setupProfileStatus.classList.add("hidden");
  els.setupQuickOptions.classList.add("hidden");
  applySetupProfile("balanced");
  els.applySuggestedSetupBtn.disabled = true;
  els.applySuggestedPresetBtn.disabled = true;
  els.runVariantCompareBtn.disabled = true;
}

function describeQuickSetupChoices() {
  return {
    applyPalette: Boolean(els.setupApplyPalette.checked),
    applyTrim: Boolean(els.setupApplyTrim.checked),
    forceUltra: Boolean(els.setupForceUltra.checked),
  };
}

function detectSetupProfileName(opts) {
  for (const [key, profile] of Object.entries(SETUP_PROFILES)) {
    if (
      profile.applyPalette === Boolean(opts.applyPalette) &&
      profile.applyTrim === Boolean(opts.applyTrim) &&
      profile.forceUltra === Boolean(opts.forceUltra)
    ) {
      return key;
    }
  }
  return "custom";
}

function updateSetupProfileUi() {
  const profileName = detectSetupProfileName(describeQuickSetupChoices());
  activeSetupProfile = profileName;
  document.querySelectorAll(".setupProfileBtn").forEach((btn) => {
    const name = btn.getAttribute("data-setup-profile");
    btn.classList.toggle("is-active", Boolean(name) && name === profileName);
  });

  if (!suggestedSetup) {
    els.setupProfileStatus.textContent = "";
    els.setupProfileStatus.classList.add("hidden");
    return;
  }

  if (profileName === "custom") {
    els.setupProfileStatus.textContent = "Profilo rapido attivo: Personalizzato";
  } else {
    els.setupProfileStatus.textContent = `Profilo rapido attivo: ${SETUP_PROFILES[profileName].label}`;
  }
  els.setupProfileStatus.classList.remove("hidden");
}

function applySetupProfile(profileName) {
  const profile = SETUP_PROFILES[profileName] ?? SETUP_PROFILES.balanced;
  els.setupApplyPalette.checked = profile.applyPalette;
  els.setupApplyTrim.checked = profile.applyTrim;
  els.setupForceUltra.checked = profile.forceUltra;
  updateSetupProfileUi();
}

function updateSuggestedSetupSummary() {
  if (!suggestedSetup) {
    els.presetSetupSummary.textContent = "";
    els.presetSetupSummary.classList.add("hidden");
    els.setupProfileBar.classList.add("hidden");
    els.setupProfileStatus.textContent = "";
    els.setupProfileStatus.classList.add("hidden");
    els.setupQuickOptions.classList.add("hidden");
    return;
  }

  const opts = describeQuickSetupChoices();
  const parts = [
    `denoise ${String(suggestedSetup.recommended_prefilter_label ?? "off")}`,
    opts.applyPalette
      ? `palette ${String(suggestedSetup.recommended_palette_source_label ?? "cells")}`
      : "palette invariata",
    opts.applyPalette
      ? `palette-fix ${String(suggestedSetup.recommended_palette_cleanup_label ?? "basic")}`
      : "palette-fix invariato",
    opts.applyPalette
      ? `cella ${String(suggestedSetup.recommended_cell_color_label ?? "dominant")}`
      : "cella invariata",
    `cleanup ${String(suggestedSetup.recommended_cleanup_label ?? "basic")}`,
    `repair ${
      opts.forceUltra
        ? "ultra"
        : String(suggestedSetup.recommended_repair_label ?? "smart")
    }`,
    opts.applyTrim
      ? `trim ${Boolean(suggestedSetup.recommended_trim_transparent) ? "on" : "off"}`
      : "trim invariato",
  ];
  const profileName = detectSetupProfileName(opts);
  const profileLabel =
    profileName === "custom" ? "Personalizzato" : SETUP_PROFILES[profileName].label;
  els.presetSetupSummary.textContent = `Setup consigliato (${profileLabel}): ${parts.join(", ")}.`;
  els.presetSetupSummary.classList.remove("hidden");
  els.setupProfileBar.classList.remove("hidden");
  els.setupQuickOptions.classList.remove("hidden");
  updateSetupProfileUi();
}

function clearVariantCompare() {
  revokeVariantCompareUrls();
  els.variantCompareGrid.textContent = "";
  els.variantPreviewGrid.textContent = "";
  els.variantComparePanel.classList.add("hidden");
  els.variantPreviewPanel.classList.add("hidden");
  els.variantRecommendation.classList.add("hidden");
  els.variantRecommendationLabel.textContent = "Scelta consigliata";
  els.variantRecommendationReason.textContent = "";
  els.applyRecommendedVariantBtn.disabled = true;
  recommendedVariantKey = null;
  els.variantPreviewStatus.textContent = "Nessuna variante generata.";
  els.variantShowDiff.checked = false;
  setVariantCompareStatus("");
}

function computeDifficulty(suggestion) {
  const presetKey = String(suggestion?.preset_key ?? "");
  const opaqueRatio = Number(suggestion?.opaque_ratio ?? 0);
  const uniqueColors = Number(suggestion?.unique_colors ?? 0);
  const edgeDensity = Number(suggestion?.edge_density ?? 0);
  const dominantAlpha = Boolean(suggestion?.dominant_alpha ?? false);

  let score = 0;
  if (uniqueColors >= 56) score += 3;
  else if (uniqueColors >= 36) score += 2;
  else if (uniqueColors >= 20) score += 1;

  if (edgeDensity >= 22) score += 3;
  else if (edgeDensity >= 16) score += 2;
  else if (edgeDensity >= 10) score += 1;

  if (dominantAlpha) score += 2;
  else if (opaqueRatio < 0.45) score += 1;
  if (opaqueRatio < 0.28) score += 1;

  if (presetKey === "ultra-cleanup") score += 2;
  else if (presetKey === "character-cleanup" || presetKey === "tileset-cleanup") score += 1;

  if (score <= 2) {
    return {
      label: "Difficolta' stimata: facile",
      levelClass: "level-easy",
    };
  }
  if (score <= 4) {
    return {
      label: "Difficolta' stimata: media",
      levelClass: "level-medium",
    };
  }
  if (score <= 7) {
    return {
      label: "Difficolta' stimata: difficile",
      levelClass: "level-hard",
    };
  }
  return {
    label: "Difficolta' stimata: molto difficile",
    levelClass: "level-extreme",
  };
}

function renderDifficulty(suggestion) {
  if (!suggestion) {
    els.presetDifficulty.textContent = "";
    els.presetDifficulty.className = "difficultyBadge hidden";
    return;
  }
  const difficulty = computeDifficulty(suggestion);
  els.presetDifficulty.textContent = difficulty.label;
  els.presetDifficulty.className = `difficultyBadge ${difficulty.levelClass}`;
}

function buildForecast(suggestion) {
  const difficulty = computeDifficulty(suggestion);
  const presetKey = String(suggestion?.preset_key ?? "");
  const uniqueColors = Number(suggestion?.unique_colors ?? 0);
  const edgeDensity = Number(suggestion?.edge_density ?? 0);
  const dominantAlpha = Boolean(suggestion?.dominant_alpha ?? false);
  const opaqueRatio = Number(suggestion?.opaque_ratio ?? 0);

  if (difficulty.levelClass === "level-easy") {
    return {
      levelClass: "level-good",
      html: "<strong>Esito previsto</strong>: buona probabilita' di ottenere un risultato gia' pulito con poche correzioni manuali.",
    };
  }

  if (
    difficulty.levelClass === "level-medium" &&
    uniqueColors <= 36 &&
    edgeDensity < 18 &&
    !dominantAlpha
  ) {
    return {
      levelClass: "level-good",
      html: "<strong>Esito previsto</strong>: probabile risultato buono. Conviene partire dal preset suggerito e rifinire solo se necessario.",
    };
  }

  if (
    presetKey === "ultra-cleanup" ||
    uniqueColors >= 56 ||
    edgeDensity >= 22 ||
    opaqueRatio < 0.28
  ) {
    return {
      levelClass: "level-risky",
      html: "<strong>Esito previsto</strong>: caso difficile. Possibile intervento manuale dopo il processing; valuta `palette lock`, `repair Ultra` e confronto con l'originale.",
    };
  }

  return {
    levelClass: "level-mixed",
    html: "<strong>Esito previsto</strong>: risultato potenzialmente buono ma con aree che potrebbero richiedere qualche regolazione su palette, repair o denoise.",
  };
}

function renderForecast(suggestion) {
  if (!suggestion) {
    els.presetForecast.textContent = "";
    els.presetForecast.className = "forecastBox hidden";
    return;
  }
  const forecast = buildForecast(suggestion);
  els.presetForecast.innerHTML = forecast.html;
  els.presetForecast.className = `forecastBox ${forecast.levelClass}`;
}

function buildPresetWarnings(suggestion) {
  const presetKey = String(suggestion?.preset_key ?? "");
  const opaqueRatio = Number(suggestion?.opaque_ratio ?? 0);
  const uniqueColors = Number(suggestion?.unique_colors ?? 0);
  const edgeDensity = Number(suggestion?.edge_density ?? 0);
  const dominantAlpha = Boolean(suggestion?.dominant_alpha ?? false);

  const warnings = [];
  const push = (title, text) => {
    warnings.push(`<strong>${title}</strong>: ${text}`);
  };

  if (uniqueColors >= 56) {
    push("Troppi colori", "la sorgente sembra molto frammentata: senza `palette cleanup Strict` o palette lock il risultato puo' restare instabile.");
  }

  if (edgeDensity >= 22) {
    push("Griglia instabile", "i bordi sono molto densi o rumorosi: il recovery della griglia puo' oscillare e richiedere `denoise Box 3x3`.");
  }

  if (dominantAlpha || opaqueRatio < 0.28) {
    push("Contenuto opaco ridotto", "molta trasparenza o poco contenuto visibile possono rendere piu' difficile stimare correttamente griglia e silhouette.");
  }

  if (presetKey === "ultra-cleanup") {
    push("Repair aggressivo", "`Ultra` puo' chiudere dettagli fini o alterare piccole forme: confronta sempre output e originale.");
  }

  if (presetKey === "strict-retro" && uniqueColors > 28) {
    push("Compressione forte", "`Strict Retro` su una sorgente ricca di colori puo' sacrificare molto dettaglio.");
  }

  return warnings.slice(0, 4);
}

function renderPresetWarnings(items) {
  els.presetWarnings.textContent = "";
  if (!Array.isArray(items) || items.length === 0) {
    els.presetWarnings.classList.add("hidden");
    return;
  }

  for (const item of items) {
    const el = document.createElement("div");
    el.className = "warningItem";
    el.innerHTML = item;
    els.presetWarnings.appendChild(el);
  }
  els.presetWarnings.classList.remove("hidden");
}

function buildPresetAdvice(suggestion) {
  const presetKey = String(suggestion?.preset_key ?? "");
  const opaqueRatio = Number(suggestion?.opaque_ratio ?? 0);
  const uniqueColors = Number(suggestion?.unique_colors ?? 0);
  const edgeDensity = Number(suggestion?.edge_density ?? 0);
  const dominantAlpha = Boolean(suggestion?.dominant_alpha ?? false);

  const advice = [];
  const push = (title, text) => {
    advice.push(`<strong>${title}</strong>: ${text}`);
  };

  if (presetKey === "ultra-cleanup") {
    push("Setup", "parti da `Ultra Cleanup` o da `repair Ultra` se vedi outline tremolanti, checker diagonali o micro-isole.");
    push("Palette", "se i colori sono instabili, abbina `palette cleanup Strict` e valuta una `palette lock` se hai gia' una palette target.");
  } else if (presetKey === "tileset-cleanup") {
    push("Setup", "usa `palette dalle celle` e `repair Smart` per tenere la griglia piu' coerente tra tile adiacenti.");
    push("Output", "evita dithering se il tileset deve restare molto pulito o modulare.");
  } else if (presetKey === "character-cleanup") {
    push("Setup", "tieni `repair Smart` e `cell color Medoid` per silhouette piu' leggibili e masse piu' compatte.");
    push("Contorni", "se i bordi restano sporchi, prova `repair Ultra` solo come secondo passaggio.");
  } else if (presetKey === "icon-cleanup") {
    push("Setup", "mantieni palette piu' piccola e poco dithering per icone e UI piu' leggibili.");
    push("Output", "usa `trim trasparenza` se vuoi esportare asset pronti da interfaccia.");
  } else if (presetKey === "strict-retro") {
    push("Setup", "buona scelta se vuoi blocchi netti, pochi colori e look retro piu' controllato.");
    push("Colore", "tieni `dither Off` se vuoi pixel piu' puliti e meno texture artificiale.");
  } else {
    push("Setup", "parti da `AI Sprite` con `repair Smart` e `cell color Medoid` per il caso generale.");
    push("Fallback", "se il risultato resta troppo sporco, passa a `Ultra Cleanup` oppure aumenta il rigore della palette.");
  }

  if (uniqueColors > 40) {
    push("Segnale", "l'immagine sembra avere molti colori: `palette cleanup Strict` o una palette piu' piccola possono aiutare.");
  } else if (uniqueColors > 0 && uniqueColors <= 18) {
    push("Segnale", "la palette di partenza sembra gia' contenuta: evita correzioni troppo aggressive se il look ti piace.");
  }

  if (edgeDensity > 16) {
    push("Bordi", "la densita' dei bordi e' alta: `denoise Box 3x3` puo' stabilizzare il recovery della griglia.");
  }

  if (dominantAlpha || opaqueRatio < 0.45) {
    push("Trasparenza", "se c'e' molto alpha o poco contenuto opaco, controlla bene `trim trasparenza` e i contorni esterni.");
  }

  return advice.slice(0, 4);
}

function renderPresetAdvice(items) {
  els.presetAdvice.textContent = "";
  if (!Array.isArray(items) || items.length === 0) {
    els.presetAdvice.classList.add("hidden");
    return;
  }

  for (const item of items) {
    const el = document.createElement("div");
    el.className = "adviceItem";
    el.innerHTML = item;
    els.presetAdvice.appendChild(el);
  }
  els.presetAdvice.classList.remove("hidden");
}

function updateDebugSummary(debug) {
  if (!debug) {
    clearDebugSummary();
    return;
  }
  const stepX = Number(debug.step_x ?? 0);
  const stepY = Number(debug.step_y ?? 0);
  const outputWidth = Number(debug.output_width ?? 0);
  const outputHeight = Number(debug.output_height ?? 0);
  const paletteCount = Array.isArray(debug.palette) ? debug.palette.length : Number(debug.palette_count ?? 0);
  const config = debug.config ?? {};
  const paletteLockUsed = Boolean(config.palette_lock_used ?? false);
  const paletteLockSize = Number(config.palette_lock_size ?? 0);

  els.debugPixelSize.textContent = `${stepX.toFixed(1)} x ${stepY.toFixed(1)} px`;
  els.debugGridSize.textContent = outputWidth > 0 && outputHeight > 0 ? `${outputWidth} x ${outputHeight}` : "-";
  els.debugPaletteCount.textContent = paletteCount > 0 ? `${paletteCount} colori` : "0";
  els.debugModes.textContent =
    `${paletteLockUsed ? `lock ${paletteLockSize}` : "lock off"} | prefilter ${config.prefilter_mode ?? "off"} | palette ${config.palette_source ?? "cells"} | ` +
    `palette-fix ${config.palette_cleanup_mode ?? "basic"} | cella ${config.cell_color_mode ?? "dominant"} | ` +
    `dither ${config.dither_mode ?? "off"} | spazio ${config.color_space ?? "linear"} | ` +
    `cleanup ${config.cleanup_mode ?? "basic"} | repair ${config.repair_mode ?? "smart"}`;
}

function normalizeDebugResult(dbg, k) {
  const col = Array.from(dbg.col_cuts ?? []);
  const row = Array.from(dbg.row_cuts ?? []);
  return {
    overlayBytes: new Uint8Array(dbg.overlay_bytes ?? []),
    debug: {
      palette: Array.from(dbg.palette ?? []).map((entry) => ({
        r: Number(entry.r ?? 0),
        g: Number(entry.g ?? 0),
        b: Number(entry.b ?? 0),
        a: Number(entry.a ?? 255),
        count: Number(entry.count ?? 0),
        hex: String(entry.hex ?? ""),
      })),
      palette_count: Number(dbg.palette_count ?? 0),
      col_cuts: col,
      row_cuts: row,
      input_width: Number(dbg.input_width ?? 0),
      input_height: Number(dbg.input_height ?? 0),
      output_width: Number(dbg.output_width ?? 0),
      output_height: Number(dbg.output_height ?? 0),
      step_x: Number(dbg.step_x ?? 0),
      step_y: Number(dbg.step_y ?? 0),
      config: {
        k_colors: Number(dbg.config?.k_colors ?? k),
        pixel_size_override: dbg.config?.pixel_size_override ?? null,
        pixel_size_override_used: Boolean(dbg.config?.pixel_size_override_used ?? false),
        palette_lock_used: Boolean(dbg.config?.palette_lock_used ?? false),
        palette_lock_size: Number(dbg.config?.palette_lock_size ?? 0),
        prefilter_mode: String(dbg.config?.prefilter_mode ?? "off"),
        palette_source: String(dbg.config?.palette_source ?? "cells"),
        palette_cleanup_mode: String(dbg.config?.palette_cleanup_mode ?? "basic"),
        cell_color_mode: String(dbg.config?.cell_color_mode ?? "dominant"),
        dither_mode: String(dbg.config?.dither_mode ?? "off"),
        color_space: String(dbg.config?.color_space ?? "linear"),
        cleanup_mode: String(dbg.config?.cleanup_mode ?? "basic"),
        repair_mode: String(dbg.config?.repair_mode ?? "smart"),
      },
    },
  };
}

async function getPaletteLockBytes() {
  if (!els.paletteLockEnabled.checked) return null;
  const file = els.paletteLockFile?.files?.[0] ?? null;
  if (!file) {
    throw new Error("Palette lock attiva ma nessun file palette selezionato");
  }
  return new Uint8Array(await file.arrayBuffer());
}

function applyZoom() {
  const z = Number.parseInt(els.zoom.value, 10);
  const zoom = Number.isFinite(z) ? z : 1;
  const targets = [
    els.inputPreview,
    els.inputGridOverlay,
    els.outputPreview,
    els.compareInput,
    els.compareOutput,
  ];
  for (const el of targets) {
    el.style.transform = zoom === 1 ? "" : `scale(${zoom})`;
    el.style.transformOrigin = "center center";
  }
}

function updateCompare() {
  const v = Number.parseInt(els.compareSlider.value, 10);
  const pct = Number.isFinite(v) ? Math.min(100, Math.max(0, v)) : 50;
  els.compareInputWrap.style.width = `${pct}%`;
  els.compareLabel.textContent = `${pct}%`;
  els.compareLine.style.left = `calc(${pct}% - 1px)`;
  els.compareHandle.style.left = `${pct}%`;
}

function setComparePct(pct) {
  const p = Math.min(100, Math.max(0, Math.round(pct)));
  els.compareSlider.value = String(p);
  updateCompare();
}

function comparePctFromClientX(clientX) {
  const rect = els.compareFrame.getBoundingClientRect();
  if (!rect.width) return 50;
  return ((clientX - rect.left) / rect.width) * 100;
}

function getDownloadMode() {
  const el = document.querySelector('input[name="downloadMode"]:checked');
  return el?.value ?? "single";
}

function openDrawer() {
  els.drawer.classList.remove("hidden");
  els.drawerOverlay.classList.remove("hidden");
  els.drawer.setAttribute("aria-hidden", "false");
}

function closeDrawer() {
  els.openOptions.focus();
  els.drawer.classList.add("hidden");
  els.drawerOverlay.classList.add("hidden");
  els.drawer.setAttribute("aria-hidden", "true");
}

function readSettings() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function writeSettings() {
  try {
    const settings = {
      activePresetKey,
      kColors: els.kColors.value,
      pixelOverrideEnabled: els.pixelOverrideEnabled.checked,
      pixelSize: els.pixelSize.value,
      zoom: els.zoom.value,
      batchMode: els.batchMode.checked,
      denoiseMode: els.denoiseMode.value,
      paletteSource: els.paletteSource.value,
      paletteLockEnabled: els.paletteLockEnabled.checked,
      paletteCleanupMode: els.paletteCleanupMode.value,
      cellColorMode: els.cellColorMode.value,
      ditherMode: els.ditherMode.value,
      colorSpace: els.colorSpace.value,
      cleanupMode: els.cleanupMode.value,
      repairMode: els.repairMode.value,
      trimTransparent: els.trimTransparent.checked,
      scaleFactor: els.scaleFactor.value,
      showPalette: els.showPalette.checked,
      showGrid: els.showGrid.checked,
      downloadMode: getDownloadMode(),
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {}
}

function applySettings(settings) {
  if (!settings) return;
  if (settings.activePresetKey != null) activePresetKey = String(settings.activePresetKey);
  if (settings.kColors != null) els.kColors.value = String(settings.kColors);
  if (settings.pixelOverrideEnabled != null)
    els.pixelOverrideEnabled.checked = Boolean(settings.pixelOverrideEnabled);
  if (settings.pixelSize != null) els.pixelSize.value = String(settings.pixelSize);
  if (settings.zoom != null) els.zoom.value = String(settings.zoom);
  if (settings.batchMode != null) els.batchMode.checked = Boolean(settings.batchMode);
  if (settings.denoiseMode != null) els.denoiseMode.value = String(settings.denoiseMode);
  if (settings.paletteSource != null) els.paletteSource.value = String(settings.paletteSource);
  if (settings.paletteLockEnabled != null)
    els.paletteLockEnabled.checked = Boolean(settings.paletteLockEnabled);
  if (settings.paletteCleanupMode != null)
    els.paletteCleanupMode.value = String(settings.paletteCleanupMode);
  if (settings.cellColorMode != null) els.cellColorMode.value = String(settings.cellColorMode);
  if (settings.ditherMode != null) els.ditherMode.value = String(settings.ditherMode);
  if (settings.colorSpace != null) els.colorSpace.value = String(settings.colorSpace);
  if (settings.cleanupMode != null) els.cleanupMode.value = String(settings.cleanupMode);
  if (settings.repairMode != null) els.repairMode.value = String(settings.repairMode);
  if (settings.trimTransparent != null)
    els.trimTransparent.checked = Boolean(settings.trimTransparent);
  if (settings.scaleFactor != null) els.scaleFactor.value = String(settings.scaleFactor);
  if (settings.showPalette != null) els.showPalette.checked = Boolean(settings.showPalette);
  if (settings.showGrid != null) els.showGrid.checked = Boolean(settings.showGrid);
  if (settings.downloadMode === "zip") {
    const zipRadio = document.querySelector('input[name="downloadMode"][value="zip"]');
    if (zipRadio) zipRadio.checked = true;
  }
}

function updatePresetStatus() {
  document.querySelectorAll(".presetBtn").forEach((btn) => {
    const preset = btn.getAttribute("data-preset");
    btn.classList.toggle("is-active", Boolean(activePresetKey) && preset === activePresetKey);
  });

  if (!activePresetKey) {
    els.presetStatus.textContent = "Preset manuale non selezionato.";
    return;
  }

  const labels = {
    "ai-sprite": "Preset attivo: AI Sprite Cleanup",
    "strict-retro": "Preset attivo: Strict Retro",
    "tileset-cleanup": "Preset attivo: Tileset Cleanup",
    "character-cleanup": "Preset attivo: Character Cleanup",
    "icon-cleanup": "Preset attivo: Icon Cleanup",
    "ultra-cleanup": "Preset attivo: Ultra Cleanup",
  };
  els.presetStatus.textContent = labels[activePresetKey] ?? `Preset attivo: ${activePresetKey}`;
}

function markPresetCustom() {
  if (!activePresetKey) return;
  activePresetKey = null;
  updatePresetStatus();
}

async function applyPreset(presetKey) {
  const preset = get_preset_config(presetKey);
  els.kColors.value = String(preset.k_colors ?? els.kColors.value);
  els.pixelOverrideEnabled.checked = false;
  els.pixelSize.disabled = true;
  els.denoiseMode.value = String(preset.prefilter_mode ?? els.denoiseMode.value);
  els.paletteSource.value = String(preset.palette_source ?? els.paletteSource.value);
  els.paletteCleanupMode.value = String(
    preset.palette_cleanup_mode ?? els.paletteCleanupMode.value,
  );
  els.cellColorMode.value = String(preset.cell_color_mode ?? els.cellColorMode.value);
  els.ditherMode.value = String(preset.dither_mode ?? els.ditherMode.value);
  els.colorSpace.value = String(preset.color_space ?? els.colorSpace.value);
  els.cleanupMode.value = String(preset.cleanup_mode ?? els.cleanupMode.value);
  els.repairMode.value = String(preset.repair_mode ?? els.repairMode.value);
  activePresetKey = presetKey;
  updatePresetStatus();
  updateUiFromSettings();
  writeSettings();
  setStatus(`Preset applicato: ${preset.label ?? presetKey}`);
}

async function applySuggestedSetupConfig(setup) {
  if (!setup || !setup.preset_key) return;
  const opts = describeQuickSetupChoices();
  await applyPreset(String(setup.preset_key));
  els.denoiseMode.value = String(setup.recommended_prefilter_mode ?? els.denoiseMode.value);
  if (opts.applyPalette) {
    els.paletteSource.value = String(setup.recommended_palette_source ?? els.paletteSource.value);
    els.paletteCleanupMode.value = String(
      setup.recommended_palette_cleanup_mode ?? els.paletteCleanupMode.value,
    );
    els.cellColorMode.value = String(
      setup.recommended_cell_color_mode ?? els.cellColorMode.value,
    );
  }
  els.cleanupMode.value = String(setup.recommended_cleanup_mode ?? els.cleanupMode.value);
  els.repairMode.value = opts.forceUltra
    ? "3"
    : String(setup.recommended_repair_mode ?? els.repairMode.value);
  if (opts.applyTrim) {
    els.trimTransparent.checked = Boolean(setup.recommended_trim_transparent);
  }
  activePresetKey = String(setup.preset_key);
  updatePresetStatus();
  updateUiFromSettings();
  writeSettings();
  const note = String(setup.recommendation_reason ?? "").trim();
  setStatus(
    note
      ? `Setup consigliato applicato: ${setup.preset_key} (${note})`
      : `Setup consigliato applicato: ${setup.preset_key}`,
  );
}

async function refreshPresetSuggestion(file) {
  if (!file || !wasmReady) {
    clearPresetSuggestion();
    return;
  }

  try {
    const inputBytes = new Uint8Array(await file.arrayBuffer());
    const suggestion = suggest_setup(inputBytes);
    suggestedSetup = suggestion;
    suggestedPresetKey = String(suggestion.preset_key ?? "");
    els.presetSuggestion.textContent =
      `${suggestedPresetKey}: ${String(suggestion.reason ?? "Nessuna motivazione")}`;
    renderDifficulty(suggestion);
    renderForecast(suggestion);
    renderPresetWarnings(buildPresetWarnings(suggestion));
    renderPresetAdvice(buildPresetAdvice(suggestion));
    applySetupProfile("balanced");
    updateSuggestedSetupSummary();
    els.applySuggestedSetupBtn.disabled = !suggestedSetup;
    els.applySuggestedPresetBtn.disabled = !suggestedPresetKey;
    els.runVariantCompareBtn.disabled = !suggestedSetup || els.batchMode.checked;
  } catch (e) {
    suggestedPresetKey = null;
    suggestedSetup = null;
    els.presetSuggestion.textContent = `Suggerimento non disponibile: ${String(e)}`;
    els.presetDifficulty.textContent = "";
    els.presetDifficulty.className = "difficultyBadge hidden";
    els.presetForecast.textContent = "";
    els.presetForecast.className = "forecastBox hidden";
    els.presetWarnings.textContent = "";
    els.presetWarnings.classList.add("hidden");
    els.presetAdvice.textContent = "";
    els.presetAdvice.classList.add("hidden");
    els.presetSetupSummary.textContent = "";
    els.presetSetupSummary.classList.add("hidden");
    els.setupProfileBar.classList.add("hidden");
    els.setupProfileStatus.textContent = "";
    els.setupProfileStatus.classList.add("hidden");
    els.setupQuickOptions.classList.add("hidden");
    applySetupProfile("balanced");
    els.applySuggestedSetupBtn.disabled = true;
    els.applySuggestedPresetBtn.disabled = true;
    els.runVariantCompareBtn.disabled = true;
  }
}

function updateUiFromSettings() {
  els.pixelSize.disabled = !els.pixelOverrideEnabled.checked;
  els.fileInput.multiple = els.batchMode.checked;
  els.downloadAllBtn.textContent = els.batchMode.checked ? "Scarica Tutto" : "Scarica PNG";
  els.batchList.parentElement.classList.toggle("hidden", !els.batchMode.checked);
  els.zipStatus.textContent = els.batchMode.checked
    ? "ZIP: interno (offline). Consigliato per batch."
    : "ZIP: interno (offline).";
  setDownloadAllEnabled(els.batchMode.checked ? batchResults.length > 0 : Boolean(outputUrl));
}

function getAlgoOptions() {
  const denoise = Number.parseInt(els.denoiseMode.value, 10);
  const paletteSource = Number.parseInt(els.paletteSource.value, 10);
  const paletteCleanupMode = Number.parseInt(els.paletteCleanupMode.value, 10);
  const cellColorMode = Number.parseInt(els.cellColorMode.value, 10);
  const dither = Number.parseInt(els.ditherMode.value, 10);
  const colorSpace = Number.parseInt(els.colorSpace.value, 10);
  const cleanupMode = Number.parseInt(els.cleanupMode.value, 10);
  const repairMode = Number.parseInt(els.repairMode.value, 10);
  return {
    denoise: Number.isFinite(denoise) ? denoise : undefined,
    paletteSource: Number.isFinite(paletteSource) ? paletteSource : undefined,
    paletteCleanupMode: Number.isFinite(paletteCleanupMode) ? paletteCleanupMode : undefined,
    cellColorMode: Number.isFinite(cellColorMode) ? cellColorMode : undefined,
    dither: Number.isFinite(dither) ? dither : undefined,
    colorSpace: Number.isFinite(colorSpace) ? colorSpace : undefined,
    cleanupMode: Number.isFinite(cleanupMode) ? cleanupMode : undefined,
    repairMode: Number.isFinite(repairMode) ? repairMode : undefined,
  };
}

function clearGridOverlay() {
  const c = els.inputGridOverlay;
  const ctx = c.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, c.width, c.height);
}

function renderInputGrid() {
  if (!els.showGrid.checked || !lastDebug || !els.inputPreview.src) {
    clearGridOverlay();
    return;
  }

  const img = els.inputPreview;
  const w = img.clientWidth;
  const h = img.clientHeight;
  if (!w || !h) return;

  const dpr = window.devicePixelRatio || 1;
  const canvas = els.inputGridOverlay;
  canvas.width = Math.max(1, Math.round(w * dpr));
  canvas.height = Math.max(1, Math.round(h * dpr));

  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);

  const sx = w / lastDebug.input_width;
  const sy = h / lastDebug.input_height;

  ctx.lineWidth = 1;
  ctx.strokeStyle = "rgba(75, 227, 255, 0.55)";

  ctx.beginPath();
  for (const x of lastDebug.col_cuts) {
    const px = x * sx;
    ctx.moveTo(px + 0.5, 0);
    ctx.lineTo(px + 0.5, h);
  }
  for (const y of lastDebug.row_cuts) {
    const py = y * sy;
    ctx.moveTo(0, py + 0.5);
    ctx.lineTo(w, py + 0.5);
  }
  ctx.stroke();
}

let crcTable = null;

function crc32(bytes) {
  if (!crcTable) {
    crcTable = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) {
        c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
      }
      crcTable[n] = c >>> 0;
    }
  }

  let crc = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) {
    crc = crcTable[(crc ^ bytes[i]) & 255] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function toZipEntryName(originalName) {
  const base = originalName.replace(/^.*[\\/]/, "").replace(/\.[^/.]+$/, "");
  return `${base}_nasty-retropixel.png`;
}

function buildZipStore(entries) {
  const enc = new TextEncoder();
  const chunks = [];
  const records = [];
  let offset = 0;

  const push = (u8) => {
    chunks.push(u8);
    offset += u8.length;
  };

  for (const e of entries) {
    const nameBytes = enc.encode(e.name);
    const data = e.bytes instanceof Uint8Array ? e.bytes : new Uint8Array(e.bytes);
    const crc = crc32(data);
    const localOffset = offset;

    const hdr = new Uint8Array(30 + nameBytes.length);
    const dv = new DataView(hdr.buffer);
    dv.setUint32(0, 0x04034b50, true);
    dv.setUint16(4, 20, true);
    dv.setUint16(6, 0x0800, true);
    dv.setUint16(8, 0, true);
    dv.setUint16(10, 0, true);
    dv.setUint16(12, 0, true);
    dv.setUint32(14, crc, true);
    dv.setUint32(18, data.length, true);
    dv.setUint32(22, data.length, true);
    dv.setUint16(26, nameBytes.length, true);
    dv.setUint16(28, 0, true);
    hdr.set(nameBytes, 30);

    push(hdr);
    push(data);

    records.push({
      nameBytes,
      crc,
      size: data.length,
      offset: localOffset,
    });
  }

  const centralStart = offset;
  for (const r of records) {
    const hdr = new Uint8Array(46 + r.nameBytes.length);
    const dv = new DataView(hdr.buffer);
    dv.setUint32(0, 0x02014b50, true);
    dv.setUint16(4, 20, true);
    dv.setUint16(6, 20, true);
    dv.setUint16(8, 0x0800, true);
    dv.setUint16(10, 0, true);
    dv.setUint16(12, 0, true);
    dv.setUint16(14, 0, true);
    dv.setUint32(16, r.crc, true);
    dv.setUint32(20, r.size, true);
    dv.setUint32(24, r.size, true);
    dv.setUint16(28, r.nameBytes.length, true);
    dv.setUint16(30, 0, true);
    dv.setUint16(32, 0, true);
    dv.setUint16(34, 0, true);
    dv.setUint16(36, 0, true);
    dv.setUint32(38, 0, true);
    dv.setUint32(42, r.offset, true);
    hdr.set(r.nameBytes, 46);
    push(hdr);
  }

  const centralSize = offset - centralStart;
  const end = new Uint8Array(22);
  const endDv = new DataView(end.buffer);
  endDv.setUint32(0, 0x06054b50, true);
  endDv.setUint16(4, 0, true);
  endDv.setUint16(6, 0, true);
  endDv.setUint16(8, records.length, true);
  endDv.setUint16(10, records.length, true);
  endDv.setUint32(12, centralSize, true);
  endDv.setUint32(16, centralStart, true);
  endDv.setUint16(20, 0, true);
  push(end);

  let total = 0;
  for (const c of chunks) total += c.length;
  const out = new Uint8Array(total);
  let p = 0;
  for (const c of chunks) {
    out.set(c, p);
    p += c.length;
  }
  return out;
}

async function decodePngToBitmap(pngBytes) {
  const blob = new Blob([pngBytes], { type: "image/png" });
  if (typeof createImageBitmap === "function") {
    return createImageBitmap(blob);
  }

  const url = URL.createObjectURL(blob);
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("Impossibile decodificare PNG"));
    };
    img.src = url;
  });
}

async function decodeImageBytesToBitmap(bytes, mimeType = "image/png") {
  const blob = new Blob([bytes], { type: mimeType || "image/png" });
  if (typeof createImageBitmap === "function") {
    return createImageBitmap(blob);
  }

  const url = URL.createObjectURL(blob);
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("Impossibile decodificare immagine"));
    };
    img.src = url;
  });
}

function canvasToPngBytes(canvas) {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      async (blob) => {
        if (!blob) {
          reject(new Error("toBlob failed"));
          return;
        }
        const ab = await blob.arrayBuffer();
        resolve(new Uint8Array(ab));
      },
      "image/png",
      1,
    );
  });
}

async function buildVariantDiffPng(inputBytes, inputMimeType, outputBytes) {
  const inputBmp = await decodeImageBytesToBitmap(inputBytes, inputMimeType);
  const outputBmp = await decodePngToBitmap(outputBytes);
  const width = Math.max(
    Number(inputBmp.width ?? inputBmp.naturalWidth ?? 0),
    Number(outputBmp.width ?? outputBmp.naturalWidth ?? 0),
    1,
  );
  const height = Math.max(
    Number(inputBmp.height ?? inputBmp.naturalHeight ?? 0),
    Number(outputBmp.height ?? outputBmp.naturalHeight ?? 0),
    1,
  );

  const sourceA = document.createElement("canvas");
  sourceA.width = width;
  sourceA.height = height;
  const ctxA = sourceA.getContext("2d", { willReadFrequently: true });

  const sourceB = document.createElement("canvas");
  sourceB.width = width;
  sourceB.height = height;
  const ctxB = sourceB.getContext("2d", { willReadFrequently: true });

  const diffCanvas = document.createElement("canvas");
  diffCanvas.width = width;
  diffCanvas.height = height;
  const diffCtx = diffCanvas.getContext("2d", { willReadFrequently: true });

  if (!ctxA || !ctxB || !diffCtx) {
    throw new Error("Canvas diff non disponibile");
  }

  ctxA.imageSmoothingEnabled = false;
  ctxB.imageSmoothingEnabled = false;
  diffCtx.imageSmoothingEnabled = false;

  const drawCentered = (ctx, bmp) => {
    const w = Number(bmp.width ?? bmp.naturalWidth ?? 0);
    const h = Number(bmp.height ?? bmp.naturalHeight ?? 0);
    const x = Math.floor((width - w) / 2);
    const y = Math.floor((height - h) / 2);
    ctx.clearRect(0, 0, width, height);
    ctx.drawImage(bmp, x, y);
  };

  drawCentered(ctxA, inputBmp);
  drawCentered(ctxB, outputBmp);

  const dataA = ctxA.getImageData(0, 0, width, height);
  const dataB = ctxB.getImageData(0, 0, width, height);
  const out = diffCtx.createImageData(width, height);

  let diffSum = 0;
  let activePixels = 0;
  for (let i = 0; i < out.data.length; i += 4) {
    const dr = Math.abs(dataA.data[i] - dataB.data[i]);
    const dg = Math.abs(dataA.data[i + 1] - dataB.data[i + 1]);
    const db = Math.abs(dataA.data[i + 2] - dataB.data[i + 2]);
    const da = Math.abs(dataA.data[i + 3] - dataB.data[i + 3]);
    const intensity = Math.min(255, Math.round((dr + dg + db) / 3 + da * 0.35));
    diffSum += intensity;
    if (intensity > 12) activePixels += 1;

    out.data[i] = Math.min(255, intensity * 2);
    out.data[i + 1] = Math.min(255, intensity * 0.72);
    out.data[i + 2] = Math.max(0, 210 - intensity);
    out.data[i + 3] = intensity > 8 ? Math.min(220, Math.round(intensity * 0.9)) : 0;
  }

  diffCtx.putImageData(out, 0, 0);
  return {
    bytes: await canvasToPngBytes(diffCanvas),
    score: Number((diffSum / Math.max(1, width * height)).toFixed(1)),
    activeRatio: Number((activePixels / Math.max(1, width * height)).toFixed(3)),
  };
}

function trimCanvasTransparent(canvas) {
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return canvas;
  const { width, height } = canvas;
  const img = ctx.getImageData(0, 0, width, height);
  const d = img.data;

  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const a = d[(y * width + x) * 4 + 3];
      if (a === 0) continue;
      if (x < minX) minX = x;
      if (y < minY) minY = y;
      if (x > maxX) maxX = x;
      if (y > maxY) maxY = y;
    }
  }

  if (maxX < minX || maxY < minY) return canvas;

  const w = maxX - minX + 1;
  const h = maxY - minY + 1;
  const out = document.createElement("canvas");
  out.width = w;
  out.height = h;
  const outCtx = out.getContext("2d");
  if (!outCtx) return canvas;
  outCtx.imageSmoothingEnabled = false;
  outCtx.drawImage(canvas, minX, minY, w, h, 0, 0, w, h);
  return out;
}

function scaleCanvasNearest(canvas, factor) {
  const f = Number.parseInt(factor, 10);
  if (!Number.isFinite(f) || f <= 1) return canvas;
  const out = document.createElement("canvas");
  out.width = canvas.width * f;
  out.height = canvas.height * f;
  const ctx = out.getContext("2d");
  if (!ctx) return canvas;
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(canvas, 0, 0, out.width, out.height);
  return out;
}

async function postProcessPng(pngBytes) {
  return postProcessPngWithOptions(pngBytes, {
    trimTransparent: els.trimTransparent.checked,
    scaleFactor: els.scaleFactor.value,
  });
}

async function postProcessPngWithOptions(pngBytes, options = {}) {
  const bmp = await decodePngToBitmap(pngBytes);
  const canvas = document.createElement("canvas");
  canvas.width = bmp.width ?? bmp.naturalWidth ?? 0;
  canvas.height = bmp.height ?? bmp.naturalHeight ?? 0;
  const ctx = canvas.getContext("2d");
  if (!ctx || canvas.width === 0 || canvas.height === 0) {
    throw new Error("Canvas non supportato o immagine non valida");
  }
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(bmp, 0, 0);

  let out = canvas;
  if (options.trimTransparent) out = trimCanvasTransparent(out);
  out = scaleCanvasNearest(out, options.scaleFactor ?? els.scaleFactor.value);
  return canvasToPngBytes(out);
}

async function computePaletteFromPng(pngBytes, maxColors = 64) {
  const bmp = await decodePngToBitmap(pngBytes);
  const canvas = document.createElement("canvas");
  canvas.width = bmp.width ?? bmp.naturalWidth ?? 0;
  canvas.height = bmp.height ?? bmp.naturalHeight ?? 0;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx || canvas.width === 0 || canvas.height === 0) {
    throw new Error("Canvas non supportato o immagine non valida");
  }
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(bmp, 0, 0);
  const img = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const d = img.data;

  const counts = new Map();
  const step = Math.max(1, Math.floor(Math.sqrt((canvas.width * canvas.height) / 20000)));
  for (let y = 0; y < canvas.height; y += step) {
    for (let x = 0; x < canvas.width; x += step) {
      const i = (y * canvas.width + x) * 4;
      const a = d[i + 3];
      if (a === 0) continue;
      const r = d[i];
      const g = d[i + 1];
      const b = d[i + 2];
      const key = (r << 16) | (g << 8) | b;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }

  const sorted = Array.from(counts.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, maxColors)
    .map(([key]) => {
      const r = (key >> 16) & 255;
      const g = (key >> 8) & 255;
      const b = key & 255;
      return `rgb(${r} ${g} ${b})`;
    });

  return sorted;
}

function renderSwatches(colors) {
  els.paletteSwatches.textContent = "";
  if (!els.showPalette.checked) return;
  for (const c of colors) {
    const el = document.createElement("div");
    el.className = "swatch";
    el.style.background = colorEntryToCss(c);
    if (typeof c !== "string" && c?.hex) el.title = `${c.hex} (${c.count ?? 0})`;
    els.paletteSwatches.appendChild(el);
  }
}

function clearBatchList() {
  els.batchList.textContent = "";
}

function addBatchItem(name) {
  const row = document.createElement("div");
  row.className = "batchItem";
  const left = document.createElement("div");
  left.className = "batchName";
  left.textContent = name;
  const right = document.createElement("div");
  right.className = "batchStatus";
  right.textContent = "in coda";
  row.appendChild(left);
  row.appendChild(right);
  els.batchList.appendChild(row);
  return { row, statusEl: right };
}

async function processSingleFile(file) {
  const ctx = await resolveSingleFileContext(file);
  return processResolvedSingleFile(ctx, { includeDebug: false });
}

async function resolveSingleFileContext(file) {
  const k = Number.parseInt(els.kColors.value, 10);
  if (!Number.isFinite(k) || k <= 0) {
    throw new Error("k colori non valido");
  }

  const pixelOverride = els.pixelOverrideEnabled.checked
    ? Number.parseFloat(els.pixelSize.value)
    : null;

  if (els.pixelOverrideEnabled.checked && (!Number.isFinite(pixelOverride) || pixelOverride <= 0)) {
    throw new Error("Pixel size override non valido");
  }

  const inputBytes = new Uint8Array(await file.arrayBuffer());
  const paletteLockBytes = await getPaletteLockBytes();
  return {
    k,
    pixelOverride,
    inputBytes,
    paletteLockBytes,
    algo: getAlgoOptions(),
    trimTransparent: els.trimTransparent.checked,
    scaleFactor: els.scaleFactor.value,
  };
}

async function processResolvedSingleFile(ctx, options = {}) {
  const algo = { ...ctx.algo, ...(options.algoOverrides ?? {}) };
  const trimTransparent =
    options.trimTransparentOverride ?? options.trimTransparent ?? ctx.trimTransparent;

  if (options.includeDebug) {
    const dbg = ctx.paletteLockBytes
      ? process_image_debug_with_palette_image(
          ctx.inputBytes,
          ctx.paletteLockBytes,
          ctx.k,
          ctx.pixelOverride,
          algo.denoise,
          algo.paletteSource,
          algo.paletteCleanupMode,
          algo.cellColorMode,
          algo.dither,
          algo.colorSpace,
          algo.cleanupMode,
          algo.repairMode,
        )
      : process_image_debug(
          ctx.inputBytes,
          ctx.k,
          ctx.pixelOverride,
          algo.denoise,
          algo.paletteSource,
          algo.paletteCleanupMode,
          algo.cellColorMode,
          algo.dither,
          algo.colorSpace,
          algo.cleanupMode,
          algo.repairMode,
        );
    const bytes = await postProcessPngWithOptions(dbg.bytes, {
      trimTransparent,
      scaleFactor: ctx.scaleFactor,
    });
    return {
      bytes,
      ...normalizeDebugResult(dbg, ctx.k),
    };
  }

  const rawBytes = ctx.paletteLockBytes
    ? process_image_with_palette_image(
        ctx.inputBytes,
        ctx.paletteLockBytes,
        ctx.k,
        ctx.pixelOverride,
        algo.denoise,
        algo.paletteSource,
        algo.paletteCleanupMode,
        algo.cellColorMode,
        algo.dither,
        algo.colorSpace,
        algo.cleanupMode,
        algo.repairMode,
      )
    : process_image(
        ctx.inputBytes,
        ctx.k,
        ctx.pixelOverride,
        algo.denoise,
        algo.paletteSource,
        algo.paletteCleanupMode,
        algo.cellColorMode,
        algo.dither,
        algo.colorSpace,
        algo.cleanupMode,
        algo.repairMode,
      );
  return {
    bytes: await postProcessPngWithOptions(rawBytes, {
      trimTransparent,
      scaleFactor: ctx.scaleFactor,
    }),
    debug: null,
  };
}

async function processSingleFileWithDebugIfNeeded(file) {
  const ctx = await resolveSingleFileContext(file);
  return processResolvedSingleFile(ctx, { includeDebug: true });
}

function buildVariantPlans() {
  const currentAlgo = getAlgoOptions();
  const ultraPreset = get_preset_config("ultra-cleanup");
  const presetKey = String(suggestedSetup?.preset_key ?? activePresetKey ?? "ai-sprite");
  const recommended = {
    denoise: Number(suggestedSetup?.recommended_prefilter_mode ?? currentAlgo.denoise ?? 0),
    paletteSource: Number(
      suggestedSetup?.recommended_palette_source ?? currentAlgo.paletteSource ?? 1,
    ),
    paletteCleanupMode: Number(
      suggestedSetup?.recommended_palette_cleanup_mode ?? currentAlgo.paletteCleanupMode ?? 1,
    ),
    cellColorMode: Number(
      suggestedSetup?.recommended_cell_color_mode ?? currentAlgo.cellColorMode ?? 1,
    ),
    dither: currentAlgo.dither,
    colorSpace: currentAlgo.colorSpace,
    cleanupMode: Number(suggestedSetup?.recommended_cleanup_mode ?? currentAlgo.cleanupMode ?? 1),
    repairMode: Number(suggestedSetup?.recommended_repair_mode ?? currentAlgo.repairMode ?? 2),
  };

  return [
    {
      key: "balanced",
      label: "Bilanciato",
      summary: "Setup consigliato standard per partire pulito senza spingere troppo il repair.",
      presetKey,
      keepPreset: true,
      trimTransparent: Boolean(suggestedSetup?.recommended_trim_transparent),
      algoOverrides: recommended,
    },
    {
      key: "aggressive",
      label: "Aggressivo",
      summary: "Spinge cleanup e repair per casi AI piu' sporchi o griglie instabili.",
      presetKey,
      keepPreset: false,
      trimTransparent: true,
      algoOverrides: {
        ...recommended,
        denoise: Math.max(Number(recommended.denoise ?? 0), 1),
        paletteCleanupMode: Math.max(Number(recommended.paletteCleanupMode ?? 1), 2),
        cellColorMode: 2,
        cleanupMode: Math.max(Number(recommended.cleanupMode ?? 1), 1),
        repairMode: 3,
      },
    },
    {
      key: "ultra",
      label: "Ultra",
      summary: "Usa il preset Ultra Cleanup per confrontare l'opzione piu' forte e strutturale.",
      presetKey: "ultra-cleanup",
      keepPreset: true,
      trimTransparent: true,
      algoOverrides: {
        denoise: Number(ultraPreset.prefilter_mode ?? 1),
        paletteSource: Number(ultraPreset.palette_source ?? 1),
        paletteCleanupMode: Number(ultraPreset.palette_cleanup_mode ?? 2),
        cellColorMode: Number(ultraPreset.cell_color_mode ?? 2),
        dither: Number(ultraPreset.dither_mode ?? currentAlgo.dither ?? 0),
        colorSpace: Number(ultraPreset.color_space ?? currentAlgo.colorSpace ?? 1),
        cleanupMode: Number(ultraPreset.cleanup_mode ?? 1),
        repairMode: 3,
      },
    },
  ];
}

async function promoteVariantResult(result) {
  if (!result) return;
  revokeUrl(outputUrl);
  const outputBlob = new Blob([result.bytes], { type: "image/png" });
  outputUrl = URL.createObjectURL(outputBlob);
  els.outputPreview.src = outputUrl;
  els.compareOutput.src = outputUrl;
  els.downloadBtn.href = outputUrl;
  setDownloadEnabled(true);
  setDownloadAllEnabled(true);

  lastDebug = result.debug;
  updateDebugSummary(lastDebug);
  const palette =
    Array.isArray(lastDebug?.palette) && lastDebug.palette.length > 0
      ? lastDebug.palette
      : await computePaletteFromPng(result.bytes, 64);
  renderSwatches(palette);
  renderInputGrid();
  updateCompare();
  applyZoom();

  revokeUrl(debugUrl);
  revokeUrl(overlayUrl);
  debugUrl = null;
  overlayUrl = null;
  if (lastDebug) {
    const debugBlob = new Blob([JSON.stringify(lastDebug, null, 2)], {
      type: "application/json",
    });
    debugUrl = URL.createObjectURL(debugBlob);
    els.downloadDebugBtn.href = debugUrl;
    setDebugDownloadEnabled(true);
    if (result.overlayBytes && result.overlayBytes.length > 0) {
      const overlayBlob = new Blob([result.overlayBytes], { type: "image/png" });
      overlayUrl = URL.createObjectURL(overlayBlob);
      els.downloadOverlayBtn.href = overlayUrl;
      setOverlayDownloadEnabled(true);
    } else {
      setOverlayDownloadEnabled(false);
    }
  } else {
    setDebugDownloadEnabled(false);
    setOverlayDownloadEnabled(false);
  }

  await applyPreset(result.plan.presetKey);
  els.denoiseMode.value = String(result.plan.algoOverrides.denoise ?? els.denoiseMode.value);
  els.paletteSource.value = String(
    result.plan.algoOverrides.paletteSource ?? els.paletteSource.value,
  );
  els.paletteCleanupMode.value = String(
    result.plan.algoOverrides.paletteCleanupMode ?? els.paletteCleanupMode.value,
  );
  els.cellColorMode.value = String(
    result.plan.algoOverrides.cellColorMode ?? els.cellColorMode.value,
  );
  els.ditherMode.value = String(result.plan.algoOverrides.dither ?? els.ditherMode.value);
  els.colorSpace.value = String(
    result.plan.algoOverrides.colorSpace ?? els.colorSpace.value,
  );
  els.cleanupMode.value = String(
    result.plan.algoOverrides.cleanupMode ?? els.cleanupMode.value,
  );
  els.repairMode.value = String(result.plan.algoOverrides.repairMode ?? els.repairMode.value);
  els.trimTransparent.checked = Boolean(result.plan.trimTransparent);
  activePresetKey = result.plan.keepPreset ? result.plan.presetKey : null;
  updatePresetStatus();
  updateUiFromSettings();
  writeSettings();
  setStatus(`Variante applicata: ${result.plan.label}`);
}

function renderVariantCompareResults(results) {
  els.variantCompareGrid.textContent = "";
  els.variantPreviewGrid.textContent = "";
  if (!Array.isArray(results) || results.length === 0) {
    els.variantComparePanel.classList.add("hidden");
    els.variantPreviewPanel.classList.add("hidden");
    els.variantPreviewStatus.textContent = "Nessuna variante generata.";
    return;
  }
  const scoreMap = buildVariantScoreMap(results);
  let recommendation = null;
  try {
    const payload = results.map((r) => ({
      key: String(r.plan.key),
      label: String(r.plan.label),
      diff_score: Number(r.diffScoreValue ?? 0),
      diff_area: Number(r.diffAreaValue ?? 0),
      palette_count: Number(r.debug?.palette_count ?? r.debug?.palette?.length ?? 0),
      aggressiveness: Number(r.aggressivenessValue ?? 0),
    }));
    recommendation = recommend_variant_from_metrics_json(JSON.stringify(payload));
  } catch (e) {
    recommendation = null;
  }

  recommendedVariantKey = recommendation ? String(recommendation.key ?? "") : null;
  if (recommendedVariantKey) {
    els.variantRecommendationLabel.textContent = `Scelta consigliata: ${String(
      recommendation.label ?? recommendedVariantKey,
    )}`;
    els.variantRecommendationReason.textContent = String(
      recommendation.reason ?? "offre il compromesso migliore tra fedelta' e pulizia",
    );
    els.variantRecommendation.classList.remove("hidden");
    els.applyRecommendedVariantBtn.disabled = false;
  } else {
    els.variantRecommendation.classList.add("hidden");
    els.applyRecommendedVariantBtn.disabled = true;
  }

  for (const result of results) {
    const score = scoreMap.get(result.plan.key);
    const card = document.createElement("div");
    card.className = "variantCard";

    const img = document.createElement("img");
    img.src = result.url;
    img.alt = `Variante ${result.plan.label}`;

    const title = document.createElement("div");
    title.className = "variantCardTitle";
    title.textContent = result.plan.label;

    const meta = document.createElement("div");
    meta.className = "variantCardMeta";
    meta.textContent =
      `${result.plan.summary} Repair ${result.debug?.config?.repair_mode ?? "smart"}, ` +
      `palette-fix ${result.debug?.config?.palette_cleanup_mode ?? "basic"}, ` +
      `trim ${result.plan.trimTransparent ? "on" : "off"}, diff ${result.diffScore}.`;

    const scoreRow = document.createElement("div");
    scoreRow.className = "variantScoreRow";
    if (score) {
      const makePill = (text, klass) => {
        const el = document.createElement("div");
        el.className = `variantScorePill ${klass}`;
        el.textContent = text;
        return el;
      };
      scoreRow.appendChild(makePill(score.fidelityText, score.fidelityClass));
      scoreRow.appendChild(makePill(score.cleanText, score.cleanClass));
      scoreRow.appendChild(makePill(score.aggressiveText, score.aggressiveClass));
    }

    const useBtn = document.createElement("button");
    useBtn.className = "secondary small";
    useBtn.textContent = "Usa questa";
    useBtn.addEventListener("click", async () => {
      try {
        await promoteVariantResult(result);
      } catch (e) {
        setStatus(`Errore variante: ${String(e)}`, true);
      }
    });

    card.appendChild(title);
    card.appendChild(img);
    card.appendChild(scoreRow);
    card.appendChild(meta);
    card.appendChild(useBtn);
    els.variantCompareGrid.appendChild(card);

    const largeCard = document.createElement("div");
    largeCard.className = "variantPreviewCard";

    const largeTitle = document.createElement("div");
    largeTitle.className = "variantCardTitle";
    largeTitle.textContent = result.plan.label;

    const largeImageFrame = document.createElement("div");
    largeImageFrame.className = "variantPreviewImage";
    const largeImg = document.createElement("img");
    largeImg.src = result.url;
    largeImg.alt = `Preview grande ${result.plan.label}`;
    largeImg.dataset.outputSrc = result.url;
    largeImg.dataset.diffSrc = result.diffUrl ?? result.url;
    largeImageFrame.appendChild(largeImg);

    const largeMeta = document.createElement("div");
    largeMeta.className = "variantCardMeta";
    largeMeta.textContent =
      `${result.plan.summary} Repair ${result.debug?.config?.repair_mode ?? "smart"}, ` +
      `palette-fix ${result.debug?.config?.palette_cleanup_mode ?? "basic"}, ` +
      `palette ${result.debug?.config?.palette_source ?? "cells"}, ` +
      `trim ${result.plan.trimTransparent ? "on" : "off"}, diff ${result.diffScore}, area ${result.diffArea}.`;

    const largeScoreRow = document.createElement("div");
    largeScoreRow.className = "variantScoreRow";
    if (score) {
      const makeLargePill = (text, klass) => {
        const el = document.createElement("div");
        el.className = `variantScorePill ${klass}`;
        el.textContent = text;
        return el;
      };
      largeScoreRow.appendChild(makeLargePill(score.fidelityText, score.fidelityClass));
      largeScoreRow.appendChild(makeLargePill(score.cleanText, score.cleanClass));
      largeScoreRow.appendChild(makeLargePill(score.aggressiveText, score.aggressiveClass));
    }

    const largeUseBtn = document.createElement("button");
    largeUseBtn.className = "secondary small";
    largeUseBtn.textContent = "Usa questa";
    largeUseBtn.addEventListener("click", async () => {
      try {
        await promoteVariantResult(result);
      } catch (e) {
        setStatus(`Errore variante: ${String(e)}`, true);
      }
    });

    largeCard.appendChild(largeTitle);
    largeCard.appendChild(largeImageFrame);
    largeCard.appendChild(largeScoreRow);
    largeCard.appendChild(largeMeta);
    largeCard.appendChild(largeUseBtn);
    els.variantPreviewGrid.appendChild(largeCard);
  }

  els.variantComparePanel.classList.remove("hidden");
  els.variantPreviewPanel.classList.remove("hidden");
  els.variantPreviewStatus.textContent =
    results.length >= 3
      ? "Confronto 3-up pronto: puoi passare da output a diff visivo."
      : "Confronto varianti pronto.";
  updateVariantPreviewMode();
}

function updateVariantPreviewMode() {
  const showDiff = Boolean(els.variantShowDiff.checked);
  document.querySelectorAll(".variantPreviewImage").forEach((frame) => {
    frame.classList.toggle("is-diff", showDiff);
    const img = frame.querySelector("img");
    if (!img) return;
    img.src = showDiff ? img.dataset.diffSrc || img.dataset.outputSrc || "" : img.dataset.outputSrc || "";
  });

  if (els.variantPreviewPanel.classList.contains("hidden")) {
    els.variantPreviewStatus.textContent = "Nessuna variante generata.";
    return;
  }
  els.variantPreviewStatus.textContent = showDiff
    ? "Diff visivo attivo: la heatmap evidenzia le aree piu' alterate rispetto all'input."
    : "Anteprime output attive: confronta i risultati puliti delle varianti.";
}

function buildVariantScoreMap(results) {
  const safeResults = Array.isArray(results) ? results : [];
  if (safeResults.length === 0) return new Map();

  const minDiff = Math.min(...safeResults.map((r) => Number(r.diffScoreValue ?? 0)));
  const maxDiff = Math.max(...safeResults.map((r) => Number(r.diffScoreValue ?? 0)));
  const minArea = Math.min(...safeResults.map((r) => Number(r.diffAreaValue ?? 0)));
  const maxArea = Math.max(...safeResults.map((r) => Number(r.diffAreaValue ?? 0)));
  const minPalette = Math.min(
    ...safeResults.map((r) => Number(r.debug?.palette_count ?? r.debug?.palette?.length ?? 0)),
  );
  const maxPalette = Math.max(
    ...safeResults.map((r) => Number(r.debug?.palette_count ?? r.debug?.palette?.length ?? 0)),
  );

  const normalize = (value, min, max) => {
    if (!Number.isFinite(value) || !Number.isFinite(min) || !Number.isFinite(max) || max <= min) {
      return 0.5;
    }
    return (value - min) / (max - min);
  };

  const fidelityWinner = safeResults.reduce((best, current) =>
    Number(current.diffScoreValue ?? Infinity) < Number(best.diffScoreValue ?? Infinity)
      ? current
      : best,
  );
  const cleanWinner = safeResults.reduce((best, current) => {
    const currentPalette = Number(current.debug?.palette_count ?? current.debug?.palette?.length ?? 0);
    const bestPalette = Number(best.debug?.palette_count ?? best.debug?.palette?.length ?? 0);
    return currentPalette < bestPalette ? current : best;
  });
  const aggressiveWinner = safeResults.reduce((best, current) =>
    Number(current.aggressivenessValue ?? -Infinity) > Number(best.aggressivenessValue ?? -Infinity)
      ? current
      : best,
  );

  const scoreMap = new Map();
  for (const result of safeResults) {
    const diffNorm = normalize(Number(result.diffScoreValue ?? 0), minDiff, maxDiff);
    const areaNorm = normalize(Number(result.diffAreaValue ?? 0), minArea, maxArea);
    const paletteNorm = normalize(
      Number(result.debug?.palette_count ?? result.debug?.palette?.length ?? 0),
      minPalette,
      maxPalette,
    );
    const fidelityValue = 1 - (diffNorm * 0.65 + areaNorm * 0.35);
    const cleanValue = (1 - paletteNorm) * 0.55 + (1 - diffNorm) * 0.2 + (1 - areaNorm) * 0.25;
    const aggressiveValue = Number(result.aggressivenessValue ?? 0);

    const toBand = (value, inverse = false) => {
      const v = inverse ? 1 - value : value;
      if (v >= 0.67) return "score-good";
      if (v >= 0.34) return "score-mid";
      return "score-risky";
    };

    scoreMap.set(result.plan.key, {
      fidelityValue,
      cleanValue,
      aggressiveValue,
      fidelityWinner: fidelityWinner?.plan?.key === result.plan.key,
      cleanWinner: cleanWinner?.plan?.key === result.plan.key,
      aggressiveWinner: aggressiveWinner?.plan?.key === result.plan.key,
      fidelityText:
        fidelityWinner?.plan?.key === result.plan.key ? "Piu' fedele" : `Fedelta': ${Math.round(fidelityValue * 100)}%`,
      fidelityClass: fidelityWinner?.plan?.key === result.plan.key ? "score-good" : toBand(fidelityValue),
      cleanText:
        cleanWinner?.plan?.key === result.plan.key ? "Piu' pulita" : `Pulizia: ${Math.round(cleanValue * 100)}%`,
      cleanClass: cleanWinner?.plan?.key === result.plan.key ? "score-good" : toBand(cleanValue),
      aggressiveText:
        aggressiveWinner?.plan?.key === result.plan.key
          ? "Piu' aggressiva"
          : `Aggressivita': ${Math.round(aggressiveValue * 100)}%`,
      aggressiveClass:
        aggressiveWinner?.plan?.key === result.plan.key
          ? "score-risky"
          : toBand(aggressiveValue, false),
    });
  }
  return scoreMap;
}

async function runVariantComparison() {
  if (!wasmReady) {
    throw new Error("WASM non pronto");
  }
  if (els.batchMode.checked) {
    throw new Error("Il confronto varianti rapide e' disponibile solo su file singolo");
  }
  const file = selectedFiles[0];
  if (!file) {
    throw new Error("Seleziona prima un'immagine");
  }
  if (!suggestedSetup) {
    throw new Error("Suggerimento setup non disponibile");
  }

  clearVariantCompare();
  setVariantCompareStatus("Generazione varianti rapide in corso...");
  els.runVariantCompareBtn.disabled = true;
  const ctx = await resolveSingleFileContext(file);
  const plans = buildVariantPlans();
  const results = [];

  for (const plan of plans) {
    const result = await processResolvedSingleFile(ctx, {
      includeDebug: true,
      algoOverrides: plan.algoOverrides,
      trimTransparentOverride: plan.trimTransparent,
    });
    const diff = await buildVariantDiffPng(
      ctx.inputBytes,
      selectedFiles[0]?.type || "image/png",
      result.bytes,
    );
    const blob = new Blob([result.bytes], { type: "image/png" });
    const url = URL.createObjectURL(blob);
    const diffBlob = new Blob([diff.bytes], { type: "image/png" });
    const diffUrl = URL.createObjectURL(diffBlob);
    results.push({
      ...result,
      url,
      diffUrl,
      diffScore: `${diff.score}`,
      diffScoreValue: Number(diff.score),
      diffArea: `${Math.round(diff.activeRatio * 100)}%`,
      diffAreaValue: Number(diff.activeRatio),
      aggressivenessValue: Math.min(
        1,
        (Number(plan.algoOverrides.repairMode ?? 0) / 3) * 0.55 +
          (Number(plan.algoOverrides.paletteCleanupMode ?? 0) / 2) * 0.25 +
          (Number(plan.algoOverrides.cleanupMode ?? 0) / 1) * 0.1 +
          (plan.trimTransparent ? 0.05 : 0) +
          (Number(plan.algoOverrides.denoise ?? 0) / 1) * 0.05,
      ),
      plan,
    });
  }

  variantCompareResults = results;
  renderVariantCompareResults(results);
  setVariantCompareStatus("Varianti pronte: confronta le anteprime e scegli quella migliore.");
  els.runVariantCompareBtn.disabled = false;
}

function setDropText() {
  if (els.batchMode.checked) {
    els.dropTitle.textContent = "Trascina qui più file";
    els.dropSub.textContent = "oppure clicca per selezionare (multi)";
  } else {
    els.dropTitle.textContent = "Trascina qui un file";
    els.dropSub.textContent = "oppure clicca per selezionare";
  }
}

async function boot() {
  try {
    setStatus("Inizializzazione WASM...");
    await init();
    wasmReady = true;
    updatePresetStatus();
    setStatus("Pronto. Seleziona un'immagine.");
    setProcessEnabled(selectedFiles.length > 0);
  } catch (e) {
    setStatus(`Errore inizializzazione: ${String(e)}`, true);
  }
}

els.pixelOverrideEnabled.addEventListener("change", () => {
  markPresetCustom();
  updateUiFromSettings();
  writeSettings();
});

function setSelectedFiles(files) {
  selectedFiles = Array.isArray(files) ? files : [];
  revokeBatchUrls();

  revokeUrl(inputUrl);
  revokeUrl(outputUrl);
  revokeUrl(debugUrl);
  revokeUrl(overlayUrl);
  inputUrl = null;
  outputUrl = null;
  debugUrl = null;
  overlayUrl = null;

  els.outputPreview.removeAttribute("src");
  setDownloadEnabled(false);
  setDebugDownloadEnabled(false);
  setOverlayDownloadEnabled(false);
  setDownloadAllEnabled(false);
  els.compareOutput.removeAttribute("src");
  els.compareInput.removeAttribute("src");
  clearBatchList();
  els.paletteSwatches.textContent = "";
  lastDebug = null;
  clearGridOverlay();
  clearDebugSummary();
  clearPresetSuggestion();
  clearVariantCompare();

  const first = selectedFiles[0] ?? null;
  if (!first) {
    els.inputPreview.removeAttribute("src");
    els.compareInputWrap.style.width = "50%";
    setStatus("Carica un'immagine per iniziare.");
    setProcessEnabled(false);
    return;
  }

  inputUrl = URL.createObjectURL(first);
  els.inputPreview.src = inputUrl;
  els.compareInput.src = inputUrl;
  setStatus(
    els.batchMode.checked
      ? `${selectedFiles.length} file selezionati. Premi Elabora.`
      : "Immagine selezionata. Premi Elabora.",
  );
  setProcessEnabled(wasmReady);
  setDropText();
  void refreshPresetSuggestion(first);
}

els.fileInput.addEventListener("change", () => {
  const files = Array.from(els.fileInput.files ?? []);
  setSelectedFiles(files);
});

els.dropZone.addEventListener("click", () => {
  els.fileInput.click();
});

els.dropZone.addEventListener("keydown", (e) => {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    els.fileInput.click();
  }
});

els.dropZone.addEventListener("dragover", (e) => {
  e.preventDefault();
  els.dropZone.classList.add("dragover");
});

els.dropZone.addEventListener("dragleave", () => {
  els.dropZone.classList.remove("dragover");
});

els.dropZone.addEventListener("drop", (e) => {
  e.preventDefault();
  els.dropZone.classList.remove("dragover");
  const files = Array.from(e.dataTransfer?.files ?? []);
  if (files.length > 0) setSelectedFiles(files);
});

els.zoom.addEventListener("input", () => {
  applyZoom();
  renderInputGrid();
  writeSettings();
});

els.compareSlider.addEventListener("input", () => {
  updateCompare();
});

els.compareFrame.addEventListener("pointerdown", (e) => {
  if (e.button !== 0) return;
  compareDragging = true;
  els.compareFrame.setPointerCapture(e.pointerId);
  setComparePct(comparePctFromClientX(e.clientX));
});

els.compareFrame.addEventListener("pointermove", (e) => {
  if (!compareDragging) return;
  setComparePct(comparePctFromClientX(e.clientX));
});

els.compareFrame.addEventListener("pointerup", (e) => {
  compareDragging = false;
  try {
    els.compareFrame.releasePointerCapture(e.pointerId);
  } catch {}
});

els.compareReset.addEventListener("click", () => {
  setComparePct(50);
});

function startBlink() {
  if (compareBlinkActive) return;
  compareBlinkActive = true;
  compareBlinkPrev = Number.parseInt(els.compareSlider.value, 10);
  if (!Number.isFinite(compareBlinkPrev)) compareBlinkPrev = 50;
  setComparePct(0);
}

function stopBlink() {
  if (!compareBlinkActive) return;
  compareBlinkActive = false;
  setComparePct(compareBlinkPrev);
}

els.compareBlink.addEventListener("pointerdown", (e) => {
  e.preventDefault();
  startBlink();
});

els.compareBlink.addEventListener("pointerup", () => {
  stopBlink();
});

els.compareBlink.addEventListener("pointerleave", () => {
  stopBlink();
});

els.compareBlink.addEventListener("blur", () => {
  stopBlink();
});

els.openOptions.addEventListener("click", () => {
  openDrawer();
});

els.closeOptions.addEventListener("click", () => {
  closeDrawer();
});

els.drawerOverlay.addEventListener("click", () => {
  closeDrawer();
});

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !els.drawer.classList.contains("hidden")) {
    closeDrawer();
    return;
  }

  if ((e.key === "b" || e.key === "B" || e.key === " ") && !e.repeat) {
    const tag = document.activeElement?.tagName?.toLowerCase();
    if (tag === "input" || tag === "textarea" || tag === "select") return;
    startBlink();
  }
});

window.addEventListener("keyup", (e) => {
  if (e.key === "b" || e.key === "B" || e.key === " ") {
    stopBlink();
  }
});

els.batchMode.addEventListener("change", () => {
  updateUiFromSettings();
  setDropText();
  writeSettings();
});

els.denoiseMode.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.paletteSource.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.paletteLockEnabled.addEventListener("change", () => {
  writeSettings();
});

els.paletteLockFile.addEventListener("change", () => {
  writeSettings();
});

els.paletteCleanupMode.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.cellColorMode.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.ditherMode.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.colorSpace.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.cleanupMode.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.repairMode.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.trimTransparent.addEventListener("change", () => {
  writeSettings();
});

els.scaleFactor.addEventListener("change", () => {
  writeSettings();
});

els.showPalette.addEventListener("change", () => {
  writeSettings();
});

els.showGrid.addEventListener("change", () => {
  writeSettings();
  renderInputGrid();
});

document.querySelectorAll('input[name="downloadMode"]').forEach((r) => {
  r.addEventListener("change", () => {
    updateUiFromSettings();
    writeSettings();
  });
});

document.querySelectorAll(".presetBtn").forEach((b) => {
  b.addEventListener("click", async () => {
    const preset = b.getAttribute("data-preset");
    if (!preset) return;
    try {
      await applyPreset(preset);
    } catch (e) {
      setStatus(`Errore preset: ${String(e)}`, true);
    }
  });
});

els.applySuggestedSetupBtn.addEventListener("click", async () => {
  if (!suggestedSetup) return;
  try {
    await applySuggestedSetupConfig(suggestedSetup);
  } catch (e) {
    setStatus(`Errore setup consigliato: ${String(e)}`, true);
  }
});

[els.setupApplyPalette, els.setupApplyTrim, els.setupForceUltra].forEach((el) => {
  el.addEventListener("change", () => {
    updateSuggestedSetupSummary();
  });
});

document.querySelectorAll(".setupProfileBtn").forEach((btn) => {
  btn.addEventListener("click", () => {
    const profileName = btn.getAttribute("data-setup-profile");
    if (!profileName) return;
    applySetupProfile(profileName);
    updateSuggestedSetupSummary();
  });
});

els.applySuggestedPresetBtn.addEventListener("click", async () => {
  if (!suggestedPresetKey) return;
  try {
    await applyPreset(suggestedPresetKey);
  } catch (e) {
    setStatus(`Errore preset suggerito: ${String(e)}`, true);
  }
});

els.runVariantCompareBtn.addEventListener("click", async () => {
  try {
    await runVariantComparison();
  } catch (e) {
    els.runVariantCompareBtn.disabled = false;
    setVariantCompareStatus(String(e), true);
  }
});

els.applyRecommendedVariantBtn.addEventListener("click", async () => {
  try {
    const result = variantCompareResults.find((entry) => entry.plan.key === recommendedVariantKey);
    if (!result) {
      throw new Error("Variante consigliata non disponibile");
    }
    await promoteVariantResult(result);
  } catch (e) {
    setStatus(`Errore variante consigliata: ${String(e)}`, true);
  }
});

els.variantShowDiff.addEventListener("change", () => {
  updateVariantPreviewMode();
});

els.kColors.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.pixelSize.addEventListener("change", () => {
  markPresetCustom();
  writeSettings();
});

els.processBtn.addEventListener("click", async () => {
  if (!wasmReady) {
    setStatus("WASM non pronto.", true);
    return;
  }
  if (selectedFiles.length === 0) {
    setStatus("Seleziona un'immagine.", true);
    return;
  }

  try {
    setProcessEnabled(false);
    setLoading(true);
    setStatus("Elaborazione in corso...");

    revokeBatchUrls();
    clearBatchList();

    if (!els.batchMode.checked) {
      const file = selectedFiles[0];
      const result = await processSingleFileWithDebugIfNeeded(file);
      const processedBytes = result.bytes;
      lastDebug = result.debug;
      updateDebugSummary(lastDebug);

      revokeUrl(outputUrl);
      const blob = new Blob([processedBytes], { type: "image/png" });
      outputUrl = URL.createObjectURL(blob);

      els.outputPreview.src = outputUrl;
      els.compareOutput.src = outputUrl;
      els.downloadBtn.href = outputUrl;
      setDownloadEnabled(true);
      setDownloadAllEnabled(true);
      revokeUrl(debugUrl);
      revokeUrl(overlayUrl);
      debugUrl = null;
      overlayUrl = null;
      if (lastDebug) {
        const debugBlob = new Blob([JSON.stringify(lastDebug, null, 2)], {
          type: "application/json",
        });
        debugUrl = URL.createObjectURL(debugBlob);
        els.downloadDebugBtn.href = debugUrl;
        setDebugDownloadEnabled(true);
        if (result.overlayBytes && result.overlayBytes.length > 0) {
          const overlayBlob = new Blob([result.overlayBytes], { type: "image/png" });
          overlayUrl = URL.createObjectURL(overlayBlob);
          els.downloadOverlayBtn.href = overlayUrl;
          setOverlayDownloadEnabled(true);
        } else {
          setOverlayDownloadEnabled(false);
        }
      } else {
        setDebugDownloadEnabled(false);
        setOverlayDownloadEnabled(false);
      }
      setStatus("Fatto.");
      updateCompare();
      applyZoom();

      const palette =
        Array.isArray(lastDebug?.palette) && lastDebug.palette.length > 0
          ? lastDebug.palette
          : await computePaletteFromPng(processedBytes, 64);
      renderSwatches(palette);
      renderInputGrid();
      return;
    }

    const items = selectedFiles.map((f) => ({ file: f, ui: addBatchItem(f.name) }));
    for (const it of items) {
      it.ui.statusEl.textContent = "processing...";
      try {
        const result = await processSingleFile(it.file);
        const processedBytes = result.bytes;
        const blob = new Blob([processedBytes], { type: "image/png" });
        const url = URL.createObjectURL(blob);
        batchResults.push({ name: it.file.name, bytes: processedBytes, url });
        it.ui.statusEl.textContent = "ok";
      } catch (e) {
        it.ui.statusEl.textContent = "errore";
      }
    }

    lastDebug = null;
    clearDebugSummary();
    revokeUrl(debugUrl);
    revokeUrl(overlayUrl);
    debugUrl = null;
    overlayUrl = null;
    setDebugDownloadEnabled(false);
    setOverlayDownloadEnabled(false);

    const firstOk = batchResults[0] ?? null;
    if (firstOk) {
      revokeUrl(outputUrl);
      outputUrl = firstOk.url;
      els.outputPreview.src = outputUrl;
      els.compareOutput.src = outputUrl;
      els.downloadBtn.href = outputUrl;
      setDownloadEnabled(true);
      setDownloadAllEnabled(batchResults.length > 0);
      const palette = await computePaletteFromPng(firstOk.bytes, 64);
      renderSwatches(palette);
    }

    setStatus(`Batch completato (${batchResults.length}/${selectedFiles.length}).`);
    updateCompare();
    applyZoom();
    renderInputGrid();
  } catch (e) {
    setStatus(`Errore elaborazione: ${String(e)}`, true);
  } finally {
    setLoading(false);
    setProcessEnabled(selectedFiles.length > 0);
  }
});

els.downloadAllBtn.addEventListener("click", async () => {
  if (!els.batchMode.checked) {
    if (!outputUrl) return;
    const a = document.createElement("a");
    a.href = outputUrl;
    a.download = "nasty-retropixel.png";
    document.body.appendChild(a);
    a.click();
    a.remove();
    return;
  }

  if (batchResults.length === 0) return;

  if (getDownloadMode() === "zip") {
    try {
      els.zipStatus.textContent = "ZIP: generazione...";
      const zipBytes = buildZipStore(
        batchResults.map((r) => ({ name: toZipEntryName(r.name), bytes: r.bytes })),
      );
      const blob = new Blob([zipBytes], { type: "application/zip" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "nasty-retropixel.zip";
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 5000);
      els.zipStatus.textContent = "ZIP: pronto.";
      return;
    } catch (e) {
      els.zipStatus.textContent = `ZIP: errore (${String(e)}), fallback singoli file.`;
    }
  }

  for (const r of batchResults) {
    const a = document.createElement("a");
    a.href = r.url;
    a.download = r.name.replace(/\.[^/.]+$/, "") + "_nasty-retropixel.png";
    document.body.appendChild(a);
    a.click();
    a.remove();
    await new Promise((res) => setTimeout(res, 150));
  }
});

boot();
updateCompare();
applyZoom();
applySettings(readSettings());
updatePresetStatus();
updateUiFromSettings();
setDropText();
renderInputGrid();

els.inputPreview.addEventListener("load", () => {
  renderInputGrid();
});

window.addEventListener("resize", () => {
  renderInputGrid();
});

window.addEventListener("beforeunload", () => {
  revokeUrl(inputUrl);
  revokeUrl(outputUrl);
  revokeUrl(debugUrl);
  revokeUrl(overlayUrl);
  for (const r of batchResults) revokeUrl(r.url);
});
