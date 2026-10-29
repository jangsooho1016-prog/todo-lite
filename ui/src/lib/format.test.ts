import { describe, expect, it } from 'vitest';
import {
  describeSync,
  errorText,
  formatDue,
  isOverdue,
  nowParts,
  parseTagInput,
  tagsToText,
  timeInputValue,
  todayIso,
} from './format';

const TODAY = '2026-10-28';

describe('todayIso / nowParts', () => {
  it('uses the local calendar date', () => {
    const d = new Date(2026, 9, 28, 15, 5);
    expect(todayIso(d)).toBe('2026-10-28');
    expect(nowParts(d)).toEqual({ date: '2026-10-28', time: '15:05' });
  });
});

describe('formatDue', () => {
  it('returns empty when there is no date', () => {
    expect(formatDue(null, '15:00:00', TODAY)).toBe('');
  });
  it('uses relative words for nearby days', () => {
    expect(formatDue('2026-10-28', null, TODAY)).toBe('오늘');
    expect(formatDue('2026-10-29', null, TODAY)).toBe('내일');
    expect(formatDue('2026-10-27', null, TODAY)).toBe('어제');
  });
  it('uses M/D within the year and Y/M/D otherwise', () => {
    expect(formatDue('2026-10-30', null, TODAY)).toBe('10/30');
    expect(formatDue('2027-01-05', null, TODAY)).toBe('2027/1/5');
  });
  it('appends HH:MM and trims seconds', () => {
    expect(formatDue('2026-10-28', '15:00:00', TODAY)).toBe('오늘 15:00');
    expect(formatDue('2026-10-29', '09:30', TODAY)).toBe('내일 09:30');
  });
});

describe('isOverdue', () => {
  const now = { date: '2026-10-28', time: '15:00' };
  it('is false for done tasks and tasks without a date', () => {
    expect(isOverdue({ due_date: '2026-10-01', due_time: null, completed_at: 'x' }, now)).toBe(false);
    expect(isOverdue({ due_date: null, due_time: null, completed_at: null }, now)).toBe(false);
  });
  it('is true for past dates', () => {
    expect(isOverdue({ due_date: '2026-10-27', due_time: null, completed_at: null }, now)).toBe(true);
  });
  it('compares the time only on the same day', () => {
    expect(isOverdue({ due_date: '2026-10-28', due_time: '09:00:00', completed_at: null }, now)).toBe(true);
    expect(isOverdue({ due_date: '2026-10-28', due_time: '19:00:00', completed_at: null }, now)).toBe(false);
    expect(isOverdue({ due_date: '2026-10-28', due_time: null, completed_at: null }, now)).toBe(false);
  });
});

describe('time and tag helpers', () => {
  it('timeInputValue trims seconds', () => {
    expect(timeInputValue('15:00:00')).toBe('15:00');
    expect(timeInputValue(null)).toBe('');
  });
  it('parseTagInput splits on spaces/commas, strips # and dedupes case-insensitively', () => {
    expect(parseTagInput('#학교, 개인 #학교 Work work')).toEqual(['학교', '개인', 'Work']);
    expect(parseTagInput('  # ,, ')).toEqual([]);
  });
  it('tagsToText prefixes #', () => {
    expect(tagsToText(['학교', '개인'])).toBe('#학교 #개인');
    expect(tagsToText([])).toBe('');
  });
});

describe('errorText', () => {
  it('handles strings, Errors and unknown values', () => {
    expect(errorText('제목을 입력해 주세요')).toBe('제목을 입력해 주세요');
    expect(errorText(new Error('boom'))).toBe('boom');
    expect(errorText(42)).toBe('42');
  });
});

describe('describeSync', () => {
  it('summarises counts', () => {
    expect(describeSync({ configured: true, exported: 2, applied: 1, corrupt_lines: 0 }, '15:05')).toBe(
      '15:05 동기화 · 가져옴 1 · 보냄 2',
    );
  });
  it('mentions skipped corrupt lines', () => {
    expect(describeSync({ configured: true, exported: 0, applied: 0, corrupt_lines: 3 }, '09:00')).toBe(
      '09:00 동기화 · 가져옴 0 · 보냄 0 · 손상된 줄 3개 건너뜀',
    );
  });
});
