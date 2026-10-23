// Menu radial: abre em leque a partir do centro, mostra o progresso na fatia
// e, ao terminar, o arquivo novo "surge" ao lado. Clicar nele abre o menu do
// arquivo novo (encadear: JPG -> PDF -> Comprimir).

import './styles/theme.css';
import './styles/menu.css';

import { call, demoState, inTauri, on, type Action, type FileInfo, type JobDone, type JobFailed, type JobProgress, type MenuState } from './api';
import { C, R_HUB, R_IN, R_OUT, R_SATELLITE, SIZE, polar, progressArc, slices, type SliceGeometry } from './geometry';
import { icon, iconPaths } from './icons';

const SVG_NS = 'http://www.w3.org/2000/svg';
const stage = document.getElementById('stage')!;
const ring = document.getElementById('ring') as unknown as SVGSVGElement;
const hub = document.getElementById('hub')!;
const satellite = document.getElementById('satellite')!;
const message = document.getElementById('message')!;

interface Busy {
  jobId: number | null;
  index: number;
}

let state: MenuState | null = null;
let geometry: SliceGeometry[] = [];
let busy: Busy | null = null;
let closing = false;
let messageTimer = 0;

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

const number = new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 1 });

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${number.format(value)} ${units[unit]}`;
}

function kindIcon(file: FileInfo): string {
  if (file.isDir) return 'folder';
  return { pdf: 'pdf', image: 'jpg', text: 'txt', icon: 'ico' }[file.kind] ?? 'file';
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number> = {}): SVGElementTagNameMap[K] {
  const el = document.createElementNS(SVG_NS, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

const nextFrame = () => new Promise<void>((r) => requestAnimationFrame(() => r()));

// ---------------------------------------------------------------------------
// Desenho
// ---------------------------------------------------------------------------

function render(next: MenuState, animate: boolean): void {
  state = next;
  busy = null;
  geometry = slices(Math.max(next.actions.length, 1));
  document.documentElement.classList.toggle('no-glass', !next.glass || !next.backdrop);

  ring.replaceChildren();
  ring.setAttribute('viewBox', `0 0 ${SIZE} ${SIZE}`);
  const defs = svg('defs');
  ring.append(defs);

  // Centro (vidro).
  defs.append(clip('clip-hub', svg('circle', { cx: C, cy: C, r: R_HUB })));
  const hubGroup = svg('g', { class: 'hub-disc' });
  hubGroup.append(glass('clip-hub'), svg('circle', { class: 'surface', cx: C, cy: C, r: R_HUB }));
  ring.append(hubGroup);

  if (next.actions.length === 0) {
    const empty = svg('g', { class: 'slice disabled', style: '--i:0' });
    defs.append(clip('clip-empty', svg('path', { d: geometry[0].path, 'fill-rule': 'evenodd' })));
    empty.append(glass('clip-empty'), svg('path', { class: 'surface', d: geometry[0].path, 'fill-rule': 'evenodd' }));
    ring.append(empty);
  }

  next.actions.forEach((action, i) => ring.append(sliceElement(action, geometry[i], i, defs)));

  renderHub(next.files);
  hideSatellite();
  if (next.actions.length === 0) showMessage('Nenhuma conversão disponível para esta seleção', 'error', 8000);
  stage.classList.toggle('animate', animate);
  if (animate) {
    stage.classList.remove('open');
    void stage.offsetWidth; // reinicia a animação
  }
  stage.classList.add('open');
}

function clip(id: string, shape: SVGElement): SVGClipPathElement {
  const el = svg('clipPath', { id });
  el.append(shape);
  return el;
}

/** Fundo capturado da tela, já desfocado, recortado no formato do elemento. */
function glass(clipId: string): SVGElement {
  if (!state?.backdrop) return svg('g');
  return svg('image', {
    class: 'backdrop',
    href: state.backdrop,
    x: 0,
    y: 0,
    width: SIZE,
    height: SIZE,
    preserveAspectRatio: 'none',
    'clip-path': `url(#${clipId})`,
  });
}

function sliceElement(action: Action, g: SliceGeometry, i: number, defs: SVGDefsElement): SVGGElement {
  const group = svg('g', { class: 'slice', role: 'button', tabindex: 0, 'aria-label': action.label, style: `--i:${i}` });
  const clipId = `clip-${i}`;
  defs.append(clip(clipId, svg('path', { d: g.path, 'fill-rule': 'evenodd' })));

  const content = polar((R_IN + R_OUT) / 2, g.mid);
  const iconGroup = svg('g', { class: 'slice-icon', transform: `translate(${content.x - 12} ${content.y - 19})` });
  iconGroup.innerHTML = `<g fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">${iconPaths(action.icon)}</g>`;
  const label = svg('text', { class: 'slice-label', x: content.x, y: content.y + 20, 'text-anchor': 'middle' });
  label.textContent = action.label;

  group.append(
    glass(clipId),
    svg('path', { class: 'surface', d: g.path, 'fill-rule': 'evenodd' }),
    svg('path', { class: 'progress', d: progressArc(g.start, g.end, 0) }),
    iconGroup,
    label,
  );
  group.addEventListener('click', (e) => {
    e.stopPropagation();
    void runAction(i);
  });
  group.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ' ') void runAction(i);
  });
  return group;
}

