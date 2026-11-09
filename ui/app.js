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
    sun: svg('<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>'),
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
    upd: svg('<path d="M12 4v11"/><path d="M7 10.5l5 5 5-5"/><path d="M5 20h14"/>'),
    user: svg('<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>'),
    power: svg('<path d="M12 3v9"/><path d="M6.3 6.8a8 8 0 1 0 11.4 0"/>'),
    moon: svg('<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z"/>'),
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
    update: null, // { version, current, notes, progress: null | {pct} | "installing" }
    cfg: null, // configurações (settings.json no Rust); null até carregar = padrão
    pinned: false, // aberta pela bandeja/atalho: fica expandida até o mouse entrar e sair
    fullscreen: false, // app cobrindo a ilha: tela cheia ou janela maximizada por baixo (com "esconder" a janela some; com "traço" vira peek)
    user: null, // { name, login, picture } do painel do usuário
  };

  // padrão igual ao settings.rs; só vale até o Rust responder (e no modo demo)
  const DEFAULT_CFG = {
    modules: { volume: true, brightness: true, caps: true, battery: true, network: true, track: true, clipboard: true, notifications: true, pomo_sound: true },
    aura: { intensity: "subtle", expanded: false, color: "auto" },
    scale: 1.0,
    monitor: null,
    shortcuts: { open: "Ctrl+Alt+KeyI", play: "Ctrl+Alt+Space", pomo: "Ctrl+Alt+KeyP" },
    muted_apps: [],
    fullscreen: "hide",
  };
  const cfg = () => S.cfg || DEFAULT_CFG;
  /** aviso ligado nas configurações? */
  const on = (k) => cfg().modules[k] !== false;

  // ------------------------------------------------------------ geometria + molas
  const STAGE_W = 760, TOP = 6;
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
    peek: [72, 5, 2.5],
    hidden: [180, 3, 1.5], // invisível; a largura é a área de "puxar" no topo
    idle: [128, 30, 15],
    media: [280, 32, 16],
    timer: [260, 32, 16],
    volume: [300, 38, 19],
    brightness: [300, 38, 19],
    toast: [340, 54, 27],
    expanded: [480, 200, 34],
  };
  const PANEL_SIZES = {
    cpu: [560, 300, 34], ram: [560, 300, 34], gpu: [560, 300, 34], net: [560, 300, 34],
    wifi: [420, 344, 34], battery: [440, 300, 34], clock: [420, 412, 36], settings: [460, 408, 34], user: [420, 218, 34],
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

  // modos colados na borda de cima (app por baixo da ilha)
  const AT_EDGE = ["peek", "hidden"];
  let lastRect = "";
  function reportRect(w, h) {
    const r = { x: (STAGE_W - w) / 2, y: AT_EDGE.includes(mode) ? 0 : TOP, w, h };
    const key = Object.values(r).map((n) => n.toFixed(0)).join(",");
    if (key === lastRect) return;
    lastRect = key;
    fire("set_hit_rect", r);
  }

  // ------------------------------------------------------------ modo / render
  let mode = "idle", sizeKey = "";
  function computeMode() {
    if (S.hover || S.drag || S.pinned) return "expanded";
    // app por baixo da ilha: traço visível, ou escondida (invisível, mas com uma
    // faixa no topo que puxa a ilha ao encostar o mouse)
    if (S.fullscreen) return cfg().fullscreen === "peek" ? "peek" : "hidden";
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
    mode = m; // antes do animateTo: reportRect usa o modo (o traço fica colado no topo)
    if (key !== sizeKey) { sizeKey = key; animateTo(w, h, r); }
    pill.dataset.mode = m;
    stage.dataset.mode = m; // a aura (irmã da pílula) muda o recuo com a ilha aberta
    pill.dataset.panel = panel();
    pill.classList.toggle("in-panel", !!S.view && !S.drag);
    pill.classList.toggle("drag", S.drag);
    pill.classList.toggle("playing", !!(S.media && S.media.playing));
    pill.classList.toggle("has-media", !!S.media);
    pill.classList.toggle("no-media", !S.media);
    pill.classList.toggle("break", S.pomo.phase === "break");
    const a = cfg().aura;
    const glowOn =
      a.intensity !== "off" &&
      ((!!S.media && (m === "media" || (m === "expanded" && pill.dataset.panel === "media") || (m === "toast" && S.hud?.glow))) ||
        (a.expanded && m === "expanded"));
    aura.classList.toggle("on", glowOn);
    ensureSpin();
    $$(".st").forEach((b) => b.classList.remove("on"));
    if (S.view === "settings" || S.view === "user") $(".js-open-user").classList.add("on");
    if (S.view === "wifi") $(".js-open-wifi").classList.add("on");
    if (S.view === "battery") $(".js-open-batt").classList.add("on");
    if (S.view === "clock") $(".js-open-clock").classList.add("on");
  }

  // ------------------------------------------------------------ hover
  let hoverTimer = 0, pinEntered = false, pinTimer = 0, pullTimer = 0, pullReady = false;
  function setHover(v) {
    clearTimeout(hoverTimer);
    if (!v) { clearTimeout(pullTimer); pullReady = false; }
    hoverTimer = setTimeout(() => {
      if (S.hover === v) return;
      // aberta pela bandeja: só fecha depois que o mouse entrou e saiu
      if (S.pinned) {
        if (v) { pinEntered = true; clearTimeout(pinTimer); }
        else if (!pinEntered) return;
        else S.pinned = false;
      }
      // puxar da borda (traço/escondida) pede o mouse parado um pouco mais:
      // quem mira nas abas do navegador passa por ali sem querer abrir
      if (v && AT_EDGE.includes(mode) && !pullReady) {
        pullTimer = setTimeout(() => { pullReady = true; setHover(true); }, 170); // +90 do hover ≈ 260 ms
        return;
      }
      pullReady = false;
      S.hover = v;
      if (v) {
        clearTimeout(pullTimer);
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
  listen("island:covered", (v) => {
    S.fullscreen = v;
    if (v) { S.hud = null; clearTimeout(hudTimer); }
    render();
  });

  /** Expande sem o mouse em cima (bandeja; depois, atalho global). Fecha quando
   *  o mouse entra e sai, ou em 10 s se ele nem chegar. */
  function pinOpen(view) {
    S.pinned = true; pinEntered = S.hover;
    S.hud = null; clearTimeout(hudTimer);
    clearTimeout(pinTimer);
    pinTimer = setTimeout(() => {
      if (!S.pinned || pinEntered) return;
      S.pinned = false; openView(null); render();
    }, 10000);
    refreshExpanded();
    if (view !== undefined && S.view !== view) openView(view);
    render();
  }

  // ------------------------------------------------------------ HUD / toasts
  let hudTimer = 0;
  function hud(kind, ms, data = {}) {
    if (S.hover || S.drag || S.pinned || S.fullscreen) return; // nada salta sobre tela cheia
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
  const VIEW_TITLES = { cpu: "CPU", ram: "Memória", gpu: "GPU", net: "Rede", wifi: "Wi-Fi", battery: "Bateria", clock: "Relógio", settings: "Configurações", user: "Conta" };
  $(".js-tabs").innerHTML = TABS.map(([id, ic, t]) => `<button class="tab" data-id="${id}" title="${t}">${ic}</button>`).join("");
  $$(".tab").forEach((b) =>
    b.addEventListener("click", () => { S.tab = b.dataset.id; save("island.tab", S.tab); refreshExpanded(); render(); })
  );
  $(".js-back").addEventListener("click", () => {
    // configurações abertas pelo painel do usuário voltam para ele; o resto volta às abas
    if (S.view === "settings" && settingsFromUser) return openView("user");
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
    if (recording && v !== "settings") stopRecording(); // saiu do painel no meio da gravação
    if (v === "settings") { renderSettings(); loadMonitors(); loadShortcutErrs(); }
    if (v !== "settings") settingsFromUser = false;
    if (v === "user") renderUser();
    render();
  }

  // ------------------------------------------------------------ usuário + energia
  let settingsFromUser = false, armed = "", armTimer = 0;
  const initial = (n) => (n || "?").trim().charAt(0).toUpperCase();
  const avatar = (u) => (u?.picture ? `<span class="avatar"><img src="${u.picture}" alt=""></span>` : `<span class="avatar">${esc(initial(u?.name))}</span>`);
  const POWER = [["sleep", I.moon, "Suspender"], ["restart", I.refresh, "Reiniciar"], ["shutdown", I.power, "Desligar"]];
  function renderUserButton() { $(".js-open-user").innerHTML = S.user ? avatar(S.user) : I.user; }
  function renderUser() {
    const u = S.user;
    $(".js-user").innerHTML = `
      <div class="u-top">${avatar(u)}<div style="min-width:0"><div class="u-name">${esc(u?.name || "…")}</div><div class="u-login">${esc(u?.login || "")}</div></div></div>
      <div class="u-actions">
        ${POWER.map(([k, ic, l]) => `<button class="u-btn ${armed === k ? "armed" : ""}" data-power="${k}">${ic}${armed === k ? "Confirmar" : l}</button>`).join("")}
        <button class="u-btn js-u-settings">${I.gear}Configurações</button>
      </div>`;
  }
  $(".js-user").addEventListener("click", (e) => {
    if (e.target.closest(".js-u-settings")) { openView("settings"); settingsFromUser = true; return; }
    const b = e.target.closest("[data-power]");
    if (!b) return;
    const k = b.dataset.power;
    // reiniciar/desligar: primeiro clique arma, o segundo (em até 3 s) executa
    if (k !== "sleep" && armed !== k) {
      armed = k; renderUser();
      clearTimeout(armTimer);
      armTimer = setTimeout(() => { armed = ""; if (S.view === "user") renderUser(); }, 3000);
      return;
    }
    armed = ""; clearTimeout(armTimer);
    openView(null); render();
    invoke("power_action", { action: k }).catch((err) =>
      hud("toast", 3000, { icon: I.power, iconBg: "rgba(255,69,58,.18)", iconColor: "#ff453a", title: "Não deu certo", sub: String(err), acc: "" })
    );
  });
  $(".js-open-user").addEventListener("click", () => openView("user"));
  renderUserButton();
  invoke("user_info").then((u) => { if (u) { S.user = u; renderUserButton(); if (S.view === "user") renderUser(); } }).catch(() => {});
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
  // cor da aura sem capa (modo automático com a ilha aberta e nada tocando)
  const AURA_FALLBACK = "#5e5ce6";
  const AURA_OPACITY = { off: 0, subtle: 0.2, medium: 0.32 };
  // grão do dithering (ver #grain no CSS): 96×96 pixels claros/escuros com alfa ≤ 2/255
  (function makeGrain() {
    const c = document.createElement("canvas");
    c.width = c.height = 96;
    const x = c.getContext("2d");
    const img = x.createImageData(96, 96);
    for (let i = 0; i < img.data.length; i += 4) {
      const v = Math.random() < 0.5 ? 0 : 255;
      img.data[i] = img.data[i + 1] = img.data[i + 2] = v;
      img.data[i + 3] = Math.floor(Math.random() * 3); // 0, 1 ou 2 de 255
    }
    x.putImageData(img, 0, 0);
    stage.style.setProperty("--grain-img", `url(${c.toDataURL("image/png")})`);
  })();
  function applyAccent() {
    const c = `rgb(${S.color.join(",")})`;
    const a = cfg().aura;
    stage.style.setProperty("--accent", c);
    stage.style.setProperty("--glow", a.color !== "auto" ? a.color : S.media && S.art ? c : AURA_FALLBACK);
    stage.style.setProperty("--aura-op", AURA_OPACITY[a.intensity] ?? 0.32);
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
    if (newTrack && m.playing && prev !== undefined && on("track")) {
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
      if (on("caps")) hud("toast", 1500, {
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
      if (on("volume")) hud("volume", 1400);
    }
  }
  listen("island:fast", onFast);

  // ------------------------------------------------------------ brilho
  // o Rust só emite quando o valor muda (e nunca na primeira leitura)
  $(".js-br-ico").innerHTML = I.sun;
  function onBrightness(b) {
    $(".js-br-fill").style.width = b + "%";
    $(".js-br-num").textContent = b;
    if (on("brightness")) hud("brightness", 1400);
  }
  listen("island:brightness", onBrightness);

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
    const b = on("battery") && s.battery, pb = prev && prev.battery;
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
    if (pn && n.online !== pn.online && on("network")) {
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
    if (!on("pomo_sound")) return;
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
    if (!on("clipboard")) return; // histórico continua; só o aviso some
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
  // apps que já notificaram (para a lista "silenciar por app"); vai além das 30 notificações guardadas
  let notifApps = load("island.notifApps", []);
  function onNotif(n) {
    if (/island/i.test(n.app)) return;
    S.notifs = [n, ...S.notifs.filter((x) => x.id !== n.id)].slice(0, 30);
    save("island.notifs", S.notifs.map((x) => ({ ...x, icon: x.icon && x.icon.length < 40000 ? x.icon : null })));
    if (n.app && !notifApps.includes(n.app)) { notifApps = [n.app, ...notifApps].slice(0, 40); save("island.notifApps", notifApps); }
    renderNotifs();
    // desligado geral ou app silenciado: continua na aba Notificações, sem aviso
    if (!on("notifications") || cfg().muted_apps?.includes(n.app)) return;
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

  // ------------------------------------------------------------ atualização
  // O Rust checa o latest.json do último Release; aqui só aparece o aviso e o
  // botão verde no cabeçalho do expandido. Instalar fecha e reabre o Island.
  const updBtn = $(".js-upd");
  const GREEN = { iconBg: "rgba(48,209,88,.18)", iconColor: "#30d158" };
  let updToasted = "";
  function renderUpd() {
    const u = S.update;
    updBtn.hidden = !u;
    if (!u) return;
    const p = u.progress;
    updBtn.classList.toggle("busy", !!p);
    updBtn.innerHTML = `${I.upd}<span>${p === "installing" ? "Instalando…" : p != null ? (p.pct != null ? `${p.pct}%` : "Baixando…") : `v${esc(u.version)}`}</span>`;
    updBtn.title = p ? "Atualizando o Island" : `Instalar o Island v${u.version} e reiniciar`;
  }
  function onUpdate(u) {
    S.update = { ...u, progress: S.update?.progress ?? null };
    renderUpd();
    if (updToasted === u.version) return; // um aviso por versão, não a cada 6 h
    updToasted = u.version;
    hud("toast", 5000, { icon: I.upd, ...GREEN, title: "Atualização disponível", sub: `Island v${u.version} · passe o mouse para instalar`, acc: "" });
  }
  function onUpdateProgress(p) {
    if (p.state === "error") {
      if (S.update) S.update.progress = null;
      renderUpd();
      hud("toast", 4500, { icon: I.upd, iconBg: "rgba(255,69,58,.18)", iconColor: "#ff453a", title: "Falha na atualização", sub: p.msg, acc: "" });
      return;
    }
    if (!S.update) return;
    S.update.progress = p.state === "installing" ? "installing" : { pct: p.pct };
    renderUpd();
  }
  updBtn.addEventListener("click", () => {
    if (!S.update || S.update.progress) return;
    S.update.progress = { pct: null };
    renderUpd();
    if (DEMO) { demo.fakeInstall(); return; }
    invoke("install_update").catch((e) => onUpdateProgress({ state: "error", msg: String(e) }));
  });
  listen("island:update", onUpdate);
  listen("island:update-progress", onUpdateProgress);
  listen("island:update-none", (v) =>
    hud("toast", 2600, { icon: I.check, ...GREEN, title: "Island atualizado", sub: `Você já está na versão mais recente (v${v})`, acc: "" })
  );
  // após "Recarregar visual" o aviso pendente continua valendo
  invoke("pending_update").then((u) => { if (u) { updToasted = u.version; onUpdate(u); } }).catch(() => {});

  // ------------------------------------------------------------ configurações
  // Painel dentro da ilha. Cada mudança vale na hora (aura, avisos) e é gravada
  // pelo Rust em settings.json com um pequeno atraso (o slider de cor manda
  // muitos eventos). A escala é aplicada pelo Rust: janela + zoom do WebView.
  const AURA_LEVELS = [["off", "Desligada"], ["subtle", "Sutil"], ["medium", "Média"]];
  const SCALE_OPTS = [[0.9, "90%"], [1.0, "100%"], [1.1, "110%"]];
  const FS_OPTS = [["hide", "Esconder a ilha"], ["peek", "Deixar um traço"]];
  const SWATCHES = ["auto", "#ffffff", "#0a84ff", "#bf5af2", "#ff375f", "#ff9f0a", "#30d158"];
  const MODULE_LABELS = [
    ["volume", "Volume"], ["brightness", "Brilho"], ["caps", "Caps Lock"], ["battery", "Bateria e carregador"], ["network", "Internet caiu/voltou"],
    ["track", "Nova música"], ["clipboard", "Texto copiado"], ["notifications", "Notificações"], ["pomo_sound", "Som do pomodoro"],
  ];
  const hueHex = (h) => {
    // HSL(h, 90%, 62%) -> #rrggbb
    const s = 0.9, l = 0.62, k = (n) => (n + h / 30) % 12, a = s * Math.min(l, 1 - l);
    const f = (n) => Math.round(255 * (l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1))));
    return "#" + [f(0), f(8), f(4)].map((v) => v.toString(16).padStart(2, "0")).join("");
  };
  const hexHue = (hex) => {
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
    const mx = Math.max(r, g, b), d = mx - Math.min(r, g, b);
    if (!d) return 0;
    const h = mx === r ? ((g - b) / d) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return Math.round((h * 60 + 360) % 360);
  };
  let appVersion = DEMO ? "demo" : "";
  if (T?.app?.getVersion) T.app.getVersion().then((v) => { appVersion = v; if (S.view === "settings") renderSettings(); }).catch(() => {});

  const seg = (key, opts, cur) => {
    const i = Math.max(0, opts.findIndex(([v]) => v === cur));
    return `<div class="seg" data-seg="${key}" style="--n:${opts.length}"><span class="thumb" style="transform:translateX(${i * 100}%)"></span>${opts
      .map(([v, l], j) => `<button data-v="${v}" class="${j === i ? "on" : ""}">${l}</button>`).join("")}</div>`;
  };
  const toggle = (key, label, onv) =>
    `<button class="s-row" data-toggle="${key}"><span>${label}</span><span class="sw g ${onv ? "on" : ""}"></span></button>`;

  // \\.\DISPLAY2 -> "Tela 2"
  const monLabel = (m) => `Tela ${(m.id.match(/(\d+)\s*$/) || [, "?"])[1]} · ${m.width}×${m.height}`;
  let monitors = null;
  function loadMonitors() {
    invoke("list_monitors").then((l) => { monitors = l || []; if (S.view === "settings") renderSettings(); }).catch(() => {});
  }
  function monitorChips(cur) {
    if (!monitors) return `<div class="s-label">Procurando monitores…</div>`;
    if (monitors.length <= 1) return `<div class="s-label">Só um monitor conectado: a ilha fica nele.</div>`;
    // escolhido mas desconectado: continua marcado, avisando que está no principal
    const missing = cur && !monitors.some((m) => m.id === cur);
    return `<div class="plans s-mon">
      <button class="chip ${!cur ? "on" : ""}" data-monitor="">Principal</button>
      ${monitors.map((m) => `<button class="chip ${cur === m.id ? "on" : ""}" data-monitor="${esc(m.id)}">${monLabel(m)}${m.primary ? " · principal" : ""}</button>`).join("")}
    </div>${missing ? `<div class="s-label">O monitor escolhido está desconectado; a ilha voltou para o principal.</div>` : ""}`;
  }

  // ---- atalhos globais: formato do global-hotkey ("Ctrl+Alt+KeyI") = KeyboardEvent.code
  const SHORTCUTS = [["open", "Abrir a ilha"], ["play", "Tocar/pausar"], ["pomo", "Pomodoro"]];
  // só nomes que não dependem do layout (no ABNT2 os símbolos mudam de lugar)
  const KEY_NAMES = { Space: "Espaço", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→" };
  const prettyKey = (acc) =>
    acc.split("+").map((t) => t === "Super" ? "Win" : KEY_NAMES[t] || t.replace(/^Key/, "").replace(/^Digit/, "").replace(/^Numpad/, "Num ")).join(" + ");
  let recording = null; // ação sendo gravada
  let shortcutErrs = {};
  function shortcutRow(k, label, acc) {
    const rec = recording === k;
    const err = shortcutErrs[k];
    return `<div class="s-row s-key"><span>${label}${err ? `<small class="s-err">Não registrou: combinação em uso ou inválida</small>` : ""}</span>
      <span class="keys"><button class="keycap ${rec ? "rec" : ""} ${acc ? "" : "none"}" data-rec="${k}">${rec ? "Pressione as teclas…" : acc ? esc(prettyKey(acc)) : "Nenhum"}</button>${acc && !rec ? `<button class="key-x" data-clear="${k}" title="Remover">×</button>` : ""}</span></div>`;
  }
  function loadShortcutErrs() {
    invoke("shortcut_errors").then((e) => { shortcutErrs = e || {}; if (S.view === "settings") renderSettings(); }).catch(() => {});
  }
  function stopRecording(acc) {
    const k = recording;
    recording = null;
    if (acc !== undefined) {
      // set_settings já registra os atalhos de novo
      setCfg("shortcuts", { ...cfg().shortcuts, [k]: acc }, { now: true });
      setTimeout(loadShortcutErrs, 300);
    } else {
      fire("pause_shortcuts", { paused: false });
      renderSettings();
    }
  }
  const MOD_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;
  window.addEventListener("keydown", (e) => {
    if (!recording) return;
    e.preventDefault(); e.stopImmediatePropagation(); // não deixa outros handlers verem a tecla
    if (e.code === "Escape") return stopRecording();
    if (e.code === "Backspace" || e.code === "Delete") return stopRecording("");
    if (MOD_CODES.test(e.code)) return; // esperando a tecla principal
    const fkey = /^F\d{1,2}$/.test(e.code);
    if (!(e.ctrlKey || e.altKey || e.metaKey) && !fkey) return; // precisa de Ctrl/Alt/Win (ou F1–F24)
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    stopRecording([...mods, e.code].join("+"));
  }, true);

  function onShortcut(action) {
    if (action === "open") {
      if (S.pinned) { S.pinned = false; openView(null); render(); }
      else if (!S.hover) pinOpen();
    } else if (action === "play") {
      $(".js-play").click();
    } else if (action === "pomo") {
      pomoToggle();
      if (!S.pomo.running) hud("toast", 1600, { icon: I.timer, iconBg: "rgba(255,159,10,.18)", iconColor: "#ff9f0a", title: "Pomodoro pausado", sub: fmt(Math.ceil(S.pomo.left)), acc: "" });
    }
  }
  listen("island:shortcut", onShortcut);

  // chave ligada = o app gera aviso; desligada = silenciado (só na aba Notificações)
  function appMuteList(c) {
    const muted = c.muted_apps || [];
    const apps = [...new Set([...notifApps, ...muted])];
    if (!apps.length) return `<div class="s-label">Os apps aparecem aqui depois da primeira notificação.</div>`;
    const dis = c.modules.notifications === false ? " s-dis" : "";
    return `<div class="s-grid${dis}">${apps.map((a) =>
      `<button class="s-row" data-mute="${esc(a)}"><span class="s-app">${esc(a)}</span><span class="sw g ${muted.includes(a) ? "" : "on"}"></span></button>`).join("")}</div>
      ${dis ? `<div class="s-label">Avisos de notificação estão desligados acima.</div>` : ""}`;
  }

  function renderSettings() {
    const c = cfg(), a = c.aura;
    const custom = a.color !== "auto" && !SWATCHES.includes(a.color);
    $(".js-settings").innerHTML = `
      <div class="sec">Aura</div>
      ${seg("aura.intensity", AURA_LEVELS, a.intensity)}
      ${toggle("aura.expanded", "Aura com a ilha aberta", a.expanded)}
      <div class="s-label">Cor</div>
      <div class="swatches">${SWATCHES.map((s) =>
        `<button class="swatch ${s === "auto" ? "auto" : ""} ${a.color === s ? "on" : ""}" data-color="${s}" title="${s === "auto" ? "Automática (cor da capa)" : s}" style="${s === "auto" ? "" : `--c:${s}`}"></button>`).join("")}
        <button class="swatch custom ${custom ? "on" : ""}" data-color="custom" title="Personalizada" style="${custom ? `--c:${a.color}` : ""}"></button>
      </div>
      <input type="range" class="hue js-hue" min="0" max="359" value="${custom ? hexHue(a.color) : 210}" ${custom ? "" : "hidden"}>
      <div class="sec">Tamanho da ilha</div>
      ${seg("scale", SCALE_OPTS, c.scale)}
      <div class="sec">App por baixo da ilha</div>
      <div class="s-label">Tela cheia ou janela maximizada (ex.: as abas do navegador)</div>
      ${seg("fullscreen", FS_OPTS, c.fullscreen || "hide")}
      <div class="sec">Monitor</div>
      ${monitorChips(c.monitor)}
      <div class="sec">Atalhos</div>
      ${SHORTCUTS.map(([k, l]) => shortcutRow(k, l, c.shortcuts?.[k] ?? "")).join("")}
      <div class="sec">Avisos na ilha</div>
      <div class="s-grid">${MODULE_LABELS.map(([k, l]) => toggle("modules." + k, l, c.modules[k] !== false)).join("")}</div>
      <div class="sec">Notificações por app</div>
      ${appMuteList(c)}
      <div class="s-foot"><span>Island ${esc(appVersion)}</span><button class="pillbtn sm js-check-upd">${I.refresh} Procurar atualizações</button></div>`;
  }

  let cfgSaveTimer = 0;
  function setCfg(path, value, { now = false, rerender = true } = {}) {
    const c = structuredClone(cfg());
    const [a, b] = path.split(".");
    if (b) c[a][b] = value; else c[a] = value;
    S.cfg = c;
    applyAccent(); render();
    if (rerender) renderSettings();
    clearTimeout(cfgSaveTimer);
    const send = () => invoke("set_settings", { settings: S.cfg }).then((saved) => { if (saved) S.cfg = saved; }).catch((e) => console.warn("set_settings", e));
    if (now) send(); else cfgSaveTimer = setTimeout(send, 350);
  }

  $(".js-settings").addEventListener("click", (e) => {
    const sb = e.target.closest(".seg button");
    if (sb) {
      const key = sb.closest(".seg").dataset.seg;
      const v = key === "scale" ? +sb.dataset.v : sb.dataset.v;
      return setCfg(key, v, { now: key === "scale" });
    }
    const t = e.target.closest("[data-toggle]");
    if (t) {
      const [a, b] = t.dataset.toggle.split(".");
      return setCfg(t.dataset.toggle, !cfg()[a][b]);
    }
    const sw = e.target.closest("[data-color]");
    if (sw) {
      const v = sw.dataset.color;
      return setCfg("aura.color", v === "custom" ? hueHex(+$(".js-hue").value) : v);
    }
    const rec = e.target.closest("[data-rec]");
    if (rec) {
      if (recording === rec.dataset.rec) return stopRecording();
      recording = rec.dataset.rec;
      window.focus(); // a janela nasce sem foco; o clique já ativa, isto garante
      fire("pause_shortcuts", { paused: true });
      return renderSettings();
    }
    const mu = e.target.closest("[data-mute]");
    if (mu) {
      const app = mu.dataset.mute, cur = cfg().muted_apps || [];
      return setCfg("muted_apps", cur.includes(app) ? cur.filter((x) => x !== app) : [...cur, app]);
    }
    const clr = e.target.closest("[data-clear]");
    if (clr) { recording = clr.dataset.clear; return stopRecording(""); }
    const mon = e.target.closest("[data-monitor]");
    if (mon) return setCfg("monitor", mon.dataset.monitor || null, { now: true });
    if (e.target.closest(".js-check-upd")) fire("check_update");
  });
  $(".js-settings").addEventListener("input", (e) => {
    if (!e.target.classList.contains("js-hue")) return;
    const hex = hueHex(+e.target.value);
    $(".swatch.custom").style.setProperty("--c", hex);
    setCfg("aura.color", hex, { rerender: false }); // re-render roubaria o arraste do slider
  });

  listen("island:open-settings", () => pinOpen("settings"));
  invoke("get_settings").then((c) => { if (c) { S.cfg = c; applyAccent(); render(); } }).catch(() => {});

  // ------------------------------------------------------------ inicialização
  applyGeometry(...SIZES.idle);
  refreshExpanded();
  render();
  reportRect(SIZES.idle[0], SIZES.idle[1]);

  // ================================================================ DEMO
  function demoInvoke(cmd, args) {
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
    if (cmd === "user_info") return sleep(80, { name: "Rangel Macedo", login: "Rangel", picture: null });
    if (cmd === "power_action") return null;
    if (cmd === "list_monitors")
      return sleep(120, [{ id: "\\\\.\\DISPLAY1", width: 1920, height: 1080, primary: true }, { id: "\\\\.\\DISPLAY2", width: 2560, height: 1440, primary: false }]);
    if (cmd === "get_settings") return load("island.demo.cfg", null) || DEFAULT_CFG;
    if (cmd === "set_settings") { save("island.demo.cfg", args.settings); document.body.style.zoom = args.settings.scale; return args.settings; }
    if (cmd === "pending_update") return new URLSearchParams(location.search).has("update") ? { version: "0.4.1", current: "0.4.0", notes: null } : null;
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

    let vol = 0.5, muted = false, caps = false, online = true, batt = 78, bright = 60;
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
      if (k === "l") { bright = bright >= 100 ? 5 : Math.min(100, bright + 15); onBrightness(bright); }
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
      if (recording) return; // gravando um atalho no painel
      if (k === "s") pinOpen("settings"); // como o item "Configurações" da bandeja
      if (k === "o") onShortcut("open"); // como o atalho global de abrir
      if (k === "f") { S.fullscreen = !S.fullscreen; render(); } // simula app em tela cheia
      if (k === "u") { updToasted = ""; S.update = null; onUpdate({ version: "0.4.1", current: "0.4.0", notes: null }); }
    });
    // simula download e instalação; no fim "reinicia" (some o botão)
    demo.fakeInstall = () => {
      let pct = 0;
      const t = setInterval(() => {
        pct += 9;
        if (pct < 100) return onUpdateProgress({ state: "downloading", pct });
        clearInterval(t);
        onUpdateProgress({ state: "installing" });
        setTimeout(() => { S.update = null; renderUpd(); }, 1500);
      }, 180);
    };

    // ?state=expanded&tab=system&view=cpu → estado fixo (screenshots)
    if (q.get("tab")) S.tab = q.get("tab");
    if (q.get("seed")) {
      onClip(samples[1]); onClip(samples[2]); onClip(samples[0]);
      onDrop(["C:\\Users\\Rangel\\Desktop\\relatorio-notebooks.pdf", "C:\\Users\\Rangel\\Downloads\\inventario.xlsx", "C:\\fotos\\setup.png", "C:\\scripts\\deploy.ps1"]);
      S.hud = null; clearTimeout(hudTimer);
      if (q.get("tab")) S.tab = q.get("tab");
    }
    document.body.style.zoom = cfg().scale; // no app quem faz isso é o zoom do WebView
    if (q.get("state") === "expanded") {
      S.hover = true; refreshExpanded();
      if (q.get("view")) { openView(q.get("view")); slowTick(); }
      render();
    }
    if (q.get("state") === "idle") { S.media.playing = false; render(); }
  }
})();
