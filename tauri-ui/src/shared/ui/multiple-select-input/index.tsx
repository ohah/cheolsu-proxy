import type { HttpMethod } from '../../stores';

interface MultipleSelectInputProps {
  options: readonly HttpMethod[];
  selectedOptions: HttpMethod[];
  onChange: (selected: HttpMethod[]) => void;
}

export const MultipleSelectInput = ({ options, selectedOptions, onChange }: MultipleSelectInputProps) => {
  const handleSelectChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const selected = Array.from(event.target.selectedOptions, (option) => option.value);
    onChange(selected as HttpMethod[]);
  };

  return (
    <select multiple value={selectedOptions} onChange={handleSelectChange} className="method_filter">
      {options.map((option) => (
        <option key={option} value={option}>
          {option}
        </option>
      ))}
    </select>
  );
};
