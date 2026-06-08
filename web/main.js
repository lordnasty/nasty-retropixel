import init, { process_image, process_image_debug } from "./pkg/nasty_retropixel.js";

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
  downloadAllBtn: document.getElementById("downloadAllBtn"),
  status: document.getElementById("status"),
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
  drawer: document.getElementById("drawer"),
  drawerOverlay: document.getElementById("drawerOverlay"),
  batchMode: document.getElementById("batchMode"),
  denoiseMode: document.getElementById("denoiseMode"),
  paletteSource: document.getElementById("paletteSource"),
  ditherMode: document.getElementById("ditherMode"),
  colorSpace: document.getElementById("colorSpace"),
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
  "downloadAllBtn",
  "status",
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
  "drawer",
  "drawerOverlay",
  "batchMode",
  "denoiseMode",
  "paletteSource",
  "ditherMode",
  "colorSpace",
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
let batchResults = [];
let lastDebug = null;
let compareDragging = false;
let compareBlinkActive = false;
let compareBlinkPrev = 50;

const STORAGE_KEY = "nasty-retropixel-settings-v1";

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

function setDownloadEnabled(enabled) {
  els.downloadBtn.classList.toggle("disabled", !enabled);
  if (!enabled) els.downloadBtn.removeAttribute("href");
}

function setDownloadAllEnabled(enabled) {
  els.downloadAllBtn.classList.toggle("disabled", !enabled);
  els.downloadAllBtn.disabled = !enabled;
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
      kColors: els.kColors.value,
      pixelOverrideEnabled: els.pixelOverrideEnabled.checked,
      pixelSize: els.pixelSize.value,
      zoom: els.zoom.value,
      batchMode: els.batchMode.checked,
      denoiseMode: els.denoiseMode.value,
      paletteSource: els.paletteSource.value,
      ditherMode: els.ditherMode.value,
      colorSpace: els.colorSpace.value,
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
  if (settings.kColors != null) els.kColors.value = String(settings.kColors);
  if (settings.pixelOverrideEnabled != null)
    els.pixelOverrideEnabled.checked = Boolean(settings.pixelOverrideEnabled);
  if (settings.pixelSize != null) els.pixelSize.value = String(settings.pixelSize);
  if (settings.zoom != null) els.zoom.value = String(settings.zoom);
  if (settings.batchMode != null) els.batchMode.checked = Boolean(settings.batchMode);
  if (settings.denoiseMode != null) els.denoiseMode.value = String(settings.denoiseMode);
  if (settings.paletteSource != null) els.paletteSource.value = String(settings.paletteSource);
  if (settings.ditherMode != null) els.ditherMode.value = String(settings.ditherMode);
  if (settings.colorSpace != null) els.colorSpace.value = String(settings.colorSpace);
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
  const dither = Number.parseInt(els.ditherMode.value, 10);
  const colorSpace = Number.parseInt(els.colorSpace.value, 10);
  return {
    denoise: Number.isFinite(denoise) ? denoise : undefined,
    paletteSource: Number.isFinite(paletteSource) ? paletteSource : undefined,
    dither: Number.isFinite(dither) ? dither : undefined,
    colorSpace: Number.isFinite(colorSpace) ? colorSpace : undefined,
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
  if (els.trimTransparent.checked) out = trimCanvasTransparent(out);
  out = scaleCanvasNearest(out, els.scaleFactor.value);
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
    el.style.background = c;
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
  const algo = getAlgoOptions();
  const rawBytes = process_image(
    inputBytes,
    k,
    pixelOverride,
    algo.denoise,
    algo.paletteSource,
    algo.dither,
    algo.colorSpace,
  );
  return { bytes: await postProcessPng(rawBytes), debug: null };
}

async function processSingleFileWithDebugIfNeeded(file) {
  if (!els.showGrid.checked || els.batchMode.checked) return processSingleFile(file);

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
  const algo = getAlgoOptions();
  const dbg = process_image_debug(
    inputBytes,
    k,
    pixelOverride,
    algo.denoise,
    algo.paletteSource,
    algo.dither,
    algo.colorSpace,
  );
  const bytes = await postProcessPng(dbg.bytes);
  const col = Array.from(dbg.col_cuts ?? []);
  const row = Array.from(dbg.row_cuts ?? []);
  return {
    bytes,
    debug: {
      col_cuts: col,
      row_cuts: row,
      input_width: Number(dbg.input_width ?? 0),
      input_height: Number(dbg.input_height ?? 0),
      step_x: Number(dbg.step_x ?? 0),
      step_y: Number(dbg.step_y ?? 0),
    },
  };
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
    setStatus("Pronto. Seleziona un'immagine.");
    setProcessEnabled(selectedFiles.length > 0);
  } catch (e) {
    setStatus(`Errore inizializzazione: ${String(e)}`, true);
  }
}

els.pixelOverrideEnabled.addEventListener("change", () => {
  updateUiFromSettings();
  writeSettings();
});

function setSelectedFiles(files) {
  selectedFiles = Array.isArray(files) ? files : [];
  revokeBatchUrls();

  revokeUrl(inputUrl);
  revokeUrl(outputUrl);
  inputUrl = null;
  outputUrl = null;

  els.outputPreview.removeAttribute("src");
  setDownloadEnabled(false);
  setDownloadAllEnabled(false);
  els.compareOutput.removeAttribute("src");
  els.compareInput.removeAttribute("src");
  clearBatchList();
  els.paletteSwatches.textContent = "";
  lastDebug = null;
  clearGridOverlay();

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
  writeSettings();
});

els.paletteSource.addEventListener("change", () => {
  writeSettings();
});

els.ditherMode.addEventListener("change", () => {
  writeSettings();
});

els.colorSpace.addEventListener("change", () => {
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
  b.addEventListener("click", () => {
    const k = b.getAttribute("data-k");
    if (k) els.kColors.value = k;
    writeSettings();
  });
});

els.kColors.addEventListener("change", () => {
  writeSettings();
});

els.pixelSize.addEventListener("change", () => {
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

      revokeUrl(outputUrl);
      const blob = new Blob([processedBytes], { type: "image/png" });
      outputUrl = URL.createObjectURL(blob);

      els.outputPreview.src = outputUrl;
      els.compareOutput.src = outputUrl;
      els.downloadBtn.href = outputUrl;
      setDownloadEnabled(true);
      setDownloadAllEnabled(true);
      setStatus("Fatto.");
      updateCompare();
      applyZoom();

      const palette = await computePaletteFromPng(processedBytes, 64);
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
  for (const r of batchResults) revokeUrl(r.url);
});
