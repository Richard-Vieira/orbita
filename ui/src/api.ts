// Ponte com o lado Rust. Fora do Tauri (navegador comum) entra o modo de
// demonstração, usado para desenvolver o visual e gerar capturas de tela.

import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  kind: string;
  isDir: boolean;
}

export interface Action {
  id: string;
  label: string;
  icon: string;
}

export interface MenuState {
  files: FileInfo[];
  actions: Action[];
  backdrop: string | null;
  glass: boolean;
}

export interface JobProgress {
  jobId: number;
  progress: number;
}

export interface JobDone {
  jobId: number;
  created: FileInfo[];
  notes: string[];
}

export interface JobFailed {
  jobId: number;
  message: string;
}

export interface ToastState {
  kind: 'success' | 'error' | 'info';
  title: string;
  detail: string;
  notes: string[];
  path: string | null;
  backdrop: string | null;
}

export const inTauri = '__TAURI_INTERNALS__' in window;

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return getCurrentWebviewWindow().listen<T>(event, (e) => handler(e.payload));
}

export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return demoCall<T>(command, args);
  return invoke<T>(command, args);
}

// ---------------------------------------------------------------------------
// Modo de demonstração
// ---------------------------------------------------------------------------

const demoActions: Record<string, Action[]> = {
  pdf: [
    { id: 'pdf.png', label: 'PNG', icon: 'png' },
    { id: 'pdf.jpg', label: 'JPG', icon: 'jpg' },
    { id: 'pdf.txt', label: 'Texto', icon: 'txt' },
    { id: 'pdf.compress', label: 'Comprimir', icon: 'compress' },
  ],
  image: [
    { id: 'img.png', label: 'PNG', icon: 'png' },
    { id: 'img.webp', label: 'WEBP', icon: 'webp' },
    { id: 'img.ico', label: 'Ícone', icon: 'ico' },
    { id: 'img.pdf', label: 'PDF', icon: 'pdf' },
  ],
};

export function demoState(kind: string): MenuState {
  const files: FileInfo[] =
    kind === 'image'
      ? [{ path: 'C:/demo/ferias.jpg', name: 'ferias.jpg', size: 2_480_000, kind: 'image', isDir: false }]
      : [{ path: 'C:/demo/Relatório anual.pdf', name: 'Relatório anual.pdf', size: 4_120_000, kind: 'pdf', isDir: false }];
  return { files, actions: demoActions[kind] ?? demoActions.pdf, backdrop: null, glass: true };
}

async function demoCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (command === 'menu_for') {
    const path = String((args?.files as string[])[0]);
    return demoState(path.endsWith('.pdf') ? 'pdf' : 'image') as T;
  }
  return 1 as T;
}
