/* Island — frontend. Sem build: JS puro.
 * No app (Tauri) recebe eventos do Rust; aberto no navegador roda em modo demo. */
(() => {
  "use strict";

  const T = window.__TAURI__;
  const DEMO = !T;
  const invoke = (cmd, args) => (T ? T.core.invoke(cmd, args).catch(() => {}) : Promise.resolve());
  const listen = (ev, fn) => T && T.event.listen(ev, (e) => fn(e.payload));

  const $ = (s) => document.querySelector(s);
  const $$ = (s) => [...document.querySelectorAll(s)];
  const pill = $("#pill");

  // ------------------------------------------------------------ ícones
  const svg = (d, extra = "") =>
    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" ${extra}>${d}</svg>`;
  const fillSvg = (d) => `<svg viewBox="0 0 24 24" fill="currentColor">${d}</svg>`;
  const I = {
    music: svg('<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>'),
    gauge: svg('<path d="M3 13a9 9 0 0 1 18 0"/><path d="M12 13l4-4"/><path d="M5 17h14"/>'),
    timer: svg('<circle cx="12" cy="13" r="8"/><path d="M12 9v4l2.5 2"/><path d="M9 2h6"/>'),
    clip: svg('<rect x="8" y="3" width="8" height="4" rx="1.5"/><path d="M16 5h1.5A1.5 1.5 0 0 1 19 6.5v13a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19.5v-13A1.5 1.5 0 0 1 6.5 5H8"/>'),
    tray: svg('<path d="M3 13l2.5-7h13L21 13"/><path d="M3 13v5a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5h-5.5a3.5 3.5 0 0 1-7 0z"/>'),
    play: fillSvg('<path d="M8 5.5v13a1 1 0 0 0 1.5.86l10.4-6.5a1 1 0 0 0 0-1.72L9.5 4.64A1 1 0 0 0 8 5.5z"/>'),
    pause: fillSvg('<rect x="6.5" y="5" width="4" height="14" rx="1.2"/><rect x="13.5" y="5" width="4" height="14" rx="1.2"/>'),
    next: fillSvg('<path d="M4 6.5v11a.8.8 0 0 0 1.2.7L13 13v4.5a.8.8 0 0 0 1.2.7l8.3-5.5a.8.8 0 0 0 0-1.4L14.2 5.8a.8.8 0 0 0-1.2.7V11L5.2 5.8a.8.8 0 0 0-1.2.7z"/>'),
    prev: fillSvg('<path d="M20 6.5v11a.8.8 0 0 1-1.2.7L11 13v4.5a.8.8 0 0 1-1.2.7L1.5 12.7a.8.8 0 0 1 0-1.4L9.8 5.8a.8.8 0 0 1 1.2.7V11l7.8-5.2a.8.8 0 0 1 1.2.7z"/>'),
    vol0: svg('<path d="M11 5L6 9H3v6h3l5 4z" fill="currentColor"/><path d="M22 9l-6 6M16 9l6 6"/>'),
    vol1: svg('<path d="M11 5L6 9H3v6h3l5 4z" fill="currentColor"/><path d="M15.5 9.5a3.5 3.5 0 0 1 0 5"/>'),
    vol2: svg('<path d="M11 5L6 9H3v6h3l5 4z" fill="currentColor"/><path d="M15.5 9.5a3.5 3.5 0 0 1 0 5"/><path d="M18.5 6.5a7.5 7.5 0 0 1 0 11"/>'),
    bolt: fillSvg('<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>'),
    wifi: svg('<path d="M2 8.5a15 15 0 0 1 20 0"/><path d="M5 12a10 10 0 0 1 14 0"/><path d="M8.5 15.5a5 5 0 0 1 7 0"/><circle cx="12" cy="19" r="1" fill="currentColor"/>'),
    wifiOff: svg('<path d="M2 2l20 20"/><path d="M8.5 15.5a5 5 0 0 1 7 0"/><path d="M5 12a10 10 0 0 1 5-2.7"/><path d="M2 8.5a15 15 0 0 1 4.2-2.8"/><circle cx="12" cy="19" r="1" fill="currentColor"/>'),
    lan: svg('<rect x="9" y="3" width="6" height="5" rx="1"/><rect x="3" y="16" width="6" height="5" rx="1"/><rect x="15" y="16" width="6" height="5" rx="1"/><path d="M12 8v4M6 16v-4h12v4"/>'),
    caps: svg('<path d="M12 4l7 8h-4v4H9v-4H5z"/><path d="M9 20h6"/>'),
    check: svg('<path d="M5 12.5l4.5 4.5L19 7.5"/>'),
    file: svg('<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>'),
    battery: (p, charging) => {
      const w = Math.max(1.5, (p / 100) * 14);
      const c = charging ? "#30d158" : p <= 20 ? "#ff453a" : "currentColor";
      return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2" y="7" width="18" height="10" rx="3" opacity=".55"/><rect x="4" y="9" width="${w}" height="6" rx="1.5" fill="${c}" stroke="none"/><path d="M22 10.5v3" stroke-linecap="round" opacity=".55"/></svg>`;
    },
  };

  // ------------------------------------------------------------ estado
  const load = (k, d) => { try { return JSON.parse(localStorage.getItem(k)) ?? d; } catch { return d; } };
  const save = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch {} };

  const S = {
    hover: false,
    drag: false,
    media: undefined,
    mediaAt: 0,
    art: "",
    color: [255, 255, 255],
    fast: null,
    slow: null,
    hud: null,
    tab: load("island.tab", "media"),
    clips: load("island.clips", []),
    shelf: load("island.shelf", []),
    cpuHist: [],
    netHist: [],
    pomo: { phase: "focus", focus: 25, brk: 5, running: false, endAt: 0, left: 25 * 60, cycles: 0 },
  };

  // ------------------------------------------------------------ molas
  class Spring {
    constructor(v, k, c) { this.v = v; this.t = v; this.vel = 0; this.k = k; this.c = c; }
    step(dt) {
      const f = -this.k * (this.v - this.t) - this.c * this.vel;
      this.vel += f * dt;
      this.v += this.vel * dt;
    }
    get idle() { return Math.abs(this.v - this.t) < 0.05 && Math.abs(this.vel) < 0.05; }
  }
  // um pouco de "quique" na largura, altura mais contida, raio acompanha
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
  const TOP = 6, STAGE_W = 520;

  function frame(t) {
    const dt = Math.min(0.05, (t - lastT) / 1000 || 0.016);
    lastT = t;
    const steps = Math.ceil(dt / (1 / 240));
    for (let i = 0; i < steps; i++) { sw.step(dt / steps); sh.step(dt / steps); sr.step(dt / steps); }
    pill.style.setProperty("--w", sw.v.toFixed(2) + "px");
    pill.style.setProperty("--h", sh.v.toFixed(2) + "px");
    pill.style.setProperty("--r", Math.max(0, sr.v).toFixed(2) + "px");
    if (sw.idle && sh.idle && sr.idle) {
      raf = 0;
      reportRect(sw.t, sh.t);
      return;
    }
    raf = requestAnimationFrame(frame);
  }

  function animateTo(mode) {
    const [w, h, r] = SIZES[mode];
    // área clicável = maior entre atual e destino até assentar
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
    invoke("set_hit_rect", r);
  }

  // ------------------------------------------------------------ modo
  let mode = "idle";
  function computeMode() {
    if (S.hover || S.drag) return "expanded";
    if (S.hud) return S.hud.kind;
    if (S.pomo.running) return "timer";
    if (S.media && S.media.playing) return "media";
    return "idle";
  }
  function render() {
    const m = computeMode();
    if (m !== mode) {
      mode = m;
      pill.dataset.mode = m;
      animateTo(m);
    }
    pill.dataset.tab = S.drag ? "shelf" : S.tab;
    pill.classList.toggle("drag", S.drag);
    pill.classList.toggle("playing", !!(S.media && S.media.playing));
    pill.classList.toggle("has-media", !!S.media);
    pill.classList.toggle("no-media", !S.media);
    pill.classList.toggle("break", S.pomo.phase === "break");
    const glowOn = (m === "media" || (m === "expanded" && pill.dataset.tab === "media") || (m === "toast" && S.hud?.glow)) && S.media;
    pill.style.setProperty("--glow", glowOn ? `rgba(${S.color.join(",")},.55)` : "rgba(0,0,0,0)");
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

  // ------------------------------------------------------------ abas
  const TABS = [
    ["media", I.music, "Mídia"],
    ["system", I.gauge, "Sistema"],
    ["focus", I.timer, "Foco"],
    ["clip", I.clip, "Clipboard"],
    ["shelf", I.tray, "Estante"],
  ];
  $(".js-tabs").innerHTML = TABS.map(([id, ic, t]) => `<button class="tab" data-id="${id}" title="${t}">${ic}</button>`).join("");
  $$(".tab").forEach((b) =>
    b.addEventListener("click", () => { S.tab = b.dataset.id; save("island.tab", S.tab); refreshExpanded(); render(); })
  );
  function refreshExpanded() {
    $$(".tab").forEach((b) => b.classList.toggle("on", b.dataset.id === (S.drag ? "shelf" : S.tab)));
    renderClock(); renderMedia(); renderSlow(); renderPomo(); renderClips(); renderShelf();
  }

  // ------------------------------------------------------------ relógio + status
  function renderClock() {
    const d = new Date();
    $(".js-clock").textContent = d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  }
  setInterval(renderClock, 5000);

  // ------------------------------------------------------------ mídia
  const fmt = (s) => { s = Math.max(0, Math.floor(s || 0)); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`; };
  function appName(id = "") {
    const l = id.toLowerCase();
    if (l.includes("spotify")) return "Spotify";
    if (l.includes("chrome")) return "Chrome";
    if (l.includes("msedge") || l.includes("edge")) return "Edge";
    if (l.includes("firefox")) return "Firefox";
    if (l.includes("applemusic")) return "Apple Music";
    if (l.includes("zunemusic") || l.includes("media")) return "Media Player";
    if (l.includes("deezer")) return "Deezer";
    if (l.includes("youtube")) return "YouTube Music";
    return id.replace(/\.exe$/i, "").split(/[!\\]/).pop() || "Mídia";
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
    $(".js-m-app").textContent = appName(m.app);
    $(".js-m-title").textContent = m.title;
    $(".js-m-artist").textContent = m.artist || " ";
    renderProgress();
  }
  function renderProgress() {
    const m = S.media;
    if (!m) return;
    const p = mediaPos();
    $(".js-m-pos").textContent = fmt(p);
    $(".js-m-dur").textContent = m.duration ? fmt(m.duration) : "--:--";
    $(".js-m-fill").style.width = m.duration ? (p / m.duration) * 100 + "%" : "0%";
  }
  setInterval(() => { if (mode === "expanded" && S.tab === "media") renderProgress(); }, 500);

  function setArt(url) {
    S.art = url || "";
    $$(".js-art").forEach((img) => (url ? img.setAttribute("src", url) : img.removeAttribute("src")));
    if (!url) { S.color = [255, 255, 255]; applyAccent(); return; }
    const img = new Image();
    img.onload = () => { S.color = dominant(img); applyAccent(); render(); };
    img.src = url;
  }
  function applyAccent() {
    pill.style.setProperty("--accent", `rgb(${S.color.join(",")})`);
  }
  // cor dominante saturada (média ponderada por saturação)
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
    let col = [r / wsum, g / wsum, b / wsum];
    // clareia para funcionar sobre preto
    const mx = Math.max(...col) || 1;
    const k = Math.max(1, 210 / mx);
    return col.map((v) => Math.min(255, Math.round(v * k)));
  }

  function onMedia(m) {
    const prev = S.media;
    S.media = m;
    S.mediaAt = Date.now();
    const newTrack = m && (!prev || prev.title !== m.title || prev.artist !== m.artist);
    if (!m) setArt("");
    renderMedia();
    if (newTrack && m.playing && prev !== undefined) {
      // anúncio da nova faixa, estilo Apple
      setTimeout(() => {
        if (S.media && S.media.title === m.title)
          hud("toast", 2600, {
            img: S.art, icon: I.music, glow: true,
            title: m.title, sub: m.artist || appName(m.app),
            acc: `<div class="eq" style="height:16px"><i></i><i></i><i></i><i></i><i></i></div>`,
          });
      }, 450);
    }
    render();
  }
  $(".js-play").addEventListener("click", () => {
    if (S.media) { S.media.position = mediaPos(); S.media.updated_ms = Date.now(); S.media.playing = !S.media.playing; }
    invoke("media_cmd", { cmd: "toggle" }); renderMedia(); render();
    if (DEMO) demo.toggle();
  });
  $(".js-next").addEventListener("click", () => { invoke("media_cmd", { cmd: "next" }); if (DEMO) demo.next(); });
  $(".js-prev").addEventListener("click", () => { invoke("media_cmd", { cmd: "prev" }); if (DEMO) demo.prev(); });
  $(".js-prev").innerHTML = I.prev;
  $(".js-next").innerHTML = I.next;
  $(".l-expanded .m-empty .big-ico").innerHTML = I.music;
  $(".drop .big-ico").innerHTML = I.tray;
  listen("island:media", onMedia);
  listen("island:art", (url) => setArt(url));

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
  const GB = (b) => (b / 1024 ** 3).toFixed(1);
  const rate = (b) => (b < 1024 ** 2 ? `${Math.round(b / 1024)} KB/s` : `${(b / 1024 ** 2).toFixed(1)} MB/s`);
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
    if (mode === "expanded") renderSlow();
    else renderStatus();

    if (firstSlow) { firstSlow = false; return; }
    const b = s.battery, pb = prev && prev.battery;
    if (b && pb && b.charging !== pb.charging) {
      hud("toast", 2200, {
        icon: I.bolt, iconBg: b.charging ? "rgba(48,209,88,.18)" : "", iconColor: b.charging ? "#30d158" : "",
        title: b.charging ? "Carregando" : "Na bateria", sub: b.charging ? "Carregador conectado" : "Carregador desconectado",
        acc: `<span style="color:${b.charging ? "#30d158" : ""}">${b.percent}%</span>${I.battery(b.percent, b.charging)}`,
      });
    } else if (b && pb && !b.charging && [20, 10, 5].includes(b.percent) && pb.percent !== b.percent) {
      hud("toast", 3000, {
        icon: I.battery(b.percent, false), iconBg: "rgba(255,69,58,.18)", iconColor: "#ff453a",
        title: "Bateria fraca", sub: `Restam ${b.percent}%`, acc: "",
      });
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
    $(".js-batt").innerHTML = b ? `${b.percent}% ${I.battery(b.percent, b.charging)}` : "";
    $(".js-net").innerHTML = n.online ? (n.wifi ? I.wifi : I.lan) : I.wifiOff;
    $(".js-net").title = n.name || "";
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

  // ------------------------------------------------------------ pomodoro
  const PRESETS = [[25, 5], [50, 10], [15, 3]];
  $(".js-presets").innerHTML = PRESETS.map(([f, b]) => `<button class="chip" data-f="${f}" data-b="${b}">${f} · ${b}</button>`).join("");
  $$(".chip").forEach((c) =>
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
    $$(".chip").forEach((c) => c.classList.toggle("on", +c.dataset.f === p.focus));
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
  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
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
    invoke("copy_text", { text: c.text });
    if (DEMO) navigator.clipboard?.writeText(c.text).catch(() => {});
    S.clips = [{ ...c, t: Date.now() }, ...S.clips.filter((x) => x !== c)];
    save("island.clips", S.clips);
    li.style.background = "rgba(48,209,88,.25)";
    setTimeout(renderClips, 380);
  });
  listen("island:clip", onClip);

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
    if (f) invoke("open_path", { path: S.shelf[+f.dataset.i] });
  });
  function onDrop(paths) {
    S.shelf = [...paths, ...S.shelf.filter((p) => !paths.includes(p))].slice(0, 8);
    save("island.shelf", S.shelf);
    S.tab = "shelf"; save("island.tab", S.tab);
    renderShelf(); refreshExpanded(); render();
  }
  listen("island:drag", (v) => { S.drag = v; if (v) refreshExpanded(); render(); });
  listen("island:drop", onDrop);

  // ------------------------------------------------------------ inicialização
  refreshExpanded();
  render();
  reportRect(...SIZES.idle);

  // ================================================================ DEMO
  // Simulação para ver o visual no navegador (sem o app Tauri).
  const demo = {};
  if (DEMO) {
    document.body.classList.add("demo");
    $("#demo-hint").hidden = new URLSearchParams(location.search).has("clean");

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
      const [title, artist, dur, cols] = tracks[ti];
      setArt(cover(cols));
      onMedia({ title, artist, app: "Spotify.exe", playing, position: pos, duration: dur, updated_ms: Date.now() });
    };
    demo.toggle = () => {};
    demo.next = () => play(ti + 1);
    demo.prev = () => play(ti - 1);
    play(0, true, 74);

    let vol = 0.5, muted = false, caps = false, charging = false, online = true, batt = 78;
    onFast({ volume: vol, muted, caps });
    const slowTick = () => {
      const cpu = Math.max(3, Math.min(97, (S.cpuHist.at(-1) ?? 18) + (Math.random() - 0.5) * 14));
      onSlow({
        stats: {
          cpu, ram_used: (9.2 + Math.random() * 0.4) * 1024 ** 3, ram_total: 15.7 * 1024 ** 3,
          down: Math.random() * 3.2 * 1024 ** 2, up: Math.random() * 400 * 1024,
          gpu: { usage: Math.round(20 + Math.random() * 25), temp: 54, vram_used: 2.1 * 1024 ** 3, vram_total: 6 * 1024 ** 3 },
        },
        battery: { percent: batt, charging },
        network: { online, wifi: true, name: "Casa_5G" },
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
      if (k === "b") { charging = !charging; slowTick(); }
      if (k === "w") { online = !online; slowTick(); }
      if (k === "n") demo.next();
      if (k === "p") pomoToggle();
      if (k === " ") { e.preventDefault(); $(".js-play").click(); }
      if (k === "d") onDrop(["C:\\Users\\Rangel\\Desktop\\relatorio-notebooks.pdf", "C:\\Users\\Rangel\\Downloads\\inventario.xlsx", "C:\\fotos\\setup.png"]);
      if (k === "e") { S.hover = !S.hover; refreshExpanded(); render(); }
    });

    // ?state=expanded&tab=system  → estado fixo (para screenshots)
    const q = new URLSearchParams(location.search);
    if (q.get("tab")) { S.tab = q.get("tab"); }
    if (q.get("state") === "expanded") { S.hover = true; refreshExpanded(); render(); }
    if (q.get("seed")) {
      onClip(samples[1]); onClip(samples[2]); onClip(samples[0]);
      onDrop(["C:\\Users\\Rangel\\Desktop\\relatorio-notebooks.pdf", "C:\\Users\\Rangel\\Downloads\\inventario.xlsx", "C:\\fotos\\setup.png", "C:\\scripts\\deploy.ps1"]);
      S.hud = null; clearTimeout(hudTimer);
      if (q.get("tab")) S.tab = q.get("tab");
      if (q.get("state") !== "expanded") S.hover = false;
      refreshExpanded(); render();
    }
    if (q.get("state") === "timer") { pomoToggle(); S.media.playing = false; render(); }
    if (q.get("state") === "volume") setTimeout(() => { vol = 0.64; onFast({ volume: vol, muted, caps }); clearTimeout(hudTimer); }, 600);
    if (q.get("state") === "clip") setTimeout(() => { onClip(samples[0]); clearTimeout(hudTimer); }, 600);
    if (q.get("state") === "charge") setTimeout(() => { charging = true; slowTick(); clearTimeout(hudTimer); }, 600);
    if (q.get("state") === "track") setTimeout(() => { play(2); setTimeout(() => clearTimeout(hudTimer), 600); }, 300);
    if (q.get("state") === "idle") { S.media.playing = false; render(); }
  }
})();
