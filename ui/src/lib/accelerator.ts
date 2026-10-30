/** 키 입력을 백엔드가 받는 단축키 문자열("Ctrl+Alt+Space")로 바꾼다. 쓸 수 없는 입력이면 null. */
export interface KeyLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey?: boolean;
}

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-2])$/.test(code)) return code;
  if (code === 'Space') return 'Space';
  return null;
}

export function acceleratorFromEvent(e: KeyLike): string | null {
  if (e.metaKey) return null; // Super/Win 조합은 지원하지 않는다
  const key = keyName(e.code);
  if (!key) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (parts.length === 0) return null; // 수정자 없이는 타이핑을 가로채게 된다
  return [...parts, key].join('+');
}
