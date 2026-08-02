import eslintPluginVue from 'eslint-plugin-vue';
import ts from 'typescript-eslint';
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting';

export default ts.config(
  ...ts.configs.recommended,
  ...eslintPluginVue.configs['flat/recommended'],
  {
    files: ['*.vue', '**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: '@typescript-eslint/parser',
      },
    },
    rules: {
      'vue/multi-word-component-names': 'off',
      'vue/attributes-order': [
        'warn',
        {
          alphabetical: true,
          sortLineLength: false,
        },
      ],
    },
  },
  skipFormatting,
);
