/* RAD — bandeja (ícones ocultos). Sem build: JS puro.
 * O Rust manda o estado ao abrir (`tray:state`) e avisa quando a lista muda
 * (`tray:changed`, aí a página relê com `tray_state`). Os cliques voltam ao
 * Rust (`tray_click`), que fala com o app dono do ícone como a barra nativa
 * falaria: esquerdo, direito (menu do app) e duplo. Esc fecha (`tray_close`);
 * clicar fora fecha sozinho (a janela perde o foco). Aberto no navegador roda
 * em modo demo. */
(() => {
  "use strict";

  const T = window.__TAURI__;
  const DEMO = !T;
  const invoke = (cmd, args) => (T ? T.core.invoke(cmd, args) : Promise.resolve(demoInvoke(cmd, args)));
  const fire = (cmd, args) => { invoke(cmd, args).catch(() => {}); };
  const listen = (ev, fn) => T && T.event.listen(ev, (e) => fn(e.payload));

  const grid = document.querySelector("#grid");
  const panel = document.querySelector("#panel");
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

  function render(st) {
    if (!st) return;
    document.body.dataset.glass = st.glass || "dark";
    const icons = st.icons || [];
    grid.style.setProperty("--cols", st.cols || 1);
    panel.querySelector(".empty")?.remove();
    if (!icons.length) {
      grid.innerHTML = "";
      panel.insertAdjacentHTML("beforeend", '<p class="empty">Nenhum ícone na bandeja</p>');
      return;
    }
    // a dica do app vai no title: a do sistema pode passar das bordas da janela
    grid.innerHTML = icons.map((it) => {
      const tip = esc(it.tip || "");
      const img = it.icon
        ? `<img src="${it.icon}" alt="">`
        : `<i class="ph">${esc((it.tip || "?").trim().charAt(0).toUpperCase())}</i>`;
      return `<button class="ico" data-id="${esc(it.id)}" title="${tip}" aria-label="${tip}">${img}</button>`;
    }).join("");
  }

  const refresh = () => invoke("tray_state").then(render).catch(() => {});

  // clique simples manda o esquerdo; o segundo do duplo manda só o duplo
  // (Enter/espaço no teclado chegam com detail 0: contam como simples)
  grid.addEventListener("click", (e) => {
    const b = e.target.closest(".ico");
    if (b) fire("tray_click", { id: b.dataset.id, how: e.detail >= 2 ? "double" : "left" });
  });
  // botão direito (ou tecla de menu): o menu do próprio app
  document.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    const b = e.target.closest(".ico") || (document.activeElement?.classList.contains("ico") ? document.activeElement : null);
    if (b) fire("tray_click", { id: b.dataset.id, how: "right" });
  });
  addEventListener("keydown", (e) => {
    if (e.key === "Escape") { e.preventDefault(); fire("tray_close"); }
  });

  listen("tray:state", render);
  listen("tray:changed", refresh);
  refresh();

  // ================================================================ DEMO
  function demoInvoke(cmd) {
    if (cmd === "tray_state")
      return {
        glass: "dark",
        cols: 5,
        icons: [
          { id: "1", tip: "Steam", icon: null },
          { id: "2", tip: "Discord", icon: null },
          { id: "3", tip: "Dispositivos Bluetooth", icon: null },
          { id: "4", tip: "OneDrive", icon: null },
          { id: "5", tip: "Segurança do Windows", icon: null },
          { id: "6", tip: "Lightshot", icon: null },
        ],
      };
    return null;
  }
  if (DEMO) document.body.classList.add("demo");
})();
