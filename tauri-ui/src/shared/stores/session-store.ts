import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { load } from '@tauri-apps/plugin-store';
import type { SessionStore, SessionStoreState } from './types';

const tauriStore = await load('session.json', { autoSave: true });

const notifyStoreChange = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('store_changed');
  } catch (error) {
    console.error('Failed to notify store change:', error);
  }
};

const useSessionStore = create<SessionStoreState>()(
  subscribeWithSelector((set) => ({
    sessions: (tauriStore.get('sessions') as never as SessionStore[]) ?? ([] as SessionStore[]),
    setSessions: (sessions: SessionStore[]) => set({ sessions }),
    addSession: (session: SessionStore) => set((state) => ({ sessions: [...state.sessions, session] })),
    updateSession: (session: SessionStore) =>
      set((state) => ({ sessions: state.sessions.map((s) => (s.id === session.id ? session : s)) })),
    deleteSession: (id: string) => set((state) => ({ sessions: state.sessions.filter((s) => s.id !== id) })),
    deleteSessionByUrl: (url: string) => set((state) => ({ sessions: state.sessions.filter((s) => s.url !== url) })),
  })),
);

useSessionStore.subscribe(
  (state) => state.sessions,
  async (sessions) => {
    try {
      await tauriStore.set('sessions', sessions);
      await tauriStore.save();
      await notifyStoreChange();
    } catch (error) {
      console.error('Auto-save failed:', error);
    }
  },
);

export { useSessionStore };
