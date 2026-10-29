export type KeyAction =
  | { type: 'focus-add' }
  | { type: 'focus-search' }
  | { type: 'move'; delta: 1 | -1 }
  | { type: 'toggle' }
  | { type: 'edit' }
  | { type: 'delete' }
  | { type: 'escape' };

export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  isComposing?: boolean;
  /** 물리 키 코드. 한글 자판에서도 'KeyN'처럼 고정이다. */
  code?: string;
  keyCode?: number;
}

/** text: 글자를 입력하는 칸. control: 버튼·체크박스. none: 그 밖(본문 등). */
export type TargetKind = 'text' | 'control' | 'none';

export function actionForKey(e: KeyInput, target: TargetKind): KeyAction | null {
  // 한글 조합 중에는 어떤 단축키도 가로채지 않는다.
  if (e.isComposing || e.keyCode === 229) return null;

  if (e.ctrlKey) {
    const k = e.key.toLowerCase();
    if (e.code === 'KeyN' || k === 'n') return { type: 'focus-add' };
    if (e.code === 'KeyF' || k === 'f') return { type: 'focus-search' };
    return null;
  }
  if (e.key === 'Escape') return { type: 'escape' };
  if (target === 'text') return null;

  switch (e.key) {
    case 'ArrowDown':
      return { type: 'move', delta: 1 };
    case 'ArrowUp':
      return { type: 'move', delta: -1 };
    case 'Delete':
      return { type: 'delete' };
    case ' ':
      // 버튼·체크박스에서는 브라우저 기본 동작(클릭)을 그대로 둔다. 이중 토글 방지.
      return target === 'control' ? null : { type: 'toggle' };
    case 'Enter':
      return target === 'control' ? null : { type: 'edit' };
    default:
      return null;
  }
}

const CLICKABLE_INPUTS = new Set(['checkbox', 'radio', 'button', 'submit', 'reset']);

export function targetKind(target: EventTarget | null): TargetKind {
  const el = target as { tagName?: unknown; type?: unknown; isContentEditable?: unknown; closest?: unknown } | null;
  if (!el || typeof el.tagName !== 'string') return 'none';
  if (el.isContentEditable === true) return 'text';
  switch (el.tagName) {
    case 'TEXTAREA':
    case 'SELECT':
      return 'text';
    case 'BUTTON':
      // 행을 클릭하면 행 본문 버튼에 포커스가 간다. 그 상태에서도 Space/Enter 단축키가 동작해야 한다.
      return typeof el.closest === 'function' && el.closest('.row') ? 'none' : 'control';
    case 'INPUT':
      return CLICKABLE_INPUTS.has(String(el.type)) ? 'control' : 'text';
    default:
      return 'none';
  }
}