function renderHub(files: FileInfo[]): void {
  const single = files.length === 1 ? files[0] : null;
  const title = single ? single.name : `${files.length} arquivos`;
  const total = files.reduce((sum, f) => sum + f.size, 0);
  const iconName = single ? kindIcon(single) : 'files';
  hub.innerHTML = `
    <div class="hub-icon">${icon(iconName, 34)}</div>
    <div class="hub-name" title="${escapeHtml(title)}">${escapeHtml(title)}</div>
    <div class="hub-meta">${single?.isDir ? 'pasta' : formatSize(total)}</div>
  `;
}

/**
 * Mensagem curta numa pílula acima ou abaixo do anel: embaixo por padrão, em
 * cima quando o arquivo novo (`awayFrom`, em graus) surgiu na metade de baixo.
 */
function showMessage(text: string, tone: 'error' | 'ok', duration = 4500, awayFrom?: number): void {
  const satelliteBelow = awayFrom !== undefined && Math.cos((awayFrom * Math.PI) / 180) < -0.2;
  message.textContent = text;
  message.className = `visible ${tone}`;
  message.style.left = `${C}px`;
  message.style.top = `${satelliteBelow ? C - R_OUT - 34 : C + R_OUT + 34}px`;
  window.clearTimeout(messageTimer);
  messageTimer = window.setTimeout(hideMessage, duration);
}

function hideMessage(): void {
  message.className = '';
}

function sliceAt(i: number): SVGGElement | null {
  return ring.querySelectorAll<SVGGElement>('.slice')[i] ?? null;
}

// ---------------------------------------------------------------------------
// Conversão
// ---------------------------------------------------------------------------

async function runAction(i: number): Promise<void> {
  if (!state || busy) return;
  void call('ui_log', { message: `ação ${state.actions[i].id}` });
  const action = state.actions[i];
  busy = { jobId: null, index: i };
  stage.classList.add('busy');
  sliceAt(i)?.classList.add('running');
  setProgress(0);
  hideSatellite();

  try {
    const jobId = await call<number>('run_action', { action: action.id, files: state.files.map((f) => f.path) });
    if (busy) busy.jobId = jobId;
    if (!inTauri) demoRun(jobId);
  } catch (e) {
    fail(String(e));
  }
}

function setProgress(fraction: number): void {
  if (!busy) return;
  const el = sliceAt(busy.index);
  const g = geometry[busy.index];
  el?.querySelector('.progress')?.setAttribute('d', progressArc(g.start, g.end, fraction));
  const label = el?.querySelector('.slice-label');
  if (label) label.textContent = `${Math.round(fraction * 100)}%`;
}

function finishBusy(): number | null {
  if (!busy || !state) return null;
  const index = busy.index;
  const el = sliceAt(index);
  el?.classList.remove('running');
  const label = el?.querySelector('.slice-label');
  if (label) label.textContent = state.actions[index].label;
  el?.querySelector('.progress')?.setAttribute('d', progressArc(0, 0, 0));
  stage.classList.remove('busy');
  busy = null;
  return index;
}

function done(result: JobDone): void {
  const index = finishBusy();
  if (index === null || !state) return;
  const el = sliceAt(index);
  el?.classList.add('success');
  window.setTimeout(() => el?.classList.remove('success'), 900);

  const first = result.created[0];
  if (!first) return;
  const extra = result.created.length > 1 ? ` (+${result.created.length - 1})` : '';
  const angle = geometry[index].mid;
  showMessage(result.notes[0] ?? `${first.name}${extra} criado`, result.notes.length ? 'error' : 'ok', 6000, angle);
  showSatellite(first, angle);
}

function fail(message: string): void {
  const index = finishBusy();
  if (index !== null) {
    const el = sliceAt(index);
    el?.classList.add('shake');
    window.setTimeout(() => el?.classList.remove('shake'), 500);
  }
  showMessage(message, 'error');
}

// ---------------------------------------------------------------------------
// Arquivo novo ao lado ("satélite") e encadeamento
// ---------------------------------------------------------------------------

function showSatellite(file: FileInfo, angle: number): void {
  const from = polar((R_IN + R_OUT) / 2, angle);
  const to = polar(R_SATELLITE, angle);
  const ext = file.isDir ? 'pasta' : (file.name.split('.').pop() ?? '').toUpperCase();
  satellite.innerHTML = `${icon(kindIcon(file), 22)}<span>${escapeHtml(ext)}</span>`;
  satellite.title = file.isDir ? `Abrir a pasta ${file.name}` : `Converter ${file.name}`;
  satellite.style.setProperty('--from-x', `${from.x}px`);
  satellite.style.setProperty('--from-y', `${from.y}px`);
  satellite.style.setProperty('--to-x', `${to.x}px`);
  satellite.style.setProperty('--to-y', `${to.y}px`);
  satellite.classList.remove('visible', 'to-center');
  void satellite.offsetWidth;
  satellite.classList.add('visible');
  satellite.onclick = (e) => {
    e.stopPropagation();
    if (file.isDir) {
      void call('reveal', { path: file.path });
    } else {
      void chain(file);
    }
  };
}

