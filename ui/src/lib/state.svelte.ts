import { api } from './api';
import { marks } from './marks';
import { describeSync, errorText, nowParts } from './format';
import type { TagCount, Task, TaskEdit, ViewName } from './types';

export const VIEWS: { name: ViewName; label: string }[] = [
  { name: 'today', label: '오늘' },
  { name: 'upcoming', label: '예정' },
  { name: 'overdue', label: '지난 일' },
  { name: 'all', label: '전체' },
  { name: 'completed', label: '완료됨' },
];

const SYNC_INTERVAL_MS = 5 * 60 * 1000;
const SYNC_DEBOUNCE_MS = 2000;
const SEARCH_DEBOUNCE_MS = 150;
const TOAST_MS = 5000;

class AppState {
  view = $state<ViewName>('today');
  tag = $state<string | null>(null);
  search = $state('');
  tasks = $state<Task[]>([]);
  tags = $state<TagCount[]>([]);
  selectedId = $state<string | null>(null);
  doneOpen = $state(false);
  error = $state<string | null>(null);
  undo = $state<{ id: string; title: string } | null>(null);
  syncFolder = $state<string | null>(null);
  syncMessage = $state<string | null>(null);
  syncing = $state(false);

  selected = $derived(this.tasks.find((t) => t.id === this.selectedId) ?? null);
  openTasks = $derived(this.tasks.filter((t) => !t.completed_at));
  doneTasks = $derived(this.tasks.filter((t) => !!t.completed_at));
  /** 본 목록: 완료됨 보기에서는 완료 항목, 그 밖에는 미완료 항목. */
  mainTasks = $derived(this.view === 'completed' && !this.searching ? this.doneTasks : this.openTasks);
  /** ↑/↓로 오갈 수 있는 순서. */
  navTasks = $derived(
    this.view === 'completed' && !this.searching
      ? this.doneTasks
      : this.doneOpen
        ? [...this.openTasks, ...this.doneTasks]
        : this.openTasks,
  );

  private refreshSeq = 0;
  private resync = false;
  private syncTimer: ReturnType<typeof setTimeout> | undefined;
  private searchTimer: ReturnType<typeof setTimeout> | undefined;
  private undoTimer: ReturnType<typeof setTimeout> | undefined;
  private errorTimer: ReturnType<typeof setTimeout> | undefined;

  get searching(): boolean {
    return this.search.trim() !== '';
  }

  async init() {
    try {
      this.syncFolder = await api.syncFolder();
    } catch (e) {
      this.fail(e);
    }
    marks.synced_folder = performance.now();
    await this.refresh();
    marks.data = performance.now();
    void this.runSync();
    setInterval(() => void this.runSync(), SYNC_INTERVAL_MS);
    marks.ready = performance.now();
    void api.ready(marks).catch(() => {});
  }

  async refresh() {
    const seq = ++this.refreshSeq;
    try {
      const [tasks, tags] = await Promise.all([
        api.listTasks({
          view: this.searching ? 'all' : this.view,
          tag: this.tag,
          search: this.searching ? this.search.trim() : null,
        }),
        api.listTags(),
      ]);
      if (seq !== this.refreshSeq) return; // 더 최근 요청이 있으면 버린다
      this.tasks = tasks;
      this.tags = tags;
      if (this.selectedId && !this.navTasks.some((t) => t.id === this.selectedId)) this.selectedId = null;
    } catch (e) {
      this.fail(e);
    }
  }

  toggleDoneOpen() {
    this.doneOpen = !this.doneOpen;
    if (!this.doneOpen && this.selectedId && !this.navTasks.some((t) => t.id === this.selectedId)) {
      this.selectedId = null;
    }
  }

  setView(view: ViewName) {
    this.view = view;
    this.tag = null;
    this.search = '';
    this.selectedId = null;
    void this.refresh();
  }

  setTag(name: string) {
    this.tag = this.tag === name ? null : name;
    this.view = 'all';
    this.search = '';
    this.selectedId = null;
    void this.refresh();
  }

