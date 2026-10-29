import { invoke } from '@tauri-apps/api/core';
import type { ListQuery, ParsedTask, SyncStatus, TagCount, Task, TaskEdit } from './types';

export const api = {
  preview: (input: string) => invoke<ParsedTask>('preview_input', { input }),
  addTask: (input: string) => invoke<Task>('add_task', { input }),
  listTasks: (query: ListQuery) => invoke<Task[]>('list_tasks', { query }),
  listTags: () => invoke<TagCount[]>('list_tags'),
  editTask: (id: string, edit: TaskEdit) => invoke<Task>('edit_task', { id, edit }),
  setCompleted: (id: string, done: boolean) => invoke<Task>('set_completed', { id, done }),
  deleteTask: (id: string) => invoke<void>('delete_task', { id }),
  syncFolder: () => invoke<string | null>('get_sync_folder'),
  setSyncFolder: (folder: string | null) => invoke<void>('set_sync_folder', { folder }),
  syncNow: () => invoke<SyncStatus>('sync_now'),
  ready: (marks: Record<string, number>) => invoke<void>('report_ready', { marks }),
};
