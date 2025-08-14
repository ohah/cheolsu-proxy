
import React from 'react';

interface MultipleSelectInputProps {
  options: string[];
  selectedOptions: string[];
  onChange: (selected: string[]) => void;
}

const MultipleSelectInput: React.FC<MultipleSelectInputProps> = ({ options, selectedOptions, onChange }) => {

  const handleSelectChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const selected = Array.from(event.target.selectedOptions, option => option.value);
    onChange(selected);
  };

  return (
    <select multiple value={selectedOptions} onChange={handleSelectChange} className="method_filter">
      {options.map(option => (
        <option key={option} value={option}>
          {option}
        </option>
      ))}
    </select>
  );
};

export default MultipleSelectInput;
