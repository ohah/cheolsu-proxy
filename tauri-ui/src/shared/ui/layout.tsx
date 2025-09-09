import { Sidebar } from './sidebar';

export const Layout = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="flex h-full">
      <Sidebar />
      <div className="flex-1 bg-white border border-neutral-300 rounded-xl m-2 overflow-hidden">{children}</div>
    </div>
  );
};
