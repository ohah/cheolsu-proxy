const ReactCompilerConfig = {
  // React Compiler 설정
  // 자세한 옵션은 https://react.dev/learn/react-compiler#configuration 참조
};

module.exports = function () {
  return {
    plugins: [
      ['babel-plugin-react-compiler', ReactCompilerConfig], // 반드시 먼저 실행!
      '@babel/plugin-syntax-jsx',
    ],
  };
};
