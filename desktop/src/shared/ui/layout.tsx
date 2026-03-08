import { Sidebar } from "./sidebar";

export const Layout = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="flex h-full select-none">
      <Sidebar />
      <div className="flex-1 bg-background border border-border rounded-xl m-2 overflow-hidden">
        {children}
      </div>
    </div>
  );
};
