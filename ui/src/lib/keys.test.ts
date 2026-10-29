import { describe, expect, it } from 'vitest';
import { actionForKey, targetKind } from './keys';

const key = (k: string, extra: Partial<{ ctrlKey: boolean; isComposing: boolean }> = {}) => ({
  key: k,
  ctrlKey: false,
  ...extra,
});

describe('actionForKey', () => {
  it('Ctrl+N focuses quick add and Ctrl+F focuses search, even while typing', () => {
    for (const kind of ['none', 'text', 'control'] as const) {
      expect(actionForKey(key('n', { ctrlKey: true }), kind)).toEqual({ type: 'focus-add' });
      expect(actionForKey(key('F', { ctrlKey: true }), kind)).toEqual({ type: 'focus-search' });
    }
    expect(actionForKey(key('x', { ctrlKey: true }), 'none')).toBeNull();
  });

  it('Escape always works', () => {
    expect(actionForKey(key('Escape'), 'text')).toEqual({ type: 'escape' });
    expect(actionForKey(key('Escape'), 'none')).toEqual({ type: 'escape' });
  });

  it('ignores everything while IME composition is active', () => {
    expect(actionForKey(key('Enter', { isComposing: true }), 'none')).toBeNull();
    expect(actionForKey(key('n', { ctrlKey: true, isComposing: true }), 'none')).toBeNull();
  });

  it('list keys work when nothing text-like is focused', () => {
    expect(actionForKey(key('ArrowDown'), 'none')).toEqual({ type: 'move', delta: 1 });
    expect(actionForKey(key('ArrowUp'), 'none')).toEqual({ type: 'move', delta: -1 });
    expect(actionForKey(key(' '), 'none')).toEqual({ type: 'toggle' });
    expect(actionForKey(key('Enter'), 'none')).toEqual({ type: 'edit' });
    expect(actionForKey(key('Delete'), 'none')).toEqual({ type: 'delete' });
    expect(actionForKey(key('a'), 'none')).toBeNull();
  });

  it('does not steal keys from text fields', () => {
    for (const k of ['ArrowDown', 'ArrowUp', ' ', 'Enter', 'Delete']) {
      expect(actionForKey(key(k), 'text')).toBeNull();
    }
  });

  it('on buttons/checkboxes, arrows and Delete work but Space/Enter stay native', () => {
    expect(actionForKey(key('ArrowDown'), 'control')).toEqual({ type: 'move', delta: 1 });
    expect(actionForKey(key('Delete'), 'control')).toEqual({ type: 'delete' });
    expect(actionForKey(key(' '), 'control')).toBeNull();
    expect(actionForKey(key('Enter'), 'control')).toBeNull();
  });
});

describe('targetKind', () => {
  const el = (tagName: string, extra: Record<string, unknown> = {}) => ({ tagName, ...extra }) as unknown as EventTarget;

  it('classifies text-like fields', () => {
    expect(targetKind(el('TEXTAREA'))).toBe('text');
    expect(targetKind(el('SELECT'))).toBe('text');
    expect(targetKind(el('INPUT', { type: 'text' }))).toBe('text');
    expect(targetKind(el('INPUT', { type: 'search' }))).toBe('text');
    expect(targetKind(el('INPUT', { type: 'date' }))).toBe('text');
    expect(targetKind(el('DIV', { isContentEditable: true }))).toBe('text');
  });
  it('classifies clickable controls', () => {
    expect(targetKind(el('BUTTON'))).toBe('control');
    expect(targetKind(el('INPUT', { type: 'checkbox' }))).toBe('control');
  });
  it('treats everything else as none', () => {
    expect(targetKind(el('DIV'))).toBe('none');
    expect(targetKind(null)).toBe('none');
  });
});
