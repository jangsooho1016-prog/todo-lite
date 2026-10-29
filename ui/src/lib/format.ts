import type { SyncStatus, Task } from './types';

const MS_PER_DAY = 86_400_000;
const pad = (n: number) => String(n).padStart(2, '0');

function dayNumber(iso: string): number {
  const [y, m, d] = iso.split('-').map(Number);
  return Date.UTC(y, m - 1, d) / MS_PER_DAY;
}

export function todayIso(now: Date = new Date()): string {
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export function nowParts(now: Date = new Date()): { date: string; time: string } {
  return { date: todayIso(now), time: `${pad(now.getHours())}:${pad(now.getMinutes())}` };
}

/** "15:00:00" → "15:00". `<input type="time">`의 value로 쓴다. */
export function timeInputValue(time: string | null): string {
  return time ? time.slice(0, 5) : '';
}

export function formatDue(dueDate: string | null, dueTime: string | null, today: string): string {
  if (!dueDate) return '';
  const diff = dayNumber(dueDate) - dayNumber(today);
  let label: string;
  if (diff === 0) label = '오늘';
  else if (diff === 1) label = '내일';
  else if (diff === -1) label = '어제';
  else {
    const [y, m, d] = dueDate.split('-').map(Number);
    label = y === Number(today.slice(0, 4)) ? `${m}/${d}` : `${y}/${m}/${d}`;
  }
  const time = timeInputValue(dueTime);
  return time ? `${label} ${time}` : label;
}

export function isOverdue(
  task: Pick<Task, 'due_date' | 'due_time' | 'completed_at'>,
  now: { date: string; time: string },
): boolean {
  if (task.completed_at || !task.due_date) return false;
  if (task.due_date < now.date) return true;
  return task.due_date === now.date && !!task.due_time && timeInputValue(task.due_time) < now.time;
}

export function parseTagInput(text: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of text.split(/[\s,]+/)) {
    const tag = raw.replace(/^#+/, '').trim();
    if (!tag) continue;
    const key = tag.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(tag);
  }
  return out;
}

export const tagsToText = (tags: string[]): string => tags.map((t) => `#${t}`).join(' ');

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

export function describeSync(s: SyncStatus, time: string): string {
  const base = `${time} 동기화 · 가져옴 ${s.applied} · 보냄 ${s.exported}`;
  return s.corrupt_lines > 0 ? `${base} · 손상된 줄 ${s.corrupt_lines}개 건너뜀` : base;
}