  onSearchInput() {
    clearTimeout(this.searchTimer);
    this.searchTimer = setTimeout(() => void this.refresh(), SEARCH_DEBOUNCE_MS);
  }

  clearSearch() {
    this.search = '';
    void this.refresh();
  }

  async addFromInput(text: string): Promise<Task | null> {
    try {
      const task = await api.addTask(text);
      await this.refresh();
      if (this.tasks.some((t) => t.id === task.id)) this.selectedId = task.id;
      this.scheduleSync();
      return task;
    } catch (e) {
      this.fail(e);
      return null;
    }
  }

  async toggle(id: string) {
    const task = this.tasks.find((t) => t.id === id);
    if (!task) return;
    const done = !task.completed_at;
    try {
      await api.setCompleted(id, done);
      if (done) this.showUndo({ id, title: task.title });
      await this.refresh();
      this.scheduleSync();
    } catch (e) {
      this.fail(e);
    }
  }

  toggleSelected() {
    if (this.selectedId) void this.toggle(this.selectedId);
  }

  async undoComplete() {
    const pending = this.undo;
    if (!pending) return;
    this.undo = null;
    try {
      await api.setCompleted(pending.id, false);
      await this.refresh();
      this.scheduleSync();
    } catch (e) {
      this.fail(e);
    }
  }

  /** 저장에 성공하면 true. 실패하면 화면이 원래 값으로 되돌릴 수 있게 false. */
  async save(id: string, edit: TaskEdit): Promise<boolean> {
    try {
      await api.editTask(id, edit);
      await this.refresh();
      this.scheduleSync();
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    }
  }

  async remove(id: string) {
    if (!confirm('이 할 일을 삭제할까요?')) return;
    try {
      await api.deleteTask(id);
      if (this.selectedId === id) this.selectedId = null;
      await this.refresh();
      this.scheduleSync();
    } catch (e) {
      this.fail(e);
    }
  }

  removeSelected() {
    if (this.selectedId) void this.remove(this.selectedId);
  }

  moveSelection(delta: 1 | -1) {
    const list = this.navTasks;
    if (list.length === 0) return;
    const i = list.findIndex((t) => t.id === this.selectedId);
    const next = i === -1 ? (delta === 1 ? 0 : list.length - 1) : Math.min(list.length - 1, Math.max(0, i + delta));
    this.selectedId = list[next].id;
  }

  async setSyncFolder(folder: string | null) {
    try {
      await api.setSyncFolder(folder);
      this.syncFolder = folder;
      this.syncMessage = null;
      if (folder) await this.runSync();
    } catch (e) {
      this.fail(e);
    }
  }

  async runSync() {
    if (!this.syncFolder) return;
    if (this.syncing) {
      this.resync = true; // 진행 중인 동기화가 끝나면 한 번 더 돌린다
      return;
    }
    this.syncing = true;
    try {
      const status = await api.syncNow();
      if (status.configured) {
        this.syncMessage = describeSync(status, nowParts().time);
        if (status.applied > 0) await this.refresh();
      }
    } catch (e) {
      this.syncMessage = `동기화 실패: ${errorText(e)}`;
    } finally {
      this.syncing = false;
      if (this.resync) {
        this.resync = false;
        void this.runSync();
      }
    }
  }

  private scheduleSync() {
    if (!this.syncFolder) return;
    clearTimeout(this.syncTimer);
    this.syncTimer = setTimeout(() => void this.runSync(), SYNC_DEBOUNCE_MS);
  }

  private showUndo(undo: { id: string; title: string }) {
    this.undo = undo;
    clearTimeout(this.undoTimer);
    this.undoTimer = setTimeout(() => (this.undo = null), TOAST_MS);
  }

  private fail(e: unknown) {
    this.error = errorText(e);
    clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => (this.error = null), TOAST_MS);
  }
}

export const app = new AppState();
