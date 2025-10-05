import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { load } from '@tauri-apps/plugin-store';
import type { SessionStore, SessionStoreState } from '@/entities/session';

const tauriStore = await load('session.json');

const notifyStoreChange = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    console.log('notifyStoreChange');
    await invoke('store_changed_v2');
  } catch (error) {
    console.error('Failed to notify store change:', error);
  }
};

const initialSessions = (await tauriStore.get('sessions')) as SessionStore[];

const useSessionStore = create<SessionStoreState>()(
  subscribeWithSelector((set) => ({
    sessions: (initialSessions as any) ?? ([] as any),
    setSessions: (sessions: SessionStore[]) => set({ sessions }),
    addSession: (session: SessionStore) =>
      set((state) => {
        const sessions = Array.isArray(state.sessions) ? state.sessions : [];
        const existingSessionIndex = sessions.findIndex((s) => s.url === session.url);

        if (existingSessionIndex !== -1) {
          // URL이 같으면 기존 세션을 업데이트
          const updatedSessions = [...sessions];
          updatedSessions[existingSessionIndex] = session;
          return { sessions: updatedSessions };
        } else {
          return { sessions: [...sessions, session] };
        }
      }),
    updateSession: (session: SessionStore) =>
      set((state) => {
        const sessions = Array.isArray(state.sessions) ? state.sessions : [];
        return { sessions: sessions.map((s) => (s.id === session.id ? session : s)) };
      }),
    deleteSession: (id: string) =>
      set((state) => {
        const sessions = Array.isArray(state.sessions) ? state.sessions : [];
        return { sessions: sessions.filter((s) => s.id !== id) };
      }),
    deleteSessionByUrl: (url: string) =>
      set((state) => {
        const sessions = Array.isArray(state.sessions) ? state.sessions : [];
        return { sessions: sessions.filter((s) => s.url !== url) };
      }),
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
