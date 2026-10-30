import { describe, expect, it } from 'vitest';
import { acceleratorFromEvent } from './accelerator';

const ev = (code: string, mods: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe('acceleratorFromEvent', () => {
  it('formats Ctrl+Alt+Space', () => {
    expect(acceleratorFromEvent(ev('Space', { ctrlKey: true, altKey: true }))).toBe('Ctrl+Alt+Space');
  });

  it('orders modifiers Ctrl, Alt, Shift regardless of how they were pressed', () => {
    expect(acceleratorFromEvent(ev('KeyA', { shiftKey: true, ctrlKey: true }))).toBe('Ctrl+Shift+A');
    expect(acceleratorFromEvent(ev('KeyZ', { shiftKey: true, altKey: true, ctrlKey: true }))).toBe('Ctrl+Alt+Shift+Z');
  });

  it('supports function keys and digits', () => {
    expect(acceleratorFromEvent(ev('F1', { altKey: true }))).toBe('Alt+F1');
    expect(acceleratorFromEvent(ev('F12', { altKey: true }))).toBe('Alt+F12');
    expect(acceleratorFromEvent(ev('Digit1', { ctrlKey: true }))).toBe('Ctrl+1');
  });

  it('returns null when only modifiers are pressed', () => {
    for (const code of ['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight']) {
      expect(acceleratorFromEvent(ev(code, { ctrlKey: true, altKey: true, shiftKey: true }))).toBeNull();
    }
  });

  it('returns null without Ctrl/Alt/Shift', () => {
    expect(acceleratorFromEvent(ev('KeyA'))).toBeNull();
    expect(acceleratorFromEvent(ev('F5'))).toBeNull();
  });

  it('returns null when the Meta (Windows) key is held', () => {
    expect(acceleratorFromEvent(ev('KeyA', { ctrlKey: true, metaKey: true }))).toBeNull();
  });

  it('returns null for unsupported keys', () => {
    for (const code of ['Enter', 'F13', 'Numpad1', 'Minus', 'Escape', 'Tab', 'ArrowUp']) {
      expect(acceleratorFromEvent(ev(code, { ctrlKey: true }))).toBeNull();
    }
  });
});
