// Ícones de linha próprios do Órbita (24×24, traço 1.5). Sem logos de marcas.

const FILE = '<path d="M14 3.5H7.5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2V8z"/><path d="M14 3.5V8h4.5"/>';
const FRAME = '<rect x="3.5" y="4.5" width="17" height="15" rx="2.5"/>';

const ICONS: Record<string, string> = {
  file: FILE,
  pdf: `${FILE}<path d="M9 12.5h6M9 15.5h6M9 9.5h2"/>`,
  jpg: `${FRAME}<circle cx="9" cy="9.75" r="1.5"/><path d="M20.5 15.5l-4.3-4.3a1.2 1.2 0 0 0-1.7 0L7 18.8"/>`,
  png: `${FRAME}<path d="M3.5 12H12v7.5M12 4.5V12h8.5"/>`,
  webp: `${FRAME}<path d="M3.5 14.5c2.8-2.6 5.7-2.6 8.5 0s5.7 2.6 8.5 0"/><circle cx="15.5" cy="8.75" r="1.25"/>`,
  ico: '<rect x="4" y="4" width="16" height="16" rx="4.5"/><rect x="8.5" y="8.5" width="7" height="7" rx="2"/>',
  txt: '<path d="M5 6.5h14M5 10.5h14M5 14.5h10M5 18.5h6"/>',
  compress:
    '<path d="M12 3.5v5.5M9.25 6.5 12 9.25l2.75-2.75M12 20.5V15M9.25 17.5 12 14.75l2.75 2.75M5 12h14"/>',
  merge:
    '<rect x="3.5" y="3.5" width="7" height="9" rx="1.5"/><rect x="13.5" y="3.5" width="7" height="9" rx="1.5"/><path d="M7 12.5v1.75a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V12.5M12 16.25v4.25M10 18.75l2 2 2-2"/>',
  folder: '<path d="M3.5 7.5a2 2 0 0 1 2-2h3.75l2 2.25h7.25a2 2 0 0 1 2 2v7.75a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  files:
    '<path d="M15 6.5H8.5a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h7.5a2 2 0 0 0 2-2V9.5z"/><path d="M9.5 3.5h6l5 5v9"/>',
  check: '<path d="M5.5 12.5 10 17l8.5-9.5"/>',
  alert: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.75v5M12 16.25v.01"/>',
  orbit:
    '<circle cx="12" cy="12" r="2.75"/><ellipse cx="12" cy="12" rx="9" ry="4.25" transform="rotate(-28 12 12)"/><circle cx="19.2" cy="7.6" r="1.1" fill="currentColor"/>',
};

/** Conteúdo SVG (sem a tag <svg>) do ícone pedido. */
export function iconPaths(name: string): string {
  return ICONS[name] ?? ICONS.file;
}

/** Ícone completo, pronto para inserir no HTML. */
export function icon(name: string, size = 24): string {
  return `<svg class="icon" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${iconPaths(name)}</svg>`;
}
