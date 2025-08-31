import { logo } from '../shared/assets';

export const Sidebar = () => {
  return (
    <div className="flex flex-col p-2 h-full items-center">
      <img src={logo} alt="d" className="w-[60px] h-[60px] object-cover" />
    </div>
  );
};
