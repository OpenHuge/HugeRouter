import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'

const ignores = [
  '**/dist/**',
  '**/.output/**',
  '**/coverage/**',
  '**/storybook-static/**',
  '**/node_modules/**',
  '**/routeTree.gen.ts'
]

export default tseslint.config(
  { ignores },
  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked,
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node
      },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname
      }
    },
    rules: {
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/no-floating-promises': 'error'
    }
  }
)

