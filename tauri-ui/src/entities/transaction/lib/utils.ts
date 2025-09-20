export const getStatusColor = (status: number): string => {
  if (status >= 200 && status < 300) return 'bg-green-100 text-green-800 border-green-200';
  if (status >= 300 && status < 400) return 'bg-blue-100 text-blue-800 border-blue-200';
  if (status >= 400 && status < 500) return 'bg-yellow-100 text-yellow-800 border-yellow-200';
  if (status >= 500) return 'bg-red-100 text-red-800 border-red-200';
  return 'bg-gray-100 text-gray-800 border-gray-200';
};

export const getMethodColor = (method: string) => {
  switch (method) {
    case 'GET':
      return 'bg-blue-100 text-blue-800 border-blue-200';
    case 'POST':
      return 'bg-green-100 text-green-800 border-green-200';
    case 'PUT':
      return 'bg-yellow-100 text-yellow-800 border-yellow-200';
    case 'DELETE':
      return 'bg-red-100 text-red-800 border-red-200';
    default:
      return 'bg-gray-100 text-gray-800 border-gray-200';
  }
};
