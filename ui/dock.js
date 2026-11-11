/* Island — barra de tarefas (dock). Sem build: JS puro.
 * No app recebe do Rust a lista de apps (`dock:items`), as opções (`dock:cfg`),
 * o tema (`dock:theme`), o hover (`dock:hover`) e o modo barra (`dock:max`:
 * janela maximizada no monitor, a dock vira barra colada no pé da tela);
 * aberto no navegador roda em modo demo (tecla M ou `?max` alternam o modo
 * barra). Toda ação volta ao Rust por comando: quem foca, minimiza e abre
 * apps é ele. */
(() => {
  "use strict";

  const T = window.__TAURI__;
  const DEMO = !T;
  const invoke = (cmd, args) => (T ? T.core.invoke(cmd, args) : Promise.resolve(demoInvoke(cmd, args)));
  const fire = (cmd, args) => { invoke(cmd, args).catch(() => {}); };
  const listen = (ev, fn) => T && T.event.listen(ev, (e) => fn(e.payload));

  const dock = document.querySelector("#dock");
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

  // desenhos do mockup para os botões do sistema; os apps usam o ícone real
  const G = {
    start: '<svg viewBox="0 0 24 24" fill="#5fb4ff"><rect x="3" y="3" width="8.4" height="8.4" rx="1"/><rect x="12.6" y="3" width="8.4" height="8.4" rx="1"/><rect x="3" y="12.6" width="8.4" height="8.4" rx="1"/><rect x="12.6" y="12.6" width="8.4" height="8.4" rx="1"/></svg>',
    search: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><circle cx="10.5" cy="10.5" r="6"/><path d="m15 15 5 5"/></svg>',
    up: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m7 14 5-5 5 5"/></svg>',
  };
  // padrão igual ao settings.rs (Taskbar); só vale até o Rust responder
  const DEFAULT_CFG = { enabled: false, glass: "system", align: "center", size: "small", magnify: "subtle", tray_button: true };
  const SIZE = { small: 32, medium: 40, large: 48 };
  // escala máxima extra do ícone sob o cursor (mockup: Leve 0.28, Forte 0.6)
  const MAG = { off: 0, subtle: 0.28, strong: 0.6 };

  const S = { cfg: DEFAULT_CFG, light: false, items: [], max: false };
  const icons = new Map(); // chave do app -> data URL | null (carregando ou sem ícone)
  const els = new Map(); // chave do app -> botão

  // ------------------------------------------------------------ estrutura fixa
  const sysBtn = (k, tip, cls, inner, bg = "") =>
    `<button class="app ${cls}" data-act="${k}" aria-label="${tip}"><i style="${bg}">${inner}</i><span class="tip">${tip}</span></button>`;
  dock.innerHTML =
    sysBtn("start", "Iniciar", "start", G.start) +
    sysBtn("search", "Pesquisar", "", G.search, "--bg:linear-gradient(160deg,#5b6170,#353a46)") +
    '<span class="sep s1"></span><span class="sep s2"></span>' +
    sysBtn("tray", "Ícones ocultos (bandeja)", "more", G.up) +
    sysBtn("trash", "Lixeira", "trash", "");
  const sep2 = dock.querySelector(".s2");
  const trash = dock.querySelector(".trash i");
  trash.className = "ph";
  invoke("dock_icon", { key: "trash" }).then((url) => { if (url) { trash.className = "real"; trash.innerHTML = `<img src="${url}" alt="">`; } }).catch(() => {});

  // ------------------------------------------------------------ opções e tema
  function applyCfg() {
    const c = S.cfg, B = document.body;
    B.dataset.glass = c.glass === "system" ? (S.light ? "light" : "dark") : c.glass;
    B.dataset.pos = c.align === "left" ? "left" : "center";
    B.dataset.tray = c.tray_button ? "on" : "off";
    document.documentElement.style.setProperty("--icon", (SIZE[c.size] || 32) + "px");
    resetMag();
    report(600); // a dock desliza até o lugar novo (.45 s)
  }

  // modo barra: o CSS faz o morph (.45 s) e o vidro nativo segue a cada quadro
  function applyMax() {
    const v = S.max ? "on" : "off";
    if (document.body.dataset.max === v) return;
    document.body.dataset.max = v;
    report(600);
  }

  // ------------------------------------------------------------ apps
  function iconHtml(it) {
    const url = icons.get(it.key);
    if (url) return `<img src="${url}" alt="">`;
    return esc((it.name || "?").trim().charAt(0).toUpperCase());
  }

  function loadIcon(it) {
    if (icons.has(it.key)) return;
    icons.set(it.key, null);
    invoke("dock_icon", { key: it.key }).then((url) => {
      if (!url) return;
      icons.set(it.key, url);
      const i = els.get(it.key)?.querySelector("i");
      if (i) { i.className = "real"; i.innerHTML = `<img src="${url}" alt="">`; }
    }).catch(() => {});
  }

  function render() {
    const keys = new Set(S.items.map((it) => it.key));
    for (const [k, el] of els) if (!keys.has(k)) { el.remove(); els.delete(k); }
    let prev = dock.querySelector(".s1");
    let moved = false;
    for (const it of S.items) {
      let el = els.get(it.key);
      if (!el) {
        el = document.createElement("button");
        el.className = "app";
        el.dataset.key = it.key;
        el.innerHTML = `<i class="${icons.get(it.key) ? "real" : "ph"}">${iconHtml(it)}</i><span class="tip"></span>`;
        els.set(it.key, el);
        loadIcon(it);
      }
      el.classList.toggle("on", it.open > 0);
      el.classList.toggle("focus", !!it.focus);
      const tip = it.open > 1 ? `${it.name} · ${it.open} janelas` : it.name;
      const t = el.querySelector(".tip");
      if (t.textContent !== tip) t.textContent = tip;
      el.setAttribute("aria-label", tip);
      // só move no DOM quem saiu do lugar (mover zera o :hover)
      if (prev.nextElementSibling !== el) { prev.after(el); moved = true; }
      prev = el;
    }
    if (moved || S.items.length !== els.size) resetMag();
    report(300);
  }

  dock.addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b) return;
    if (b.dataset.act) fire("dock_action", { kind: b.dataset.act });
    else if (b.dataset.key) fire("dock_click", { key: b.dataset.key });
  });
  dock.addEventListener("contextmenu", (e) => e.preventDefault());

  // ------------------------------------------------------------ ampliação
  // escala pela distância do cursor (mesma curva do mockup); a margem extra
  // abre espaço para os vizinhos e a dock cresce junto. O ícone cresce
  // (s − 1) × lado / 2 de cada lado (origem no meio do pé), então a margem é
  // essa: o ampliado nunca encosta no vizinho
  function magnify(x) {
    const max = MAG[S.cfg.magnify] ?? 0.28, reach = 110;
    if (!max) return;
    const side = SIZE[S.cfg.size] || 32;
    dock.querySelectorAll(".app").forEach((el) => {
      const r = el.getBoundingClientRect();
      const d = Math.abs(x - (r.left + r.width / 2));
      const s = 1 + max * Math.max(0, Math.cos(Math.min(1, d / reach) * Math.PI / 2));
      el.style.transform = `scale(${s.toFixed(3)})`;
      el.style.margin = `0 ${((s - 1) * side / 2).toFixed(1)}px`;
    });
    report(250);
  }
  function resetMag() {
    dock.querySelectorAll(".app").forEach((el) => { el.style.transform = ""; el.style.margin = ""; });
    setHov(null);
    report(250);
  }
  // dica do botão sob o cursor
  let hov = null;
  function setHov(el) {
    if (el === hov) return;
    hov?.classList.remove("hov");
    hov = el;
    hov?.classList.add("hov");
  }
  dock.addEventListener("mousemove", (e) => { setHov(e.target.closest(".app")); magnify(e.clientX); });
  dock.addEventListener("mouseleave", resetMag);

  // ------------------------------------------------------------ medida para o Rust
  // `bar` = retângulo do vidro (a janela nativa desfocada por baixo segue ele);
  // `hit` = a área que aceita o mouse (inclui os ícones ampliados acima da dock).
  // Mede a cada quadro enquanto algo anima e só manda o que mudou.
  let raf = 0, until = 0, lastSent = "";
  function report(ms = 0) {
    until = Math.max(until, performance.now() + ms);
    if (!raf) raf = requestAnimationFrame(measure);
  }
  function measure() {
    raf = 0;
    const r = dock.getBoundingClientRect();
    let top = r.top, left = r.left, right = r.right;
    dock.querySelectorAll(".app").forEach((el) => {
      if (!el.offsetParent) return; // escondido (botão ⌃ desligado)
      const a = el.getBoundingClientRect();
      top = Math.min(top, a.top); left = Math.min(left, a.left); right = Math.max(right, a.right);
    });
    const q = (v) => Math.round(v * 2) / 2;
    // o vidro vai quase exato: o desfoque nativo tem de casar com ele px a px
    const f = (v) => Math.round(v * 100) / 100;
    const bar = { x: f(r.left), y: f(r.top), w: f(r.width), h: f(r.height) };
    const hit = { x: q(left), y: q(top), w: q(right - left), h: q(r.bottom - top) };
    // vidro nativo de quina reta (modo barra) quando o raio do CSS já passou
    // da metade dos 8 px: o canto nativo é tudo ou nada, troca no meio do morph
    const square = parseFloat(getComputedStyle(dock).borderTopLeftRadius) < 4;
    // botão ⌃: a bandeja abre acima dele (e é a "posição" dos ícones dela)
    const more = dock.querySelector(".more");
    const mr = more && more.offsetParent ? more.getBoundingClientRect() : null;
    const tray = mr ? { x: q(mr.left), y: q(mr.top), w: q(mr.width), h: q(mr.height) } : null;
    const key = JSON.stringify([bar, hit, square, tray]);
    if (key !== lastSent) { lastSent = key; fire("dock_layout", { bar, hit, square, tray }); }
    if (performance.now() < until) raf = requestAnimationFrame(measure);
  }
  window.addEventListener("resize", () => report(100));

  // ------------------------------------------------------------ eventos do Rust
  listen("dock:items", (items) => { S.items = items || []; render(); });
  listen("dock:cfg", (c) => { if (c) { S.cfg = c; applyCfg(); render(); } });
  listen("dock:theme", (light) => { S.light = !!light; applyCfg(); });
  // o Rust tirou o mouse da dock (a página não vê o mouseleave quando a janela
  // volta a deixar os cliques passarem)
  listen("dock:hover", (inside) => { if (!inside) resetMag(); });
  listen("dock:max", (on) => { S.max = !!on; applyMax(); });

  invoke("dock_init").then((init) => {
    if (!init) return;
    S.cfg = init.cfg || DEFAULT_CFG;
    S.light = !!init.light;
    S.items = init.items || [];
    S.max = !!init.max;
    applyMax();
    applyCfg();
    render();
  }).catch(() => {});
  applyCfg();

  // ================================================================ DEMO
  function demoInvoke(cmd) {
    if (cmd === "dock_init")
      return {
        cfg: { ...DEFAULT_CFG, enabled: true, glass: "dark" },
        light: false,
        items: [
          { key: "pin:explorer", name: "Explorador de Arquivos", pinned: true, open: 1, focus: false },
          { key: "pin:brave", name: "Brave", pinned: true, open: 2, focus: true },
          { key: "pin:steam", name: "Steam", pinned: true, open: 0, focus: false },
          { key: "app:terminal", name: "Terminal", pinned: false, open: 1, focus: false },
        ],
        max: new URLSearchParams(location.search).has("max"),
      };
    return null;
  }
  if (DEMO) {
    document.body.classList.add("demo");
    // M alterna o modo barra (como maximizar/restaurar uma janela)
    addEventListener("keydown", (e) => {
      if (e.key === "m" || e.key === "M") { S.max = !S.max; applyMax(); }
    });
  }
})();
