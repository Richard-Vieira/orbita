// Geometria do menu radial (unidades em pixels CSS da janela).

export const SIZE = 560;
export const C = SIZE / 2;
export const R_HUB = 78;
export const R_IN = 94;
export const R_OUT = 196;
export const R_SATELLITE = 240;
const GAP = 7; // espaço entre fatias
const CORNER = 10; // raio dos cantos arredondados

export interface Point {
  x: number;
  y: number;
}

/** Ângulo em graus (0 = topo, sentido horário) para coordenadas. */
export function polar(radius: number, deg: number): Point {
  const rad = ((deg - 90) * Math.PI) / 180;
  return { x: C + radius * Math.cos(rad), y: C + radius * Math.sin(rad) };
}

export interface SliceGeometry {
  start: number;
  end: number;
  mid: number;
  path: string;
}

/** Fatias igualmente divididas, começando centralizadas no topo. */
export function slices(count: number): SliceGeometry[] {
  const span = 360 / count;
  return Array.from({ length: count }, (_, i) => {
    const mid = i * span;
    const start = mid - span / 2;
    const end = mid + span / 2;
    return { start, end, mid, path: count === 1 ? ringPath() : slicePath(start, end) };
  });
}

const f = (n: number) => n.toFixed(2);

/** Anel inteiro (quando só há uma opção). */
function ringPath(): string {
  const o1 = polar(R_OUT, 0), o2 = polar(R_OUT, 180);
  const i1 = polar(R_IN, 0), i2 = polar(R_IN, 180);
  return [
    `M${f(o1.x)} ${f(o1.y)}`,
    `A${R_OUT} ${R_OUT} 0 1 1 ${f(o2.x)} ${f(o2.y)}`,
    `A${R_OUT} ${R_OUT} 0 1 1 ${f(o1.x)} ${f(o1.y)}Z`,
    `M${f(i1.x)} ${f(i1.y)}`,
    `A${R_IN} ${R_IN} 0 1 0 ${f(i2.x)} ${f(i2.y)}`,
    `A${R_IN} ${R_IN} 0 1 0 ${f(i1.x)} ${f(i1.y)}Z`,
  ].join(' ');
}

/**
 * Setor de anel com cantos arredondados. O espaço entre fatias tem largura
 * constante (GAP), por isso o ângulo descontado é maior no raio interno.
 */
function slicePath(start: number, end: number): string {
  const deg = (r: number) => ((GAP / 2) / r) * (180 / Math.PI);
  const a0o = start + deg(R_OUT), a1o = end - deg(R_OUT);
  const a0i = start + deg(R_IN), a1i = end - deg(R_IN);
  const co = (CORNER / R_OUT) * (180 / Math.PI);
  const ci = (CORNER / R_IN) * (180 / Math.PI);

  const p = (r: number, a: number) => polar(r, a);
  const large = a1o - a0o - 2 * co > 180 ? 1 : 0;

  const outStart = p(R_OUT, a0o + co), outEnd = p(R_OUT, a1o - co);
  const c1 = p(R_OUT, a1o), e1 = p(R_OUT - CORNER, a1o);
  const s2 = p(R_IN + CORNER, a1i), c2 = p(R_IN, a1i), e2 = p(R_IN, a1i - ci);
  const inEnd = p(R_IN, a0i + ci);
  const c3 = p(R_IN, a0i), e3 = p(R_IN + CORNER, a0i);
  const s4 = p(R_OUT - CORNER, a0o), c4 = p(R_OUT, a0o);

  return [
    `M${f(outStart.x)} ${f(outStart.y)}`,
    `A${R_OUT} ${R_OUT} 0 ${large} 1 ${f(outEnd.x)} ${f(outEnd.y)}`,
    `Q${f(c1.x)} ${f(c1.y)} ${f(e1.x)} ${f(e1.y)}`,
    `L${f(s2.x)} ${f(s2.y)}`,
    `Q${f(c2.x)} ${f(c2.y)} ${f(e2.x)} ${f(e2.y)}`,
    `A${R_IN} ${R_IN} 0 ${large} 0 ${f(inEnd.x)} ${f(inEnd.y)}`,
    `Q${f(c3.x)} ${f(c3.y)} ${f(e3.x)} ${f(e3.y)}`,
    `L${f(s4.x)} ${f(s4.y)}`,
    `Q${f(c4.x)} ${f(c4.y)} ${f(outStart.x)} ${f(outStart.y)}Z`,
  ].join(' ');
}

/** Arco de progresso ao longo da borda externa da fatia. */
export function progressArc(start: number, end: number, fraction: number): string {
  const r = R_OUT - 7;
  const pad = ((GAP / 2 + CORNER) / r) * (180 / Math.PI);
  const a0 = start + pad;
  const a1 = a0 + (end - pad - a0) * Math.min(Math.max(fraction, 0.001), 1);
  const s = polar(r, a0), e = polar(r, a1);
  const large = a1 - a0 > 180 ? 1 : 0;
  return `M${f(s.x)} ${f(s.y)} A${r} ${r} 0 ${large} 1 ${f(e.x)} ${f(e.y)}`;
}
