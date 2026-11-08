/* Island — frontend. Sem build: JS puro.
 * No app (Tauri) recebe eventos do Rust; aberto no navegador roda em modo demo. */
(() => {
  "use strict";

  const T = window.__TAURI__;
  const DEMO = !T;
  const invoke = (cmd, args) => (T ? T.core.invoke(cmd, args) : Promise.resolve(demoInvoke(cmd, args)));
  const call = (cmd, args) => invoke(cmd, args).catch((e) => { console.warn(cmd, e); throw e; });
  const fire = (cmd, args) => { invoke(cmd, args).catch(() => {}); };
  const listen = (ev, fn) => T && T.event.listen(ev, (e) => fn(e.payload));

  const $ = (s, root = document) => root.querySelector(s);
  const $$ = (s, root = document) => [...root.querySelectorAll(s)];
  const stage = $("#stage");
  const pill = $("#pill");
  const aura = $("#aura");

  // ------------------------------------------------------------ ícones
  const svg = (d, extra = "") =>
    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" ${extra}>${d}</svg>`;
  const fillSvg = (d) => `<svg viewBox="0 0 24 24" fill="currentColor">${d}</svg>`;
  const I = {
    music: svg('<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>'),
    gauge: svg('<path d="M3 13a9 9 0 0 1 18 0"/><path d="M12 13l4-4"/><path d="M5 17h14"/>'),
    timer: svg('<circle cx="12" cy="13" r="8"/><path d="M12 9v4l2.5 2"/><path d="M9 2h6"/>'),
    clip: svg('<rect x="8" y="3" width="8" height="4" rx="1.5"/><path d="M16 5h1.5A1.5 1.5 0 0 1 19 6.5v13a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19.5v-13A1.5 1.5 0 0 1 6.5 5H8"/>'),
    bell: svg('<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>'),
    tray: svg('<path d="M3 13l2.5-7h13L21 13"/><path d="M3 13v5a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5h-5.5a3.5 3.5 0 0 1-7 0z"/>'),
    play: fillSvg('<path d="M8 5.5v13a1 1 0 0 0 1.5.86l10.4-6.5a1 1 0 0 0 0-1.72L9.5 4.64A1 1 0 0 0 8 5.5z"/>'),
    pause: fillSvg('<rect x="6.5" y="5" width="4" height="14" rx="1.2"/><rect x="13.5" y="5" width="4" height="14" rx="1.2"/>'),
    next: fillSvg('<path d="M4 6.5v11a.8.8 0 0 0 1.2.7L13 13v4.5a.8.8 0 0 0 1.2.7l8.3-5.5a.8.8 0 0 0 0-1.4L14.2 5.8a.8.8 0 0 0-1.2.7V11L5.2 5.8a.8.8 0 0 0-1.2.7z"/>'),
    prev: fillSvg('<path d="M20 6.5v11a.8.8 0 0 1-1.2.7L11 13v4.5a.8.8 0 0 1-1.2.7L1.5 12.7a.8.8 0 0 1 0-1.4L9.8 5.8a.8.8 0 0 1 1.2.7V11l7.8-5.2a.8.8 0 0 1 1.2.7z"/>'),
    vol0: svg('<path d="M11 5L6 9H3v6h3l5 4z" fill="currentColor"/><path d="M22 9l-6 6M16 9l6 6"/>'),
    vol1: svg('<path d="M11 5L6 9H3v6h3l5 4z" fill="currentColor"/><path d="M15.5 9.5a3.5 3.5 0 0 1 0 5"/>'),
    vol2: svg('<path d="M11 5L6 9H3v6h3l5 4z" fill="currentColor"/><path d="M15.5 9.5a3.5 3.5 0 0 1 0 5"/><path d="M18.5 6.5a7.5 7.5 0 0 1 0 11"/>'),
    bolt: fillSvg('<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>'),
    leaf: svg('<path d="M5 19c0-8 6-14 15-14 0 9-6 15-14 15"/><path d="M5 19l7-7"/>'),
    rocket: svg('<path d="M5 15c-1 1-1.5 4-1.5 4.5S6 19 7 18"/><path d="M9 15l-3-3 2-4c3-4 7-5 12-5 0 5-1 9-5 12l-4 2z"/><circle cx="15" cy="9" r="1.5"/>'),
    scale: svg('<path d="M12 3v18"/><path d="M5 7h14"/><path d="M5 7l-3 7a3.5 3.5 0 0 0 6 0z"/><path d="M19 7l-3 7a3.5 3.5 0 0 0 6 0z"/>'),
    wifi: svg('<path d="M2 8.5a15 15 0 0 1 20 0"/><path d="M5 12a10 10 0 0 1 14 0"/><path d="M8.5 15.5a5 5 0 0 1 7 0"/><circle cx="12" cy="19" r="1" fill="currentColor"/>'),
    wifiOff: svg('<path d="M2 2l20 20"/><path d="M8.5 15.5a5 5 0 0 1 7 0"/><path d="M5 12a10 10 0 0 1 5-2.7"/><path d="M2 8.5a15 15 0 0 1 4.2-2.8"/><circle cx="12" cy="19" r="1" fill="currentColor"/>'),
    lan: svg('<rect x="9" y="3" width="6" height="5" rx="1"/><rect x="3" y="16" width="6" height="5" rx="1"/><rect x="15" y="16" width="6" height="5" rx="1"/><path d="M12 8v4M6 16v-4h12v4"/>'),
    lock: svg('<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>'),
    caps: svg('<path d="M12 4l7 8h-4v4H9v-4H5z"/><path d="M9 20h6"/>'),
    check: svg('<path d="M5 12.5l4.5 4.5L19 7.5"/>'),
    ext: svg('<path d="M14 4h6v6"/><path d="M20 4l-9 9"/><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>'),
    refresh: svg('<path d="M20 11a8 8 0 1 0-2.3 5.7"/><path d="M20 4v7h-7"/>'),
    gear: svg('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>'),
    battery: (p, charging) => {
      const w = Math.max(1.5, (p / 100) * 14);
      const c = charging ? "#30d158" : p <= 20 ? "#ff453a" : "currentColor";
      return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2" y="7" width="18" height="10" rx="3" opacity=".55"/><rect x="4" y="9" width="${w}" height="6" rx="1.5" fill="${c}" stroke="none"/><path d="M22 10.5v3" stroke-linecap="round" opacity=".55"/></svg>`;
    },
    signal: (q) => {
      const lv = q >= 75 ? 3 : q >= 45 ? 2 : q >= 20 ? 1 : 0;
      const arc = (d, on) => `<path d="${d}" opacity="${on ? 1 : 0.25}"/>`;
      return `<svg class="sig" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">${arc("M2 8.5a15 15 0 0 1 20 0", lv >= 3)}${arc("M5 12a10 10 0 0 1 14 0", lv >= 2)}${arc("M8.5 15.5a5 5 0 0 1 7 0", lv >= 1)}<circle cx="12" cy="19" r="1.2" fill="currentColor" stroke="none"/></svg>`;
    },
  };

  // ------------------------------------------------------------ utilidades
  const load = (k, d) => { try { return JSON.parse(localStorage.getItem(k)) ?? d; } catch { return d; } };
  const save = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch {} };
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const fmt = (s) => { s = Math.max(0, Math.floor(s || 0)); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`; };
  const GB = (b) => (b / 1024 ** 3).toFixed(1);
  const MB = (b) => (b >= 1024 ** 3 ? `${GB(b)} GB` : `${Math.round(b / 1024 ** 2)} MB`);
  const rate = (b) => (b < 1024 ** 2 ? `${Math.round(b / 1024)} KB/s` : `${(b / 1024 ** 2).toFixed(1)} MB/s`);
  const dur = (sec) => { const h = Math.floor(sec / 3600), m = Math.round((sec % 3600) / 60); return h ? `${h} h ${m} min` : `${m} min`; };

  // ------------------------------------------------------------ estado
  const S = {
    hover: false,
    drag: false,
    media: undefined,
    mediaAt: 0,
    art: "",
    artReal: false,
    color: [255, 255, 255],
    fast: null,
    slow: null,
    hud: null,
    tab: load("island.tab", "media"),
    view: null, // painel aberto: cpu | ram | gpu | net | wifi | battery | clock
    clips: load("island.clips", []),
    notifs: load("island.notifs", []),
    nstatus: null,
    shelf: load("island.shelf", []),
    cpuHist: [],
    netHist: [],
    pomo: { phase: "focus", focus: 25, brk: 5, running: false, endAt: 0, left: 25 * 60, cycles: 0 },
    wifi: null,
    power: null,
  };

  // ------------------------------------------------------------ geometria + molas
  const STAGE_W = 720, TOP = 6;
  const dpr = () => window.devicePixelRatio || 1;
  // arredonda no pixel físico: evita bordas borradas em 125%/150%
  const snap = (v) => Math.round(v * dpr()) / dpr();

  class Spring {
    constructor(v, k, c) { this.v = v; this.t = v; this.vel = 0; this.k = k; this.c = c; }
    step(dt) {
      const f = -this.k * (this.v - this.t) - this.c * this.vel;
      this.vel += f * dt;
      this.v += this.vel * dt;
    }
    get idle() { return Math.abs(this.v - this.t) < 0.05 && Math.abs(this.vel) < 0.05; }
  }
  const sw = new Spring(128, 340, 24);
  const sh = new Spring(30, 300, 26);
  const sr = new Spring(15, 300, 26);
  let raf = 0, lastT = 0;

  const SIZES = {
    idle: [128, 30, 15],
    media: [280, 32, 16],
    timer: [260, 32, 16],
    volume: [300, 38, 19],
    toast: [340, 54, 27],
    expanded: [480, 200, 34],
  };
  const PANEL_SIZES = {
    cpu: [560, 300, 34], ram: [560, 300, 34], gpu: [560, 300, 34], net: [560, 300, 34],
    wifi: [420, 344, 34], battery: [440, 300, 34], clock: [420, 412, 36],
  };
  const targetSize = (m) => (m === "expanded" && S.view ? PANEL_SIZES[S.view] : SIZES[m]);

  function applyGeometry(w, h, r) {
    const W = snap(w), H = snap(h);
    stage.style.setProperty("--w", W + "px");
    stage.style.setProperty("--h", H + "px");
    stage.style.setProperty("--r", Math.max(0, Math.min(r, H / 2)).toFixed(2) + "px");
    stage.style.setProperty("--x", snap((STAGE_W - W) / 2) + "px");
  }

  function frame(t) {
    const dt = Math.min(0.05, (t - lastT) / 1000 || 0.016);
    lastT = t;
    const steps = Math.ceil(dt / (1 / 240));
    for (let i = 0; i < steps; i++) { sw.step(dt / steps); sh.step(dt / steps); sr.step(dt / steps); }
    if (sw.idle && sh.idle && sr.idle) {
      sw.v = sw.t; sh.v = sh.t; sr.v = sr.t;
      applyGeometry(sw.v, sh.v, sr.v);
      raf = 0;
      reportRect(sw.t, sh.t);
      return;
    }
    applyGeometry(sw.v, sh.v, sr.v);
    raf = requestAnimationFrame(frame);
  }

  function animateTo(w, h, r) {
    reportRect(Math.max(w, sw.v), Math.max(h, sh.v));
    sw.t = w; sh.t = h; sr.t = r;
    if (!raf) { lastT = performance.now(); raf = requestAnimationFrame(frame); }
  }

  let lastRect = "";
  function reportRect(w, h) {
    const r = { x: (STAGE_W - w) / 2, y: TOP, w, h };
    const key = Object.values(r).map((n) => n.toFixed(0)).join(",");
    if (key === lastRect) return;
    lastRect = key;
    fire("set_hit_rect", r);
  }

  // ------------------------------------------------------------ modo / render
  let mode = "idle", sizeKey = "";
  function computeMode() {
    if (S.hover || S.drag) return "expanded";
    if (S.hud) return S.hud.kind;
    if (S.pomo.running) return "timer";
    if (S.media && S.media.playing) return "media";
    return "idle";
  }
  const panel = () => (S.drag ? "shelf" : S.view ? (["cpu", "ram", "gpu", "net"].includes(S.view) ? "detail" : S.view) : S.tab);

  function render() {
    const m = computeMode();
    const [w, h, r] = targetSize(m);
    const key = `${m}:${w}x${h}`;
    if (m === "expanded") {
      const ex = $(".js-expanded");
      ex.style.setProperty("--lw", w + "px");
      ex.style.setProperty("--lh", h + "px");
    }
    if (key !== sizeKey) { sizeKey = key; animateTo(w, h, r); }
    mode = m;
    pill.dataset.mode = m;
    pill.dataset.panel = panel();
    pill.classList.toggle("in-panel", !!S.view && !S.drag);
    pill.classList.toggle("drag", S.drag);
    pill.classList.toggle("playing", !!(S.media && S.media.playing));
    pill.classList.toggle("has-media", !!S.media);
    pill.classList.toggle("no-media", !S.media);
    pill.classList.toggle("break", S.pomo.phase === "break");
    const glowOn =
      !!S.media && (m === "media" || (m === "expanded" && pill.dataset.panel === "media") || (m === "toast" && S.hud?.glow));
    aura.classList.toggle("on", glowOn);
    ensureSpin();
    $$(".st").forEach((b) => b.classList.remove("on"));
    if (S.view === "wifi") $(".js-open-wifi").classList.add("on");
    if (S.view === "battery") $(".js-open-batt").classList.add("on");
    if (S.view === "clock") $(".js-open-clock").classList.add("on");
  }

  // ------------------------------------------------------------ hover
  let hoverTimer = 0;
  function setHover(v) {
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => {
      if (S.hover === v) return;
      S.hover = v;
      if (v) {
        S.hud = null;
        clearTimeout(hudTimer);
        refreshExpanded();
      } else {
        openView(null);
      }
      render();
    }, v ? 90 : 320);
  }
  pill.addEventListener("mouseenter", () => setHover(true));
  pill.addEventListener("mouseleave", () => setHover(false));
  listen("island:hover", (v) => setHover(v));

  // ------------------------------------------------------------ HUD / toasts
  let hudTimer = 0;
  function hud(kind, ms, data = {}) {
    if (S.hover || S.drag) return;
    S.hud = { kind, ...data };
    if (kind === "toast") {
      $(".js-t-ico").innerHTML = data.img ? `<img src="${data.img}" alt="">` : data.icon || "";
      $(".js-t-ico").style.background = data.iconBg || "";
      $(".js-t-ico").style.color = data.iconColor || "";
      $(".js-t-title").textContent = data.title || "";
      $(".js-t-sub").textContent = data.sub || "";
      $(".js-t-acc").innerHTML = data.acc || "";
    }
    clearTimeout(hudTimer);
    hudTimer = setTimeout(() => { S.hud = null; render(); }, ms);
    render();
  }

  // ------------------------------------------------------------ abas e painéis
  const TABS = [
    ["media", I.music, "Mídia"],
    ["system", I.gauge, "Sistema"],
    ["focus", I.timer, "Foco"],
    ["clip", I.clip, "Clipboard"],
    ["notif", I.bell, "Notificações"],
    ["shelf", I.tray, "Estante"],
  ];
  const VIEW_TITLES = { cpu: "CPU", ram: "Memória", gpu: "GPU", net: "Rede", wifi: "Wi-Fi", battery: "Bateria", clock: "Relógio" };
  $(".js-tabs").innerHTML = TABS.map(([id, ic, t]) => `<button class="tab" data-id="${id}" title="${t}">${ic}</button>`).join("");
  $$(".tab").forEach((b) =>
    b.addEventListener("click", () => { S.tab = b.dataset.id; save("island.tab", S.tab); refreshExpanded(); render(); })
  );
  $(".js-back").addEventListener("click", () => {
    // detalhe de rede aberto pelo Wi-Fi volta para o Wi-Fi; o resto volta às abas
    openView(null);
    render();
  });

  let clockRaf = 0, wifiTimer = 0, powerTimer = 0;
  function openView(v) {
    if (S.view === v) v = null; // clicar de novo fecha
    S.view = v;
    $(".js-x-title").textContent = v ? VIEW_TITLES[v] : "";
    fire("set_detail", { kind: ["cpu", "ram", "gpu", "net"].includes(v) ? v : null });
    cancelAnimationFrame(clockRaf); clockRaf = 0;
    clearInterval(wifiTimer); clearInterval(powerTimer);
    if (["cpu", "ram", "gpu", "net"].includes(v)) renderDetail();
    if (v === "clock") { calMonth = null; renderCalendar(); clockLoop(); }
    if (v === "wifi") { loadWifi(); wifiTimer = setInterval(loadWifi, 5000); }
    if (v === "battery") { loadPower(); powerTimer = setInterval(loadPower, 4000); }
    render();
  }
  $(".js-open-wifi").addEventListener("click", () => openView("wifi"));
  $(".js-open-batt").addEventListener("click", () => openView("battery"));
  $(".js-open-clock").addEventListener("click", () => openView("clock"));
  $$(".tile").forEach((t) => t.addEventListener("click", () => openView(t.dataset.detail)));

  function refreshExpanded() {
    $$(".tab").forEach((b) => b.classList.toggle("on", b.dataset.id === (S.drag ? "shelf" : S.tab)));
    renderClock(); renderMedia(); renderSlow(); renderPomo(); renderClips(); renderShelf(); renderNotifs();
  }

  // ------------------------------------------------------------ relógio do cabeçalho
  function renderClock() {
    $(".js-open-clock").textContent = new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  }
  setInterval(renderClock, 5000);

  // ------------------------------------------------------------ mídia
  const APPS = [
    ["spotify", "Spotify", "#1db954"], ["brave", "Brave", "#fb542b"], ["chrome", "Chrome", "#4285f4"],
    ["msedge", "Edge", "#0c8ce9"], ["firefox", "Firefox", "#ff7139"], ["opera", "Opera", "#ff1b2d"],
    ["applemusic", "Apple Music", "#fa2d48"], ["zunemusic", "Media Player", "#7a5cff"], ["deezer", "Deezer", "#a238ff"],
    ["youtube", "YouTube Music", "#ff0033"], ["vlc", "VLC", "#ff8800"],
  ];
  function appInfo(id = "") {
    const l = id.toLowerCase();
    const hit = APPS.find(([k]) => l.includes(k));
    if (hit) return { name: hit[1], color: hit[2] };
    return { name: id.replace(/\.exe$/i, "").split(/[!\\]/).pop() || "Mídia", color: "#8e8e93" };
  }
  // capa provisória enquanto a real não chega (ou quando o player não manda)
  function placeholderArt(app) {
    const c = appInfo(app).color;
    const s = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${c}"/><stop offset="1" stop-color="#111"/></linearGradient></defs><rect width="200" height="200" fill="url(#g)"/><g fill="none" stroke="#fff" stroke-opacity=".85" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" transform="translate(50 46) scale(4.2)"><path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/></g></svg>`;
    return "data:image/svg+xml;base64," + btoa(s);
  }
  function mediaPos() {
    const m = S.media;
    if (!m) return 0;
    let p = m.position;
    if (m.playing) p += (Date.now() - (m.updated_ms || S.mediaAt)) / 1000;
    return m.duration ? Math.min(p, m.duration) : p;
  }
  function renderMedia() {
    const m = S.media;
    $(".js-play").innerHTML = m && m.playing ? I.pause : I.play;
    if (!m) return;
    $(".js-m-app").textContent = appInfo(m.app).name;
    $(".js-m-title").textContent = m.title;
    $(".js-m-artist").textContent = m.artist || " ";
    renderProgress();
  }
  function renderProgress() {
    const m = S.media;
    if (!m) return;
    const p = mediaPos();
    $(".js-m-pos").textContent = fmt(p);
    $(".js-m-dur").textContent = m.duration ? fmt(m.duration) : "ao vivo";
    $(".js-m-fill").style.width = m.duration ? (p / m.duration) * 100 + "%" : "100%";
  }
  setInterval(() => { if (mode === "expanded" && panel() === "media") renderProgress(); }, 500);

  function setArt(url, real) {
    S.art = url || "";
    S.artReal = !!real;
    $$(".js-label").forEach((l) => (l.style.backgroundImage = url ? `url("${url}")` : ""));
    if (!url) { S.color = [255, 255, 255]; applyAccent(); return; }
    const img = new Image();
    img.onload = () => { S.color = dominant(img); applyAccent(); render(); };
    img.src = url;
  }
  function applyAccent() {
    const c = `rgb(${S.color.join(",")})`;
    stage.style.setProperty("--accent", c);
    stage.style.setProperty("--glow", c);
  }
  function dominant(img) {
    const c = document.createElement("canvas");
    c.width = c.height = 24;
    const x = c.getContext("2d", { willReadFrequently: true });
    x.drawImage(img, 0, 0, 24, 24);
    const d = x.getImageData(0, 0, 24, 24).data;
    let r = 0, g = 0, b = 0, wsum = 0;
    for (let i = 0; i < d.length; i += 4) {
      const R = d[i], G = d[i + 1], B = d[i + 2];
      const mx = Math.max(R, G, B), mn = Math.min(R, G, B);
      const sat = mx ? (mx - mn) / mx : 0, lum = mx / 255;
      const w = sat * sat * (lum > 0.15 ? 1 : 0.1) + 0.01;
      r += R * w; g += G * w; b += B * w; wsum += w;
    }
    const col = [r / wsum, g / wsum, b / wsum];
    const k = Math.max(1, 210 / (Math.max(...col) || 1));
    return col.map((v) => Math.min(255, Math.round(v * k)));
  }

  function onMedia(m) {
    const prev = S.media;
    S.media = m;
    S.mediaAt = Date.now();
    const newTrack = m && (!prev || prev.title !== m.title || prev.artist !== m.artist);
    if (!m) setArt("", false);
    else if (newTrack || !S.art) setArt(placeholderArt(m.app), false);
    renderMedia();
    if (newTrack && m.playing && prev !== undefined) {
      setTimeout(() => {
        if (S.media && S.media.title === m.title)
          hud("toast", 2600, {
            img: S.art, glow: true, title: m.title, sub: m.artist || appInfo(m.app).name,
            acc: `<div class="eq" style="height:16px"><i></i><i></i><i></i><i></i><i></i></div>`,
          });
      }, 600);
    }
    render();
  }
  function onArt(url) {
    if (url) setArt(url, true);
    else if (S.media && !S.artReal) setArt(placeholderArt(S.media.app), false);
    if (S.hud && S.hud.img !== undefined && S.art) { const im = $(".js-t-ico img"); if (im) im.src = S.art; }
  }
  $(".js-play").addEventListener("click", () => {
    if (S.media) { S.media.position = mediaPos(); S.media.updated_ms = Date.now(); S.media.playing = !S.media.playing; }
    fire("media_cmd", { cmd: "toggle" }); renderMedia(); render();
  });
  $(".js-next").addEventListener("click", () => { fire("media_cmd", { cmd: "next" }); if (DEMO) demo.next(); });
  $(".js-prev").addEventListener("click", () => { fire("media_cmd", { cmd: "prev" }); if (DEMO) demo.prev(); });
  $(".js-prev").innerHTML = I.prev;
  $(".js-next").innerHTML = I.next;
  $(".js-ico-music").innerHTML = I.music;
  $(".js-ico-tray").innerHTML = I.tray;
  listen("island:media", onMedia);
  listen("island:art", onArt);

  // ------------------------------------------------------------ vinil girando
  // 33⅓ rpm = 200°/s; acelera e desacelera como um prato de verdade
  const spins = $$(".disc .spin");
  let spinAng = 0, spinSpeed = 0, spinRaf = 0, spinLast = 0;
  function spinFrame(t) {
    const dt = Math.min(0.05, (t - spinLast) / 1000 || 0.016);
    spinLast = t;
    const playing = !!(S.media && S.media.playing);
    const target = playing ? 200 : 0;
    spinSpeed += (target - spinSpeed) * Math.min(1, dt * (playing ? 2.2 : 1.4));
    spinAng = (spinAng + spinSpeed * dt) % 360;
    const tr = `rotate(${spinAng.toFixed(2)}deg)`;
    spins.forEach((e) => (e.style.transform = tr));
    const visible = mode === "media" || (mode === "expanded" && panel() === "media");
    if (visible && (playing || spinSpeed > 0.5)) spinRaf = requestAnimationFrame(spinFrame);
    else spinRaf = 0;
  }
  function ensureSpin() {
    if (!spinRaf) { spinLast = performance.now(); spinRaf = requestAnimationFrame(spinFrame); }
  }

  // ------------------------------------------------------------ volume / caps
  let firstFast = true;
  function onFast(f) {
    const prev = S.fast;
    S.fast = f;
    $(".js-caps").hidden = !f.caps;
    if (firstFast) { firstFast = false; return; }
    if (prev && f.caps !== prev.caps) {
      hud("toast", 1500, {
        icon: I.caps, iconColor: f.caps ? "#ff9f0a" : "", title: f.caps ? "Caps Lock ativado" : "Caps Lock desativado",
        sub: "", acc: `<span class="dot ${f.caps ? "" : "off"}" style="${f.caps ? "background:#ff9f0a;box-shadow:0 0 10px #ff9f0a" : ""}"></span>`,
      });
      return;
    }
    if (f.volume != null && prev && (f.volume !== prev.volume || f.muted !== prev.muted)) {
      const v = Math.round(f.volume * 100);
      $(".js-vol-fill").style.width = (f.muted ? 0 : v) + "%";
      $(".js-vol-num").textContent = f.muted ? "—" : v;
      $(".js-vol-ico").innerHTML = f.muted || v === 0 ? I.vol0 : v < 50 ? I.vol1 : I.vol2;
      pill.classList.toggle("muted", f.muted);
      hud("volume", 1400);
    }
  }
  listen("island:fast", onFast);

  // ------------------------------------------------------------ sistema
  function sparkPath(arr, max) {
    if (arr.length < 2) return "";
    const n = 40, step = 100 / (n - 1), off = n - arr.length;
    const pts = arr.map((v, i) => [(i + off) * step, 28 - Math.min(1, v / (max || 1)) * 24 - 2]);
    return `M${pts[0][0]},28 ` + pts.map((p) => `L${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ") + ` L100,28 Z`;
  }
  let firstSlow = true;
  function onSlow(s) {
    const prev = S.slow;
    S.slow = s;
    S.cpuHist.push(s.stats.cpu); if (S.cpuHist.length > 40) S.cpuHist.shift();
    S.netHist.push(s.stats.down); if (S.netHist.length > 40) S.netHist.shift();
    if (mode === "expanded") renderSlow(); else renderStatus();
    if (s.detail && S.view === s.detail.kind) renderDetail();

    if (firstSlow) { firstSlow = false; return; }
    const b = s.battery, pb = prev && prev.battery;
    if (b && pb && b.charging !== pb.charging) {
      hud("toast", 2200, {
        icon: I.bolt, iconBg: b.charging ? "rgba(48,209,88,.18)" : "", iconColor: b.charging ? "#30d158" : "",
        title: b.charging ? "Carregando" : "Na bateria", sub: b.charging ? "Carregador conectado" : "Carregador desconectado",
        acc: `<span style="color:${b.charging ? "#30d158" : ""}">${b.percent}%</span>${I.battery(b.percent, b.charging)}`,
      });
    } else if (b && pb && !b.charging && [20, 10, 5].includes(b.percent) && pb.percent !== b.percent) {
      hud("toast", 3000, { icon: I.battery(b.percent, false), iconBg: "rgba(255,69,58,.18)", iconColor: "#ff453a", title: "Bateria fraca", sub: `Restam ${b.percent}%`, acc: "" });
    }
    const n = s.network, pn = prev && prev.network;
    if (pn && n.online !== pn.online) {
      hud("toast", 2000, {
        icon: n.online ? (n.wifi ? I.wifi : I.lan) : I.wifiOff, iconColor: n.online ? "#0a84ff" : "#ff453a",
        iconBg: n.online ? "rgba(10,132,255,.16)" : "rgba(255,69,58,.16)",
        title: n.online ? "Conectado" : "Sem internet", sub: n.online ? n.name || (n.wifi ? "Wi-Fi" : "Cabo") : "Verifique sua conexão",
        acc: `<span class="dot ${n.online ? "" : "off"}"></span>`,
      });
    }
  }
  function renderStatus() {
    const s = S.slow;
    if (!s) return;
    const b = s.battery, n = s.network;
    $(".js-open-batt").innerHTML = b ? `${b.percent}% ${I.battery(b.percent, b.charging)}` : I.bolt;
    $(".js-open-wifi").innerHTML = n.online ? (n.wifi ? I.wifi : I.lan) : I.wifiOff;
  }
  function renderSlow() {
    renderStatus();
    const s = S.slow;
    if (!s) return;
    const st = s.stats;
    const cpu = Math.round(st.cpu);
    $(".js-cpu").textContent = cpu + "%";
    $(".js-cpu").classList.toggle("hot", cpu >= 85);
    $(".js-cpu-spark").setAttribute("d", sparkPath(S.cpuHist, 100));
    $(".js-ram").textContent = GB(st.ram_used) + " GB";
    $(".js-ram-sub").textContent = `de ${GB(st.ram_total)} GB`;
    $(".js-ram-fill").style.width = (st.ram_used / st.ram_total) * 100 + "%";
    if (st.gpu) {
      $(".js-gpu").textContent = st.gpu.usage + "%";
      $(".js-gpu-sub").textContent = `${st.gpu.temp}°C · ${GB(st.gpu.vram_used)} GB`;
      $(".js-gpu-fill").style.width = st.gpu.usage + "%";
    } else {
      $(".js-gpu").textContent = "—";
      $(".js-gpu-sub").textContent = "sem NVIDIA";
    }
    $(".js-down").textContent = "↓ " + rate(st.down);
    $(".js-up").textContent = "↑ " + rate(st.up);
    $(".js-net-spark").setAttribute("d", sparkPath(S.netHist, Math.max(...S.netHist, 256 * 1024)));
  }
  listen("island:slow", onSlow);

  // ------------------------------------------------------------ detalhe de componente
  function procList(procs, valueOf, maxOf, label) {
    if (!procs || !procs.length) return `<div class="loading">Sem processos para mostrar</div>`;
    const max = Math.max(...procs.map(maxOf), 1e-9);
    return `<div class="sec">${label}</div><ul class="procs">${procs
      .map((p) => `<li style="--p:${(maxOf(p) / max) * 100}%"><span class="pb"></span><span class="pn">${esc(p.name)}</span>${p.count > 1 ? `<span class="pc">×${p.count}</span>` : ""}<span class="pv">${valueOf(p)}</span></li>`)
      .join("")}</ul>`;
  }
  const taskBtn = `<button class="pillbtn sm js-taskmgr">Gerenciador de Tarefas ${I.ext}</button>`;
  function renderDetail() {
    const box = $(".js-detail");
    const d = S.slow && S.slow.detail;
    if (!d || d.kind !== S.view) { box.innerHTML = `<div class="loading">Coletando…</div>`; return; }
    let html = "";
    if (d.kind === "cpu") {
      const cores = d.cores.map((c) => `<i><b style="height:${Math.max(4, c)}%"></b></i>`).join("");
      html = `<div class="d-wrap"><div class="d-left">
          <div class="d-name">${esc(d.brand || "Processador")}</div>
          <div class="d-big">${Math.round(d.usage)}<small>%</small></div>
          <div class="kv"><span>Frequência</span><span>${(d.freq_mhz / 1000).toFixed(2)} GHz</span></div>
          <div class="kv"><span>Núcleos / threads</span><span>${d.physical ?? "?"} / ${d.threads}</span></div>
          <div class="cores">${cores}</div>
          <div class="d-foot">${taskBtn}</div>
        </div><div class="d-right">${procList(d.procs, (p) => p.cpu.toFixed(1) + "%", (p) => p.cpu, "Processos por CPU")}</div></div>`;
    } else if (d.kind === "ram") {
      const pct = (d.used / d.total) * 100;
      html = `<div class="d-wrap"><div class="d-left">
          <div class="d-name">Memória RAM</div>
          <div class="d-big">${GB(d.used)}<small> / ${GB(d.total)} GB</small></div>
          <div class="bar" style="margin:4px 0 6px"><div class="fill" style="width:${pct}%"></div></div>
          <div class="kv"><span>Em uso</span><span>${pct.toFixed(0)}%</span></div>
          <div class="kv"><span>Disponível</span><span>${GB(d.available)} GB</span></div>
          <div class="kv"><span>Paginação</span><span>${GB(d.swap_used)} / ${GB(d.swap_total)} GB</span></div>
          <div class="d-foot">${taskBtn}</div>
        </div><div class="d-right">${procList(d.procs, (p) => MB(p.mem), (p) => p.mem, "Processos por memória")}</div></div>`;
    } else if (d.kind === "gpu") {
      if (!d.available) html = `<div class="loading">Nenhuma GPU NVIDIA encontrada (o driver NVML não respondeu).</div>`;
      else {
        const vp = d.vram_total ? (d.vram_used / d.vram_total) * 100 : 0;
        const rows = [
          ["Temperatura", `${d.temp}°C`],
          d.power_w != null ? ["Consumo", `${d.power_w.toFixed(1)} W`] : null,
          d.clock_mhz != null ? ["Clock", `${d.clock_mhz} MHz`] : null,
          d.fan != null ? ["Ventoinha", `${d.fan}%`] : null,
        ].filter(Boolean).map(([k, v]) => `<div class="kv"><span>${k}</span><span>${v}</span></div>`).join("");
        html = `<div class="d-wrap"><div class="d-left">
            <div class="d-name">${esc(d.name)}</div>
            <div class="d-big">${d.usage ?? 0}<small>%</small></div>
            ${rows}
            <div class="kv"><span>VRAM</span><span>${GB(d.vram_used)} / ${GB(d.vram_total)} GB</span></div>
            <div class="bar"><div class="fill" style="width:${vp}%"></div></div>
            <div class="d-foot">${taskBtn}</div>
          </div><div class="d-right">${procList(d.procs, (p) => (p.vram != null ? MB(p.vram) : "—"), (p) => p.vram || 1, "Usando a GPU")}</div></div>`;
      }
    } else if (d.kind === "net") {
      const n = S.slow.network;
      const ifs = d.ifaces.map((i) => `<div class="iface"><b>${esc(i.name)}</b><span class="r">↓ ${rate(i.down)}<small>${MB(i.total_down)} total</small></span><span class="r">↑ ${rate(i.up)}<small>${MB(i.total_up)} total</small></span><small>${i.ips.join(" · ") || "sem IPv4"}</small></div>`).join("");
      html = `<div class="sec" style="display:flex;justify-content:space-between;align-items:center">
          <span>${n.online ? `Conectado${n.name ? " a " + esc(n.name) : ""}` : "Sem internet"}</span>
          <button class="pillbtn sm js-goto-wifi">${I.wifi} Redes Wi-Fi</button></div>
        <div class="ifaces scroll" style="max-height:200px">${ifs || '<div class="loading">Nenhuma interface ativa</div>'}</div>`;
    }
    box.innerHTML = html;
  }
  $(".js-detail").addEventListener("click", (e) => {
    if (e.target.closest(".js-taskmgr")) fire("open_uri", { uri: "taskmgr" });
    if (e.target.closest(".js-goto-wifi")) openView("wifi");
  });

  // ------------------------------------------------------------ Wi-Fi
  let wifiBusy = "";
  async function loadWifi() {
    try { S.wifi = { list: await call("wifi_list"), err: null }; }
    catch (e) { S.wifi = { list: [], err: String(e) }; }
    renderWifi();
  }
  function renderWifi() {
    const box = $(".js-wifi");
    const n = S.slow ? S.slow.network : { online: false, wifi: false, name: "" };
    const w = S.wifi;
    const cur = w && w.list.find((x) => x.connected);
    let html = `<div class="w-cur ${n.online ? "" : "off"}"><div class="wi">${n.online ? (n.wifi ? I.wifi : I.lan) : I.wifiOff}</div>
      <div><b>${esc(cur ? cur.ssid : n.name || (n.online ? "Conectado" : "Desconectado"))}</b>
      <small>${n.online ? (cur ? `Conectado · sinal ${cur.signal}%` : n.wifi ? "Conectado" : "Cabo de rede") : "Sem internet"}</small></div></div>`;
    if (!w) html += `<div class="loading" style="height:166px">Procurando redes…</div>`;
    else if (w.err && w.err.includes("location"))
      html += `<div class="notice"><b>O Windows bloqueou a lista de redes</b>Desde o Windows 11 24H2, apps precisam da permissão de Localização para ver nomes de redes. Ative “Permitir que apps da área de trabalho acessem sua localização”.<br><button class="pillbtn sm js-uri" data-uri="ms-settings:privacy-location">Abrir Localização ${I.ext}</button></div>`;
    else if (w.err && w.err.includes("noadapter")) html += `<div class="notice"><b>Sem adaptador Wi-Fi</b>Nenhuma placa de rede sem fio foi encontrada.</div>`;
    else if (w.err) html += `<div class="notice"><b>Não consegui listar as redes</b>${esc(w.err)}</div>`;
    else
      html += `<ul class="nets scroll">${w.list
        .map((x, i) => `<li class="${x.connected ? "cur" : ""} ${wifiBusy === x.ssid ? "busy" : ""}" data-i="${i}">${I.signal(x.signal)}<span class="nm">${esc(x.ssid)}</span>${x.connected ? '<span class="tag">conectada</span>' : x.saved ? '<span class="tag">salva</span>' : ""}${x.secure ? `<span class="lock">${I.lock}</span>` : ""}</li>`)
        .join("")}</ul>`;
    html += `<div class="w-foot"><button class="pillbtn sm js-wifi-refresh">${I.refresh} Atualizar</button><button class="pillbtn sm js-uri" data-uri="ms-settings:network-wifi">${I.gear} Configurações</button></div>`;
    box.innerHTML = html;
  }
  $(".js-wifi").addEventListener("click", async (e) => {
    const u = e.target.closest(".js-uri");
    if (u) return fire("open_uri", { uri: u.dataset.uri });
    if (e.target.closest(".js-wifi-refresh")) { S.wifi = null; renderWifi(); return loadWifi(); }
    const li = e.target.closest(".nets li");
    if (!li || !S.wifi) return;
    const net = S.wifi.list[+li.dataset.i];
    if (net.connected) return;
    if (!net.saved) return fire("open_uri", { uri: "ms-availablenetworks:" }); // senha: painel do Windows
    wifiBusy = net.ssid; renderWifi();
    try {
      await call("wifi_connect", { profile: net.profile || net.ssid });
      setTimeout(loadWifi, 2500);
    } catch {
      fire("open_uri", { uri: "ms-availablenetworks:" });
    } finally {
      setTimeout(() => { wifiBusy = ""; renderWifi(); }, 2500);
    }
  });

  // ------------------------------------------------------------ bateria / energia
  async function loadPower() {
    try { S.power = await call("power_info"); } catch { S.power = null; }
    renderPower();
  }
  const MODES = [["efficiency", "Eficiência", I.leaf], ["balanced", "Equilibrado", I.scale], ["performance", "Desempenho", I.rocket]];
  function renderPower() {
    const box = $(".js-battery");
    const p = S.power;
    if (!p) { box.innerHTML = `<div class="loading">Lendo energia…</div>`; return; }
    const pct = p.percent;
    let status;
    if (pct == null) status = "Sem bateria · ligado na tomada";
    else if (p.ac && pct >= 100) status = "Carregado · na tomada";
    else if (p.ac) status = "Carregando";
    else status = p.time_left ? `Na bateria · ${dur(p.time_left)} restantes` : "Na bateria";
    if (p.saver) status += " · economia ligada";
    const cls = p.ac ? "chg" : p.saver ? "saver" : pct != null && pct <= 20 ? "low" : "";
    const mi = MODES.findIndex(([k]) => k === p.mode);
    const activePlan = p.plans.find((x) => x.active);
    const balancedActive = activePlan && /381b4222/i.test(activePlan.guid);
    box.innerHTML = `
      <div class="b-top">
        <div class="b-batt ${cls}" style="--p:${pct ?? 100}%"><i></i>${p.ac ? `<span class="bolt">${I.bolt}</span>` : ""}</div>
        <div><div class="b-pct">${pct != null ? pct + "%" : "AC"}</div><div class="b-status">${status}</div></div>
      </div>
      <div class="sec">Modo de energia${!balancedActive && p.mode ? " · funciona no plano Equilibrado" : ""}</div>
      <div class="seg ${p.mode ? "" : "disabled"}">
        <span class="thumb" style="transform:translateX(${Math.max(0, mi) * 100}%);${mi < 0 ? "opacity:0" : ""}"></span>
        ${MODES.map(([k, l, ic], i) => `<button data-mode="${k}" class="${i === mi ? "on" : ""}">${ic}${l}</button>`).join("")}
      </div>
      <div class="sec">Plano de energia</div>
      <div class="plans">${p.plans.map((x) => `<button class="chip ${x.active ? "on" : ""}" data-plan="${x.guid}">${esc(x.name || x.guid.slice(0, 8))}</button>`).join("")}</div>
      <div class="b-foot">
        <button class="pillbtn sm toggle js-uri" data-uri="ms-settings:batterysaver"><span class="sw ${p.saver ? "on" : ""}"></span>Economia de energia</button>
        <button class="pillbtn sm js-uri" data-uri="ms-settings:powersleep">${I.gear} Mais opções</button>
      </div>`;
  }
  $(".js-battery").addEventListener("click", async (e) => {
    const u = e.target.closest(".js-uri");
    if (u) return fire("open_uri", { uri: u.dataset.uri });
    const m = e.target.closest("[data-mode]");
    if (m && S.power) {
      S.power.mode = m.dataset.mode; renderPower();
      try { await call("set_power_mode", { mode: m.dataset.mode }); } catch {}
      return setTimeout(loadPower, 400);
    }
    const pl = e.target.closest("[data-plan]");
    if (pl && S.power) {
      S.power.plans.forEach((x) => (x.active = x.guid === pl.dataset.plan)); renderPower();
      try { await call("set_power_plan", { guid: pl.dataset.plan }); } catch {}
      return setTimeout(loadPower, 400);
    }
  });

  // ------------------------------------------------------------ relógio analógico + calendário
  (function buildFace() {
    let ticks = "", nums = "";
    for (let i = 0; i < 60; i++) {
      const big = i % 5 === 0, a = (i * 6 * Math.PI) / 180;
      const r1 = big ? 41 : 43.5, r2 = 46;
      ticks += `<line class="tick ${big ? "big" : ""}" x1="${50 + r1 * Math.sin(a)}" y1="${50 - r1 * Math.cos(a)}" x2="${50 + r2 * Math.sin(a)}" y2="${50 - r2 * Math.cos(a)}"/>`;
    }
    for (let h = 1; h <= 12; h++) {
      const a = (h * 30 * Math.PI) / 180;
      nums += `<text x="${50 + 32 * Math.sin(a)}" y="${50 - 32 * Math.cos(a)}">${h}</text>`;
    }
    $(".js-ticks").innerHTML = ticks;
    $(".js-nums").innerHTML = nums;
  })();
  let lastSec = -1;
  function clockLoop() {
    const d = new Date();
    const ms = d.getMilliseconds(), s = d.getSeconds() + ms / 1000, m = d.getMinutes() + s / 60, h = (d.getHours() % 12) + m / 60;
    $(".js-sh").setAttribute("transform", `rotate(${s * 6} 50 50)`);
    $(".js-mh").setAttribute("transform", `rotate(${m * 6} 50 50)`);
    $(".js-hh").setAttribute("transform", `rotate(${h * 30} 50 50)`);
    if (d.getSeconds() !== lastSec) {
      lastSec = d.getSeconds();
      $(".js-c-hm").textContent = d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
      $(".js-c-s").textContent = String(d.getSeconds()).padStart(2, "0");
      const ds = d.toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "long" });
      $(".js-c-date").textContent = ds.charAt(0).toUpperCase() + ds.slice(1);
      const start = new Date(d.getFullYear(), 0, 1);
      const day = Math.floor((d - start) / 864e5) + 1;
      const tz = -d.getTimezoneOffset() / 60;
      $(".js-c-sub").textContent = `Dia ${day} de ${d.getFullYear()} · semana ${isoWeek(d)} · UTC${tz >= 0 ? "+" : ""}${tz}`;
    }
    clockRaf = requestAnimationFrame(clockLoop);
  }
  function isoWeek(d) {
    const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
    const dn = t.getUTCDay() || 7;
    t.setUTCDate(t.getUTCDate() + 4 - dn);
    return Math.ceil(((t - new Date(Date.UTC(t.getUTCFullYear(), 0, 1))) / 864e5 + 1) / 7);
  }
  let calMonth = null;
  function renderCalendar() {
    const now = new Date();
    if (!calMonth) calMonth = new Date(now.getFullYear(), now.getMonth(), 1);
    const y = calMonth.getFullYear(), mo = calMonth.getMonth();
    const ct = calMonth.toLocaleDateString("pt-BR", { month: "long", year: "numeric" });
    $(".js-cal-title").textContent = ct.charAt(0).toUpperCase() + ct.slice(1);
    const first = new Date(y, mo, 1).getDay();
    const days = new Date(y, mo + 1, 0).getDate();
    const prevDays = new Date(y, mo, 0).getDate();
    let html = ["D", "S", "T", "Q", "Q", "S", "S"].map((w) => `<span class="wd">${w}</span>`).join("");
    for (let i = 0; i < 42; i++) {
      const n = i - first + 1;
      let cls = "d", label;
      if (n < 1) { label = prevDays + n; cls += " out"; }
      else if (n > days) { label = n - days; cls += " out"; }
      else {
        label = n;
        const wd = (first + n - 1) % 7;
        if (wd === 0 || wd === 6) cls += " we";
        if (n === now.getDate() && mo === now.getMonth() && y === now.getFullYear()) cls += " today";
      }
      html += `<span class="${cls}">${label}</span>`;
    }
    $(".js-cal-grid").innerHTML = html;
  }
  $(".js-cal-prev").addEventListener("click", () => { calMonth = new Date(calMonth.getFullYear(), calMonth.getMonth() - 1, 1); renderCalendar(); });
  $(".js-cal-next").addEventListener("click", () => { calMonth = new Date(calMonth.getFullYear(), calMonth.getMonth() + 1, 1); renderCalendar(); });
  $(".js-cal-today").addEventListener("click", () => { calMonth = null; renderCalendar(); });

  // ------------------------------------------------------------ pomodoro
  const PRESETS = [[25, 5], [50, 10], [15, 3]];
  $(".js-presets").innerHTML = PRESETS.map(([f, b]) => `<button class="chip" data-f="${f}" data-b="${b}">${f} · ${b}</button>`).join("");
  $$(".js-presets .chip").forEach((c) =>
    c.addEventListener("click", () => {
      const p = S.pomo;
      p.focus = +c.dataset.f; p.brk = +c.dataset.b;
      p.running = false; p.phase = "focus"; p.left = p.focus * 60;
      renderPomo(); render();
    })
  );
  const pomoTotal = () => (S.pomo.phase === "focus" ? S.pomo.focus : S.pomo.brk) * 60;
  function pomoToggle() {
    const p = S.pomo;
    if (p.running) { p.left = Math.max(0, (p.endAt - Date.now()) / 1000); p.running = false; }
    else { if (p.left <= 0) p.left = pomoTotal(); p.endAt = Date.now() + p.left * 1000; p.running = true; }
    renderPomo(); render();
  }
  $(".js-pomo-toggle").addEventListener("click", pomoToggle);
  $(".js-pomo-reset").addEventListener("click", () => {
    const p = S.pomo; p.running = false; p.phase = "focus"; p.left = p.focus * 60; renderPomo(); render();
  });
  function renderPomo() {
    const p = S.pomo;
    const left = p.running ? Math.max(0, (p.endAt - Date.now()) / 1000) : p.left;
    const frac = 1 - left / pomoTotal();
    const txt = fmt(Math.ceil(left));
    $(".js-timer-sm").textContent = txt;
    $(".js-timer-lg").textContent = txt;
    $(".js-ring-sm").style.strokeDashoffset = 50.27 * frac;
    $(".js-ring-lg").style.strokeDashoffset = 276.46 * frac;
    $(".js-phase").textContent = p.phase === "focus" ? "Foco" : "Pausa";
    $(".js-pomo-toggle").textContent = p.running ? "Pausar" : left < pomoTotal() && left > 0 ? "Continuar" : "Iniciar";
    $(".js-pomo-count").textContent = `${p.cycles} ${p.cycles === 1 ? "ciclo" : "ciclos"} hoje`;
    $$(".js-presets .chip").forEach((c) => c.classList.toggle("on", +c.dataset.f === p.focus));
    if (p.running && left <= 0) pomoDone();
  }
  function pomoDone() {
    const p = S.pomo;
    chime();
    if (p.phase === "focus") {
      p.cycles++;
      p.phase = "break"; p.left = p.brk * 60; p.endAt = Date.now() + p.left * 1000; p.running = true;
      hud("toast", 4000, { icon: I.check, iconBg: "rgba(48,209,88,.18)", iconColor: "#30d158", title: "Foco concluído", sub: `Pausa de ${p.brk} min`, acc: "" });
    } else {
      p.phase = "focus"; p.left = p.focus * 60; p.running = false;
      hud("toast", 4000, { icon: I.timer, iconBg: "rgba(255,159,10,.18)", iconColor: "#ff9f0a", title: "Pausa encerrada", sub: "Bora para o próximo ciclo?", acc: "" });
    }
    render();
  }
  setInterval(() => { if (S.pomo.running) renderPomo(); }, 250);
  function chime() {
    try {
      const ac = new AudioContext();
      [[880, 0], [1318.5, 0.16]].forEach(([f, t]) => {
        const o = ac.createOscillator(), g = ac.createGain();
        o.type = "sine"; o.frequency.value = f;
        g.gain.setValueAtTime(0, ac.currentTime + t);
        g.gain.linearRampToValueAtTime(0.18, ac.currentTime + t + 0.02);
        g.gain.exponentialRampToValueAtTime(0.0001, ac.currentTime + t + 0.9);
        o.connect(g).connect(ac.destination); o.start(ac.currentTime + t); o.stop(ac.currentTime + t + 1);
      });
    } catch {}
  }

  // ------------------------------------------------------------ clipboard
  const ago = (t) => {
    const s = (Date.now() - t) / 1000;
    if (s < 60) return "agora";
    if (s < 3600) return Math.floor(s / 60) + " min";
    if (s < 86400) return Math.floor(s / 3600) + " h";
    return Math.floor(s / 86400) + " d";
  };
  function onClip(text) {
    S.clips = [{ text, t: Date.now() }, ...S.clips.filter((c) => c.text !== text)].slice(0, 20);
    save("island.clips", S.clips);
    renderClips();
    const one = text.replace(/\s+/g, " ").trim();
    hud("toast", 1600, {
      icon: I.clip, iconBg: "rgba(255,255,255,.1)", title: "Copiado",
      sub: one.length > 60 ? one.slice(0, 60) + "…" : one, acc: `<span style="color:var(--dim);font-weight:500">${one.length} car.</span>`,
    });
  }
  function renderClips() {
    pill.classList.toggle("no-clips", !S.clips.length);
    $(".js-clips").innerHTML = S.clips
      .map((c, i) => `<li data-i="${i}"><span>${esc(c.text.replace(/\s+/g, " ").slice(0, 160))}</span><em>${ago(c.t)}</em></li>`)
      .join("");
  }
  $(".js-clips").addEventListener("click", (e) => {
    const li = e.target.closest("li");
    if (!li) return;
    const c = S.clips[+li.dataset.i];
    fire("copy_text", { text: c.text });
    if (DEMO) navigator.clipboard?.writeText(c.text).catch(() => {});
    S.clips = [{ ...c, t: Date.now() }, ...S.clips.filter((x) => x !== c)];
    save("island.clips", S.clips);
    li.style.background = "rgba(48,209,88,.25)";
    setTimeout(renderClips, 380);
  });
  listen("island:clip", onClip);

  // ------------------------------------------------------------ notificações
  function onNotif(n) {
    if (/island/i.test(n.app)) return;
    S.notifs = [n, ...S.notifs.filter((x) => x.id !== n.id)].slice(0, 30);
    save("island.notifs", S.notifs.map((x) => ({ ...x, icon: x.icon && x.icon.length < 40000 ? x.icon : null })));
    renderNotifs();
    hud("toast", 4200, {
      img: n.icon || "", icon: n.icon ? "" : I.bell, iconBg: n.icon ? "" : "rgba(255,255,255,.1)",
      title: n.title || n.app, sub: n.body || n.app,
      acc: n.title ? `<span style="color:var(--dim);font-weight:500;max-width:90px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc(n.app)}</span>` : "",
    });
  }
  function renderNotifs() {
    const st = S.nstatus;
    let setup = "";
    if (st === "noidentity")
      setup = `<div class="notice"><b>Ativar notificações</b>O Windows só entrega as notificações de outros apps para programas com identidade de pacote. O Island registra essa identidade sozinho; para isso o <b style="display:inline">Modo de Desenvolvedor</b> precisa estar ligado.<br>
        <button class="pillbtn sm primary js-notif-enable">Ativar</button> <button class="pillbtn sm js-uri" data-uri="ms-settings:developers">Modo de Desenvolvedor ${I.ext}</button>
        <div class="js-notif-err" style="margin-top:8px;color:var(--red);font-size:11.5px"></div></div>`;
    else if (st && st !== "allowed")
      setup = `<div class="notice"><b>Permita o acesso às notificações</b>Em Configurações → Privacidade → Notificações, ligue o acesso para o Island.<br><button class="pillbtn sm js-uri" data-uri="ms-settings:privacy-notifications">Abrir configurações ${I.ext}</button></div>`;
    else if (S.notifs.length)
      setup = `<div class="notif-head"><span class="sec" style="margin:0">Recentes</span><button class="chip js-notif-clear">Limpar</button></div>`;
    $(".js-notif-setup").innerHTML = setup;
    const show = !st || st === "allowed";
    $(".js-notifs").innerHTML = show
      ? S.notifs.length
        ? S.notifs.map((n) => `<li>${n.icon ? `<img src="${n.icon}" alt="">` : `<span class="ni">${I.bell}</span>`}<span class="nt"><b>${esc(n.title || n.app)} <i>· ${esc(n.app)}</i></b><small>${esc(n.body || "")}</small></span><em>${ago(n.time_ms || Date.now())}</em></li>`).join("")
        : `<div class="loading">Nenhuma notificação ainda</div>`
      : "";
  }
  $(".v-notif").addEventListener("click", async (e) => {
    const u = e.target.closest(".js-uri");
    if (u) return fire("open_uri", { uri: u.dataset.uri });
    if (e.target.closest(".js-notif-clear")) { S.notifs = []; save("island.notifs", []); return renderNotifs(); }
    const en = e.target.closest(".js-notif-enable");
    if (en) {
      en.textContent = "Registrando…";
      try { await call("enable_notifications"); } // o app reinicia sozinho se der certo
      catch (err) {
        en.textContent = "Ativar";
        const msg = String(err);
        $(".js-notif-err").textContent = /0x80073CFF|developer|desenvolvedor|0x80073D2E/i.test(msg)
          ? "Ligue o Modo de Desenvolvedor e tente de novo."
          : "Não deu certo: " + msg.slice(0, 160);
      }
    }
  });
  listen("island:notif", onNotif);
  listen("island:notif-status", (st) => { S.nstatus = st; renderNotifs(); });

  // ------------------------------------------------------------ estante
  const EXT_COLORS = { pdf: "#ff453a", doc: "#0a84ff", docx: "#0a84ff", xls: "#30d158", xlsx: "#30d158", csv: "#30d158",
    png: "#bf5af2", jpg: "#bf5af2", jpeg: "#bf5af2", gif: "#bf5af2", webp: "#bf5af2", mp4: "#ff375f", mov: "#ff375f",
    mp3: "#ff9f0a", wav: "#ff9f0a", zip: "#8e8e93", rar: "#8e8e93", "7z": "#8e8e93", exe: "#64d2ff", ps1: "#64d2ff",
    py: "#ffd60a", js: "#ffd60a", txt: "#98989d", md: "#98989d" };
  function renderShelf() {
    pill.classList.toggle("no-files", !S.shelf.length);
    $(".js-shelf").innerHTML = S.shelf
      .slice(0, 8)
      .map((p, i) => {
        const name = p.split(/[\\/]/).pop();
        const ext = name.includes(".") ? name.split(".").pop().toLowerCase() : "";
        const label = ext ? ext.toUpperCase().slice(0, 4) : "PASTA";
        return `<div class="file" data-i="${i}" title="${esc(p)}"><button class="x" data-x="${i}">×</button><span class="ext" style="--c:${EXT_COLORS[ext] || "#0a84ff"}">${label}</span><span class="nm">${esc(name)}</span></div>`;
      })
      .join("");
  }
  $(".js-shelf").addEventListener("click", (e) => {
    const x = e.target.closest("[data-x]");
    if (x) { S.shelf.splice(+x.dataset.x, 1); save("island.shelf", S.shelf); renderShelf(); e.stopPropagation(); return; }
    const f = e.target.closest(".file");
    if (f) fire("open_path", { path: S.shelf[+f.dataset.i] });
  });
  function onDrop(paths) {
    S.shelf = [...paths, ...S.shelf.filter((p) => !paths.includes(p))].slice(0, 8);
    save("island.shelf", S.shelf);
    S.tab = "shelf"; save("island.tab", S.tab);
    S.view = null;
    renderShelf(); refreshExpanded(); render();
  }
  listen("island:drag", (v) => { S.drag = v; if (v) refreshExpanded(); render(); });
  listen("island:drop", onDrop);

  // ------------------------------------------------------------ inicialização
  applyGeometry(...SIZES.idle);
  refreshExpanded();
  render();
  reportRect(SIZES.idle[0], SIZES.idle[1]);

  // ================================================================ DEMO
  function demoInvoke(cmd) {
    const sleep = (ms, v) => new Promise((r) => setTimeout(() => r(v), ms));
    if (cmd === "wifi_list")
      return sleep(500, [
        { ssid: "Casa_5G", signal: 92, secure: true, saved: true, connected: true, profile: "Casa_5G" },
        { ssid: "Casa_2.4G", signal: 78, secure: true, saved: true, connected: false, profile: "Casa_2.4G" },
        { ssid: "VIVO-8A21", signal: 54, secure: true, saved: false, connected: false, profile: "" },
        { ssid: "Lab_TI", signal: 41, secure: true, saved: true, connected: false, profile: "Lab_TI" },
        { ssid: "Cafe Livre", signal: 22, secure: false, saved: false, connected: false, profile: "" },
      ]);
    if (cmd === "power_info")
      return sleep(150, {
        plans: [
          { guid: "381B4222-F694-41F0-9685-FF5BB260DF2E", name: "Equilibrado", active: true },
          { guid: "8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C", name: "Alto desempenho", active: false },
          { guid: "A1841308-3541-4FAB-BC81-F71556F20B4A", name: "Economia de energia", active: false },
        ],
        mode: demoState.mode, saver: false, ac: demoState.charging, percent: 78, time_left: 3 * 3600 + 25 * 60,
      });
    if (cmd === "set_power_mode") return null;
    return null;
  }
  const demoState = { mode: "balanced", charging: false };
  const demo = {};
  if (DEMO) {
    document.body.classList.add("demo");
    const q = new URLSearchParams(location.search);
    $("#demo-hint").hidden = q.has("clean");

    const tracks = [
      ["Midnight City", "M83", 243, ["#ff5e8a", "#5b2a86"]],
      ["Blinding Lights", "The Weeknd", 200, ["#ff3b30", "#2b0a0a"]],
      ["Ocean Eyes", "Billie Eilish", 200, ["#4fc3f7", "#0b3a5a"]],
      ["Sunflower", "Post Malone, Swae Lee", 158, ["#ffd23f", "#ff7a00"]],
    ];
    let ti = 0;
    const cover = ([a, b]) => {
      const c = document.createElement("canvas"); c.width = c.height = 200;
      const x = c.getContext("2d");
      const g = x.createLinearGradient(0, 0, 200, 200); g.addColorStop(0, a); g.addColorStop(1, b);
      x.fillStyle = g; x.fillRect(0, 0, 200, 200);
      x.globalAlpha = 0.35; x.fillStyle = "#fff";
      x.beginPath(); x.arc(140, 60, 46, 0, 7); x.fill();
      x.globalAlpha = 0.18; x.beginPath(); x.arc(50, 160, 70, 0, 7); x.fill();
      return c.toDataURL("image/png");
    };
    const play = (i, playing = true, pos = 0) => {
      ti = (i + tracks.length) % tracks.length;
      const [title, artist, d, cols] = tracks[ti];
      onMedia({ title, artist, app: "Spotify.exe", playing, position: pos, duration: d, updated_ms: Date.now() });
      setTimeout(() => onArt(cover(cols)), 120);
    };
    demo.next = () => play(ti + 1);
    demo.prev = () => play(ti - 1);
    if (q.get("media") === "brave") {
      onMedia({ title: "(2222) AO VIVO: INTERNACIONAL x GRÊMIO", artist: "", app: "Brave", playing: true, position: 48, duration: 0, updated_ms: Date.now() });
    } else play(0, true, 74);

    let vol = 0.5, muted = false, caps = false, online = true, batt = 78;
    onFast({ volume: vol, muted, caps });
    const fakeProcs = (by) => [
      ["chrome", 4.2, 1.9e9, 31], ["Code", 2.1, 1.1e9, 12], ["Spotify", 1.4, 4.1e8, 6], ["explorer", 0.9, 1.6e8, 1],
      ["Discord", 0.7, 5.3e8, 7], ["MsMpEng", 0.6, 2.4e8, 1], ["dwm", 0.5, 1.2e8, 1],
    ].map(([name, cpu, mem, count]) => ({ name, cpu: cpu * (0.7 + Math.random() * 0.6), mem, count }))
      .sort((a, b) => (by === "cpu" ? b.cpu - a.cpu : b.mem - a.mem));
    const slowTick = () => {
      const cpu = Math.max(3, Math.min(97, (S.cpuHist.at(-1) ?? 18) + (Math.random() - 0.5) * 14));
      let detail = null;
      if (S.view === "cpu") detail = { kind: "cpu", brand: "12th Gen Intel(R) Core(TM) i5-12450HX", usage: cpu, freq_mhz: 3300 + Math.random() * 900, physical: 8, threads: 12, cores: Array.from({ length: 12 }, () => Math.random() * 70), procs: fakeProcs("cpu") };
      if (S.view === "ram") detail = { kind: "ram", used: 9.4 * 1024 ** 3, total: 15.7 * 1024 ** 3, available: 6.3 * 1024 ** 3, swap_used: 1.2 * 1024 ** 3, swap_total: 4 * 1024 ** 3, procs: fakeProcs("mem") };
      if (S.view === "gpu") detail = { kind: "gpu", available: true, name: "NVIDIA GeForce RTX 3050 6GB Laptop GPU", usage: 37, mem_util: 12, temp: 57, power_w: 18.4, clock_mhz: 1365, fan: 34, vram_used: 2.1 * 1024 ** 3, vram_total: 6 * 1024 ** 3, procs: [{ name: "dwm", count: 1, vram: null }, { name: "chrome", count: 2, vram: null }, { name: "Spotify", count: 1, vram: null }] };
      if (S.view === "net") detail = { kind: "net", ifaces: [{ name: "Wi-Fi", down: 2.4e6 * Math.random(), up: 3e5 * Math.random(), total_down: 3.2e9, total_up: 4.1e8, ips: ["192.168.0.14"] }, { name: "vEthernet (WSL)", down: 0, up: 1200, total_down: 4.2e7, total_up: 9.1e6, ips: ["172.24.80.1"] }] };
      onSlow({
        stats: { cpu, ram_used: (9.2 + Math.random() * 0.4) * 1024 ** 3, ram_total: 15.7 * 1024 ** 3, down: Math.random() * 3.2 * 1024 ** 2, up: Math.random() * 400 * 1024,
          gpu: { usage: Math.round(20 + Math.random() * 25), temp: 54, vram_used: 2.1 * 1024 ** 3, vram_total: 6 * 1024 ** 3 } },
        battery: { percent: batt, charging: demoState.charging },
        network: { online, wifi: true, name: "Casa_5G" },
        detail,
      });
    };
    for (let i = 0; i < 39; i++) slowTick();
    setInterval(slowTick, 1000);

    const samples = ["git commit -m \"feat: island fluida\"", "https://github.com/r4nges", "Reunião às 15h com a gerência", "192.168.10.25"];
    let si = 0;
    window.addEventListener("keydown", (e) => {
      const k = e.key.toLowerCase();
      if (k === "v") { vol = (vol + 0.14) % 1.01; onFast({ volume: vol, muted, caps }); }
      if (k === "m") { muted = !muted; onFast({ volume: vol, muted, caps }); }
      if (k === "k") { caps = !caps; onFast({ volume: vol, muted, caps }); }
      if (k === "c") onClip(samples[si++ % samples.length]);
      if (k === "b") { demoState.charging = !demoState.charging; slowTick(); }
      if (k === "w") { online = !online; slowTick(); }
      if (k === "n") demo.next();
      if (k === "p") pomoToggle();
      if (k === " ") { e.preventDefault(); $(".js-play").click(); }
      if (k === "d") onDrop(["C:\\Users\\Rangel\\Desktop\\relatorio-notebooks.pdf", "C:\\Users\\Rangel\\Downloads\\inventario.xlsx", "C:\\fotos\\setup.png"]);
      if (k === "t") onNotif({ id: Date.now(), app: "WhatsApp", title: "Mãe", body: "Já chegou em casa?", icon: null, time_ms: Date.now() });
      if (k === "e") { S.hover = !S.hover; refreshExpanded(); render(); }
    });

    // ?state=expanded&tab=system&view=cpu → estado fixo (screenshots)
    if (q.get("tab")) S.tab = q.get("tab");
    if (q.get("seed")) {
      onClip(samples[1]); onClip(samples[2]); onClip(samples[0]);
      onDrop(["C:\\Users\\Rangel\\Desktop\\relatorio-notebooks.pdf", "C:\\Users\\Rangel\\Downloads\\inventario.xlsx", "C:\\fotos\\setup.png", "C:\\scripts\\deploy.ps1"]);
      S.hud = null; clearTimeout(hudTimer);
      if (q.get("tab")) S.tab = q.get("tab");
    }
    if (q.get("state") === "expanded") {
      S.hover = true; refreshExpanded();
      if (q.get("view")) { openView(q.get("view")); slowTick(); }
      render();
    }
    if (q.get("state") === "idle") { S.media.playing = false; render(); }
  }
})();
