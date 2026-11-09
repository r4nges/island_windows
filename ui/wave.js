/* Island — onda do USB, na janela "wave" (ver `play_wave` em main.rs).
 * Recebe `wave:play`, sobe a captura da tela como textura e desenha o anel de
 * vidro com o shader do mockup aprovado. A saída tem alfa: fora do anel é
 * transparente (a tela de verdade aparece, viva), no anel entra a imagem
 * refratada + o brilho. Sem distorção, só o brilho. */
(() => {
  "use strict";

  const T = window.__TAURI__;
  if (!T) return;
  const invoke = (cmd, args) => T.core.invoke(cmd, args);
  const cv = document.getElementById("gl");

  const PEAK = { subtle: 0.34, medium: 0.6 }; // brilho da frente da onda
  const DELAY = 140; // a onda sai quando a pílula já está abrindo (ms depois do pedido)

  const gl = cv.getContext("webgl", { alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false });

  const VS = `attribute vec2 p; void main(){ gl_Position = vec4(p, 0., 1.); }`;
  // Anel de vidro: perfil de "bolha" gaussiano em torno do raio r. A derivada
  // do perfil desloca a amostra (refração), com leve aberração cromática, um
  // brilho especular na encosta virada para a ilha e a cor da aura somada.
  // A captura chega em BGRA: o vermelho está no canal b da textura.
  // Saída pré-multiplicada e sempre válida (rgb <= alfa), para nenhum passo da
  // composição (Chromium, DWM) cortar o brilho: no anel, alfa sobe com a bolha
  // e a imagem refratada cobre a tela; o brilho entra com alfa = seu canal mais
  // forte, o que fica perto de um "screen" sobre a tela real.
  const FS = `
    precision highp float;
    uniform sampler2D tex;
    uniform vec2 res;      // px do dispositivo
    uniform vec2 tsz;      // tamanho da captura (o monitor; a janela tem 1 px a menos)
    uniform vec2 c;        // centro da ilha (px, origem no topo)
    uniform float r, band, bulge, peak, fade, dpr;
    uniform vec3 col;
    void main() {
      vec2 px = vec2(gl_FragCoord.x, res.y - gl_FragCoord.y);
      vec2 d = px - c;
      float dist = max(length(d), 1.0);
      vec2 n = d / dist;
      float x = (dist - r) / band;
      float h = exp(-x * x * 2.2);            // altura do vidro
      float dh = -4.4 * x * h;                // inclinação
      // refração: a encosta empurra a imagem; o topo da bolha amplia um pouco
      vec2 off = n * (dh * 0.55 + h * 0.6) * bulge * dpr * fade;
      vec2 uvR = (px - off * 1.10) / tsz;
      vec2 uvG = (px - off) / tsz;
      vec2 uvB = (px - off * 0.90) / tsz;
      vec3 base = vec3(texture2D(tex, uvR).b, texture2D(tex, uvG).g, texture2D(tex, uvB).r);
      // aura: frente da onda + véu fraco por dentro, que apaga com o tempo
      float front = exp(-pow((dist - r + band * 0.15) / (band * 0.55), 2.0));
      float veil = (dist < r ? 0.16 : 0.0) * smoothstep(0.0, r, dist);
      vec3 glow = col * (front + veil) * peak * fade;
      // brilho especular fino na encosta interna (luz vindo da ilha)
      float spec = pow(max(0.0, -dh) / 2.0, 3.0) * 0.22 * peak * fade * step(0.001, bulge);
      vec3 g = min(glow + vec3(spec), 1.0);
      // o vidro cobre a tela real na medida do deslocamento (que cresce com a
      // bolha): onde ele passa de 1 px a imagem refratada é opaca; onde é
      // menor, misturar com a tela viva não deixa imagem dupla
      float a = smoothstep(0.25, 1.0, length(off)) * step(0.001, bulge);
      float ga = max(g.r, max(g.g, g.b));
      vec3 lit = min(base + g, 1.0);
      gl_FragColor = vec4(lit * a + g * (1.0 - a), a + (1.0 - a) * ga);
    }`;

  let U = {}, tex = null;
  if (gl) {
    const sh = (type, src) => {
      const s = gl.createShader(type);
      gl.shaderSource(s, src);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
      return s;
    };
    const prog = gl.createProgram();
    gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS));
    gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS));
    gl.linkProgram(prog);
    gl.useProgram(prog);
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, "p");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    ["tex", "res", "tsz", "c", "r", "band", "bulge", "peak", "fade", "dpr", "col"].forEach((k) => (U[k] = gl.getUniformLocation(prog, k)));
    tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.uniform1i(U.tex, 0);
    gl.clearColor(0, 0, 0, 0);
    emptyTexture();
  }

  /** Textura de 1 px transparente: solta a memória da captura entre as ondas. */
  function emptyTexture() {
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(4));
  }

  // a resposta crua do `wave_frame` chega como ArrayBuffer; os outros formatos
  // cobrem um IPC de reserva que serialize os bytes
  const toBuffer = (v) =>
    v instanceof ArrayBuffer ? v
      : ArrayBuffer.isView(v) ? v.buffer.slice(v.byteOffset, v.byteOffset + v.byteLength)
      : Array.isArray(v) ? new Uint8Array(v).buffer
      : null;

  /** Sobe a captura (8 bytes de largura/altura + BGRA). [w, h], ou null se não veio. */
  async function loadCapture() {
    const buf = toBuffer(await invoke("wave_frame"));
    if (!buf || buf.byteLength <= 8) return null;
    const dv = new DataView(buf);
    const w = dv.getUint32(0, true), h = dv.getUint32(4, true);
    if (!w || !h || buf.byteLength < 8 + w * h * 4) return null;
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(buf, 8, w * h * 4));
    return [w, h];
  }

  const raf = () => new Promise((r) => requestAnimationFrame(r));
  const easeOut = (t) => 1 - Math.pow(1 - t, 2.2);

  /** Fim: limpa o quadro e espera ele ir para a tela antes de o Rust esconder
   *  a janela (senão a próxima onda abriria mostrando o último quadro). */
  async function finish() {
    if (gl) {
      gl.clear(gl.COLOR_BUFFER_BIT);
      await raf();
      await raf();
      emptyTexture();
    }
    invoke("wave_done").catch(() => {});
  }

  async function play(p) {
    if (!gl || gl.isContextLost()) return finish();
    // o canvas no tamanho físico da janela (o monitor menos 1 px)
    cv.width = p.width;
    cv.height = p.height;
    gl.viewport(0, 0, cv.width, cv.height);
    gl.clear(gl.COLOR_BUFFER_BIT);
    emptyTexture();

    const dpr = p.scale || 1;
    const H = p.height / dpr, W = p.width / dpr; // px lógicos, como no mockup
    const band = Math.max(110, H * 0.14);
    // centro da pílula no monitor (o Rust já somou onde a janela da ilha está)
    const cx = Number.isFinite(p.cx) ? p.cx : W / 2, cy = Number.isFinite(p.cy) ? p.cy : 0;
    // o anel termina além do canto da tela mais distante: no topo centralizado
    // são os de baixo; na base, os de cima; num canto, o canto oposto
    const far = Math.max(Math.hypot(cx, cy), Math.hypot(W - cx, cy), Math.hypot(cx, H - cy), Math.hypot(W - cx, H - cy)) + band * 1.5;
    gl.uniform2f(U.res, cv.width, cv.height);
    gl.uniform2f(U.tsz, cv.width, cv.height);
    gl.uniform2f(U.c, cx * dpr, cy * dpr);
    gl.uniform1f(U.band, band * dpr);
    gl.uniform1f(U.bulge, 0);
    gl.uniform1f(U.peak, PEAK[p.intensity] ?? PEAK.subtle);
    gl.uniform1f(U.dpr, dpr);
    gl.uniform3f(U.col, p.color[0] / 255, p.color[1] / 255, p.color[2] / 255);

    // A captura (~8 MB em 1080p) vem em paralelo: a onda não espera por ela.
    // Até chegar, só o brilho; depois o vidro entra em ~120 ms. Nos primeiros
    // quadros o anel ainda está colado na ilha, onde a distorção mal aparece.
    let shot = null, glassAt = 0;
    const pending = p.capture && p.bulge > 0
      ? loadCapture().then((s) => { shot = s; }, () => {})
      : null;

    // início no relógio dos quadros; espera por quadros e não por setTimeout,
    // que logo depois de a janela aparecer ainda pode vir atrasado
    const t0 = performance.now() + (p.at + DELAY - Date.now()), dur = p.duration;
    for (;;) {
      const now = performance.now();
      if (now < t0) { await raf(); continue; }
      const t = Math.min(1, (now - t0) / dur);
      if (t >= 1) break;
      if (shot && !glassAt) {
        glassAt = now;
        gl.uniform2f(U.tsz, shot[0], shot[1]);
      }
      if (glassAt) gl.uniform1f(U.bulge, p.bulge * Math.min(1, (now - glassAt) / 120));
      // nasce rápido, perde força ao atravessar a tela
      const fade = (t < 0.06 ? t / 0.06 : 1) * (1 - Math.pow(t, 1.5) * 0.8);
      gl.uniform1f(U.r, easeOut(t) * far * dpr);
      gl.uniform1f(U.fade, fade);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      await raf();
    }
    await pending; // não deixa uma captura atrasada subir depois do fim
    return finish();
  }

  T.event.listen("wave:play", (e) =>
    play(e.payload).catch((err) => {
      console.warn("wave", err);
      finish();
    })
  );
})();
