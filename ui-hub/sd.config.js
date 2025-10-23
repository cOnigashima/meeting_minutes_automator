import StyleDictionary from 'style-dictionary';

// カスタムtransform: 既存CSS変数名へのマッピング（UIH-REQ-001.3準拠）
StyleDictionary.registerTransform({
  name: 'name/css/legacy',
  type: 'name',
  transform: (token) => {  // v4 API: transformer → transform
    const path = token.path;

    // 既存 src/App.css の8つのCSS変数名にマッピング
    if (path[0] === 'color' && path[1] === 'bg') return '--bg-color';
    if (path[0] === 'color' && path[1] === 'text') return '--text-color';
    if (path[0] === 'color' && path[1] === 'card' && path[2] === 'bg') return '--card-bg';
    if (path[0] === 'color' && path[1] === 'card' && path[2] === 'border') return '--card-border';
    if (path[0] === 'color' && path[1] === 'input' && path[2] === 'bg') return '--input-bg';
    if (path[0] === 'color' && path[1] === 'input' && path[2] === 'border') return '--input-border';
    if (path[0] === 'color' && path[1] === 'input' && path[2] === 'text') return '--input-text';
    if (path[0] === 'color' && path[1] === 'accent' && path[2] === 'primary') return '--accent-color';

    // その他のトークン（space, radius, shadow等）は標準命名
    return '--' + path.filter(p => p !== 'light' && p !== 'dark').join('-');
  }
});

// カスタムformat: @media (prefers-color-scheme: dark)生成（UIH-REQ-001.4準拠）
StyleDictionary.registerFormat({
  name: 'css/variables-with-dark-mode',
  formatter: ({ dictionary }) => {
    const lightVars = [];
    const darkVars = [];

    dictionary.allTokens.forEach(token => {
      const name = token.name; // カスタムtransform適用済み
      const path = token.path;
      const lastSegment = path[path.length - 1];

      if (lastSegment === 'light') {
        lightVars.push(`  ${name}: ${token.value};`);
      } else if (lastSegment === 'dark') {
        darkVars.push(`  ${name}: ${token.value};`);
      } else {
        // light/dark分類なし（accent, space, radius, shadow等）
        lightVars.push(`  ${name}: ${token.value};`);
      }
    });

    let css = ':root {\n' + lightVars.join('\n') + '\n}\n';

    if (darkVars.length > 0) {
      css += '\n@media (prefers-color-scheme: dark) {\n  :root {\n' + darkVars.join('\n') + '\n  }\n}\n';
    }

    return css;
  }
});

export default {
  source: ['tokens/**/*.tokens.json'],
  platforms: {
    css: {
      // カスタムtransformを明示的に適用
      transforms: ['attribute/cti', 'name/css/legacy', 'size/px', 'color/css', 'shadow/css'],
      buildPath: 'src/styles/',
      files: [
        {
          destination: 'tokens.css',
          format: 'css/variables-with-dark-mode'
        }
      ]
    },
    ts: {
      transformGroup: 'js',
      buildPath: 'src/styles/',
      files: [
        {
          destination: 'tokens.d.ts',
          format: 'typescript/es6-declarations'
        }
      ]
    }
  }
};
