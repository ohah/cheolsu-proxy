const ReactCompilerConfig = {
  // React Compiler 설정
  // 자세한 옵션은 https://react.dev/learn/react-compiler#configuration 참조
};

module.exports = function (api) {
  // Babel 캐싱 설정
  api.cache(true);

  return {
    presets: [
      ['@babel/preset-env', { targets: { node: 'current' } }],
      ['@babel/preset-typescript', { isTSX: true, allExtensions: true }],
      ['@babel/preset-react', { runtime: 'automatic' }],
    ],
    plugins: [
      ['babel-plugin-react-compiler', ReactCompilerConfig], // 반드시 먼저 실행!
    ],
  };
};