function hideSatellite(): void {
  satellite.classList.remove('visible', 'to-center');
  satellite.onclick = null;
}

async function chain(file: FileInfo): Promise<void> {
  if (!state) return;
  satellite.classList.add('to-center');
  stage.classList.add('leaving');
  const [next] = await Promise.all([
    call<MenuState>('menu_for', { files: [file.path] }),
    new Promise((r) => setTimeout(r, 200)),
  ]);
  stage.classList.remove('leaving');
  render({ ...next, backdrop: state.backdrop, glass: state.glass }, true);
}

// ---------------------------------------------------------------------------
// Abrir e fechar
// ---------------------------------------------------------------------------

function open(next: MenuState): void {
  closing = false;
  window.clearTimeout(messageTimer);
  hideMessage();
  render(next, true);
  window.focus();
}

async function close(reason: string): Promise<void> {
  if (closing || !state) return;
  closing = true;
  void call('ui_log', { message: `fechando: ${reason}` });
  stage.classList.add('closing');
  await new Promise((r) => setTimeout(r, 120));
  // Limpa tudo antes de esconder: evita que a próxima abertura mostre
  // por um instante o quadro antigo.
  stage.classList.remove('open', 'closing', 'busy', 'leaving');
  ring.replaceChildren();
  hub.innerHTML = '';
  hideMessage();
  hideSatellite();
  state = null;
  busy = null;
  await nextFrame();
  await nextFrame();
  await call('hide_menu');
}

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') void close('esc');
});
// Clique fora do anel (área transparente da janela) fecha.
document.addEventListener('click', (e) => {
  const target = e.target as Element;
  if (!target.closest('.slice, #hub, #satellite')) {
    const dx = e.clientX - C, dy = e.clientY - C;
    const r = Math.hypot(dx, dy);
    if (r > R_OUT + 4 || (r > R_HUB && r < R_IN - 4)) void close(`clique fora (${Math.round(e.clientX)}, ${Math.round(e.clientY)}) em ${(target as Element).tagName}`);
  }
});
// A notificação pode pegar o foco por um instante (no Linux o app o devolve
// logo em seguida): só fecha se o foco não voltar.
window.addEventListener('blur', () => {
  window.setTimeout(() => {
    if (!document.hasFocus()) void close('perdeu o foco');
  }, 150);
});
document.addEventListener('contextmenu', (e) => e.preventDefault());

void Promise.all([
  on<MenuState>('menu:open', open),
  on<JobProgress>('job:progress', (p) => busy?.jobId === p.jobId && setProgress(p.progress)),
  on<JobDone>('job:done', (d) => busy?.jobId === d.jobId && done(d)),
  on<JobFailed>('job:error', (f) => busy?.jobId === f.jobId && fail(f.message)),
]).then(() => call('ui_ready'));

// ---------------------------------------------------------------------------
// Demonstração no navegador: ?demo=pdf | image  (&theme=dark)
// ---------------------------------------------------------------------------

function demoRun(jobId: number): void {
  let p = 0;
  const timer = window.setInterval(() => {
    p += 0.12;
    if (p < 1) return setProgress(p);
    window.clearInterval(timer);
    const created: FileInfo = { path: 'C:/demo/ferias.pdf', name: 'ferias.pdf', size: 1_900_000, kind: 'pdf', isDir: false };
    done({ jobId, created: [created], notes: [] });
  }, 90);
}

/** Fundo falso (já reduzido, como o que vem do Rust) para testar o vidro. */
function demoBackdrop(): string {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 93;
  const g = canvas.getContext('2d')!;
  g.fillStyle = '#1e88e5';
  g.fillRect(0, 0, 93, 93);
  g.fillStyle = '#fff';
  g.fillRect(40, 0, 53, 50);
  g.fillStyle = '#222';
  for (let y = 4; y < 48; y += 5) g.fillRect(44, y, 40, 2);
  return canvas.toDataURL('image/png');
}

if (!inTauri) {
  const params = new URLSearchParams(location.search);
  document.documentElement.classList.add('demo');
  if (params.get('theme')) document.documentElement.dataset.theme = params.get('theme')!;
  const demo = demoState(params.get('demo') ?? 'pdf');
  open({ ...demo, backdrop: params.get('glass') ? demoBackdrop() : null, glass: true });
  const hover = params.get('hover');
  if (hover !== null) sliceAt(Number(hover))?.classList.add('hover');
  if (params.get('done') !== null) {
    busy = { jobId: 0, index: Number(params.get('done')) || 0 };
    done({ jobId: 0, created: [{ path: 'C:/demo/ferias.pdf', name: 'ferias.pdf', size: 1_900_000, kind: 'pdf', isDir: false }], notes: [] });
  }
}
