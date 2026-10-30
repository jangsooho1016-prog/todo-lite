import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  api: {
    addTask: vi.fn(),
    deleteTask: vi.fn().mockResolvedValue(undefined),
    editTask: vi.fn(),
    listTags: vi.fn().mockResolvedValue([]),
    listTasks: vi.fn().mockResolvedValue([]),
    ready: vi.fn().mockResolvedValue(undefined),
    setCompleted: vi.fn(),
    setSyncFolder: vi.fn(),
    syncFolder: vi.fn().mockResolvedValue(null),
    syncNow: vi.fn(),
  },
}));

import { api } from './api';
import { app } from './state.svelte';
import type { Task } from './types';

const task = { id: 't1', title: '보고서 쓰기' } as Task;

beforeEach(() => {
  vi.clearAllMocks();
  app.answerConfirm(false);
  app.tasks = [task];
  app.selectedId = task.id;
});

describe('askConfirm / answerConfirm', () => {
  it('resolves true on answerConfirm(true) and closes', async () => {
    const p = app.askConfirm('정말?');
    expect(app.confirmState?.message).toBe('정말?');
    app.answerConfirm(true);
    expect(await p).toBe(true);
    expect(app.confirmState).toBeNull();
  });

  it('resolves false on answerConfirm(false) and closes', async () => {
    const p = app.askConfirm('정말?');
    app.answerConfirm(false);
    expect(await p).toBe(false);
    expect(app.confirmState).toBeNull();
  });

  it('asking twice resolves the first with false and keeps the second pending', async () => {
    const first = app.askConfirm('하나');
    const second = app.askConfirm('둘');
    expect(await first).toBe(false);
    expect(app.confirmState?.message).toBe('둘');
    let settled = false;
    void second.then(() => (settled = true));
    await Promise.resolve();
    expect(settled).toBe(false);
    app.answerConfirm(true);
    expect(await second).toBe(true);
  });

  it('answerConfirm with nothing open is a no-op', () => {
    expect(() => app.answerConfirm(true)).not.toThrow();
    expect(app.confirmState).toBeNull();
  });
});

describe('remove asks before deleting (regression: "아니요" still deleted)', () => {
  it('does not delete when the answer is no', async () => {
    const p = app.remove(task.id);
    expect(app.confirmState).not.toBeNull();
    expect(api.deleteTask).not.toHaveBeenCalled();
    app.answerConfirm(false);
    await p;
    expect(api.deleteTask).not.toHaveBeenCalled();
    expect(app.selectedId).toBe(task.id);
  });

  it('deletes and clears the selection when the answer is yes', async () => {
    const p = app.remove(task.id);
    expect(api.deleteTask).not.toHaveBeenCalled();
    app.answerConfirm(true);
    await p;
    expect(api.deleteTask).toHaveBeenCalledTimes(1);
    expect(api.deleteTask).toHaveBeenCalledWith(task.id);
    expect(app.selectedId).toBeNull();
  });
});
