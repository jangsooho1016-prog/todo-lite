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
  // 수정자 없이는 타이핑을 가로채게 된다. 글자·숫자·Space는 수정자 하나로도 흔한 입력(Shift+A, Ctrl+C)을
  // 가로채므로 두 개 이상을 요구하고, F1–F12는 하나면 된다.
  const isFunctionKey = /^F\d+$/.test(key);
  if (parts.length < (isFunctionKey ? 1 : 2)) return null;
  return [...parts, key].join('+');
}
