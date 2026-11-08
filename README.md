# Island 🏝️

Dynamic Island para Windows: uma pílula preta no topo da tela que se transforma com animação de mola.

| Estado | Quando aparece |
|---|---|
| Mídia | música tocando (Spotify, YouTube no navegador, qualquer player do Windows): vinil girando com a capa no rótulo + equalizador na cor da capa |
| Notificações | notificações de outros apps (WhatsApp, Teams, Outlook…) aparecem na ilha e ficam na aba Notificações |
| Nova faixa | anúncio de 2,5 s com capa, título e artista |
| Volume | ao mudar o volume ou o mudo |
| Toasts | Caps Lock, carregador conectado/desconectado, bateria fraca (20/10/5%), internet caiu/voltou, texto copiado |
| Pomodoro | anel + contagem compacta enquanto roda |
| Expandido (mouse em cima) | abas **Mídia**, **Sistema** (CPU, RAM, GPU NVIDIA, rede), **Foco**, **Clipboard** (histórico, clique para recopiar) e **Estante** (arraste arquivos até a ilha) |

### Painéis (clique dentro da ilha expandida)

- **Cards de Sistema** → detalhe do componente: CPU (modelo, frequência, uso por núcleo), RAM (disponível, paginação), GPU (temperatura, consumo, clock, ventoinha, VRAM) e Rede (interfaces, IPs, tráfego), com os processos que mais usam cada um e atalho para o Gerenciador de Tarefas.
- **Wi-Fi** → redes ao redor com sinal; um clique conecta nas redes salvas, redes novas abrem o painel de senha do Windows.
- **Bateria** → carga, tempo restante, modo de energia (Eficiência / Equilibrado / Desempenho), plano de energia e atalho para a economia de energia.
- **Relógio** → relógio analógico com ponteiro de segundos suave, hora com segundos e calendário do mês.

Fora da pílula os cliques atravessam a janela. A ilha some sozinha em jogo, vídeo em tela cheia ou apresentação. O ícone na bandeja tem **Iniciar com o Windows**, **Recentralizar**, **Recarregar visual**, **Procurar atualizações** e **Sair**.

O Island se atualiza sozinho: procura versão nova ao iniciar e a cada 6 horas. Quando encontra, mostra um aviso e um botão verde com a versão no topo da ilha expandida; um clique baixa, instala e reabre o app.

## Pré-requisitos (uma vez)

1. **Rust**: <https://rustup.rs> (aceite o padrão `x86_64-pc-windows-msvc`)
2. **Visual Studio Build Tools** com a carga “Desenvolvimento para desktop com C++”
   `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
3. **Node.js LTS**: `winget install OpenJS.NodeJS.LTS`
4. WebView2 já vem no Windows 11.

## Rodar e gerar o instalador

```powershell
cd island_windows
npm install
npm run dev      # abre a ilha em modo desenvolvimento
npm run build    # gera o instalador em src-tauri\target\release\bundle\nsis\
```

A primeira compilação demora alguns minutos; as próximas são rápidas.

O `npm run build` assina o instalador para a atualização automática, então precisa da chave privada nas variáveis de ambiente:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "$HOME\.tauri\island.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -Raw "$HOME\.tauri\island.key.password"
```

Sem a chave, compile com `npx tauri build --config '{\"bundle\":{\"createUpdaterArtifacts\":false}}'`.

## Publicar uma versão

1. Suba a versão em `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `package.json` e `src-tauri/msix/AppxManifest.xml` (`X.Y.Z.0`).
2. Faça o commit e crie a tag: `git tag vX.Y.Z && git push origin main vX.Y.Z`.
3. O CI compila, assina e cria o Release com o instalador, o `.sig` e o `latest.json`. Os Islands instalados encontram a versão nova em até 6 horas, ou na hora pelo menu da bandeja.

A chave privada fica em `~\.tauri\island.key` (com a senha em `island.key.password`) e nos secrets `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` do repositório. **Guarde uma cópia:** sem ela não dá para publicar atualizações que os Islands já instalados aceitem.

## Ver o visual sem compilar

Abra `ui\index.html` no navegador: roda em modo demo com dados falsos.
Teclas: `V` volume, `M` mudo, `K` Caps Lock, `C` copiar, `B` carregador, `W` Wi-Fi, `N` próxima faixa, `P` pomodoro, `D` soltar arquivos, `E` expandir, `U` atualização disponível, `Espaço` play/pause.

## Onde mexer

- `ui/style.css`: cores, fontes, raios, brilho (`--glow`).
- `ui/app.js`, objeto `SIZES`: tamanho de cada estado. Molas `sw`, `sh`, `sr`: rigidez/amortecimento (menos amortecimento = mais quique).
- `src-tauri/src/platform/win.rs`: leitura das APIs do Windows.
- `src-tauri/src/main.rs`: frequência de leitura e eventos.

## Notificações de outros apps

O Windows só entrega notificações de outros apps a programas com **identidade de pacote**. O Island usa um *pacote esparso* (`src-tauri/msix/AppxManifest.xml`): o instalador NSIS continua igual e, ao final, registra essa identidade apontando para a pasta instalada.

1. Ligue o **Modo de Desenvolvedor**: Configurações → Sistema → Para desenvolvedores (necessário porque o pacote não é assinado).
2. Instale (ou reinstale) o Island. Se o Modo de Desenvolvedor estava desligado na instalação, abra a aba Notificações e clique em **Ativar**: o Island registra e reinicia sozinho.
3. Na primeira vez, permita o acesso em Configurações → Privacidade → Notificações.

Dica: para não ver o aviso duas vezes, desligue os *banners* do app em Configurações → Sistema → Notificações e deixe só “Mostrar na central de notificações”; a ilha continua recebendo.

## Limitações conhecidas

- **Lista de Wi-Fi no Windows 11 24H2+** exige a permissão de Localização para apps de desktop (Configurações → Privacidade → Localização). A ilha avisa e abre a tela certa.
- **Economia de energia** não tem API pública para ligar/desligar; o botão abre a página do Windows. O modo de energia só tem efeito com o plano Equilibrado ativo.
- **VRAM por processo** não é informada pelo driver NVIDIA no Windows (modo WDDM), por isso aparece “—”.

- O **OSD de volume nativo** do Windows continua aparecendo junto do HUD da ilha.
- **Brilho** não é monitorado.
- GPU só NVIDIA (via NVML); sem placa NVIDIA o card mostra “—”.

## Consumo

Tauri usa o WebView2 do sistema: espere algo entre 60 e 150 MB de RAM somando os processos do WebView2 e CPU perto de zero parado. As animações só rodam enquanto a pílula está mudando de forma.
