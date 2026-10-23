// Notificação discreta no canto da tela, com botão "Abrir pasta".

import './styles/theme.css';
import './styles/toast.css';

import { call, inTauri, on, type ToastState } from './api';
import { icon } from './icons';

const VISIBLE_MS = 6000;

const toast = document.getElementById('toast')!;
const glass = toast.querySelector<HTMLElement>('.glass')!;
const iconEl = toast.querySelector<HTMLElement>('.toast-icon')!;
const title = toast.querySelector<HTMLElement>('.toast-title')!;
const detail = toast.querySelector<HTMLElement>('.toast-detail')!;
const note = toast.querySelector<HTMLElement>('.toast-note')!;
const action = toast.querySelector<HTMLButtonElement>('.toast-action')!;
const closeButton = toast.querySelector<HTMLButtonElement>('.toast-close')!;

let path: string | null = null;
let timer = 0;

function show(state: ToastState): void {
  path = state.path;
  iconEl.innerHTML = icon(state.kind === 'error' ? 'alert' : state.kind === 'success' ? 'check' : 'orbit', 20);
  title.textContent = state.title;
  detail.textContent = state.detail;
  detail.title = state.detail;
  note.textContent = state.notes.join(' · ');
  action.hidden = !path;
  glass.style.backgroundImage = state.backdrop ? `url(${state.backdrop})` : '';
  document.documentElement.classList.toggle('no-glass', !state.backdrop);

  toast.className = `kind-${state.kind}`;
  void toast.offsetWidth;
  toast.classList.add('visible');
  schedule();
}

function schedule(): void {
  window.clearTimeout(timer);
  timer = window.setTimeout(hide, VISIBLE_MS);
}

async function hide(): Promise<void> {
  window.clearTimeout(timer);
  toast.classList.remove('visible');
  await new Promise((r) => setTimeout(r, 180));
  await call('hide_toast');
}

toast.addEventListener('mouseenter', () => window.clearTimeout(timer));
toast.addEventListener('mouseleave', schedule);
closeButton.addEventListener('click', () => void hide());
action.addEventListener('click', () => {
  if (path) void call('reveal', { path });
  void hide();
});
document.addEventListener('contextmenu', (e) => e.preventDefault());

void on<ToastState>('toast:show', show).then(() => call('ui_ready'));

if (!inTauri) {
  const params = new URLSearchParams(location.search);
  document.documentElement.classList.add('demo');
  if (params.get('theme')) document.documentElement.dataset.theme = params.get('theme')!;
  show({
    kind: 'success',
    title: 'Arquivo criado',
    detail: 'Relatório anual (comprimido).pdf',
    notes: ['4,1 MB → 0,8 MB'],
    path: 'C:/demo/x.pdf',
    backdrop: null,
  });
  window.clearTimeout(timer);
}
