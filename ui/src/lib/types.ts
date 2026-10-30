export type Priority = 'high' | 'normal' | 'low';
export type ViewName = 'today' | 'upcoming' | 'overdue' | 'all' | 'completed';

export interface Task {
  id: string;
  title: string;
  notes: string | null;
  due_date: string | null; // "2026-10-29"
  due_time: string | null; // "15:00:00"
  priority: Priority;
  tags: string[];
  completed_at: string | null;
  created_at: string;
  updated_at: string;
  updated_by: string;
  deleted: boolean;
}

export interface ParsedTask {
  title: string;
  due_date: string | null;
  due_time: string | null;
  priority: Priority;
  tags: string[];
}

export interface TagCount {
  name: string;
  count: number;
}

export interface ListQuery {
  view: ViewName;
  tag: string | null;
  search: string | null;
}

export interface TaskEdit {
  title: string;
  notes: string | null;
  due_date: string | null;
  due_time: string | null;
  priority: Priority;
  tags: string[];
}

export interface SyncStatus {
  configured: boolean;
  exported: number;
  applied: number;
  corrupt_lines: number;
}

export interface ShortcutStatus {
  current: string;
  error: string | null;
}
