import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import prettier from "eslint-config-prettier";

const baseConfig = tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  prettier,
  {
    rules: {
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unsafe-assignment": "error",
      "@typescript-eslint/no-unsafe-call": "error",
      "@typescript-eslint/no-unsafe-member-access": "error",
      "@typescript-eslint/no-unsafe-return": "error",
      "@typescript-eslint/no-unsafe-argument": "error",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/only-throw-error": "error",
      "@typescript-eslint/prefer-promise-reject-errors": "error",

      "@typescript-eslint/explicit-function-return-type": [
        "error",
        {
          allowExpressions: true,
          allowTypedFunctionExpressions: true,
        },
      ],
      "@typescript-eslint/explicit-module-boundary-types": "error",
      "@typescript-eslint/consistent-type-imports": [
        "error",
        {
          prefer: "type-imports",
        },
      ],
      "@typescript-eslint/consistent-type-exports": "error",
      "@typescript-eslint/no-import-type-side-effects": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/strict-boolean-expressions": "error",

      "no-console": [
        "error",
        { allow: ["warn", "error", "group", "groupEnd", "time", "timeEnd"] },
      ],
      "no-debugger": "error",
      "no-alert": "error",
      eqeqeq: ["error", "always"],
      "prefer-const": "error",
      "no-var": "error",
      "no-param-reassign": "error",
      "no-nested-ternary": "error",
      complexity: ["error", 20],
      "max-depth": ["error", 4],
      "max-lines-per-function": ["error", { max: 100, skipBlankLines: true, skipComments: true }],
    },
  },
);

export default baseConfig;

export function createConfig({ tsconfigRootDir, ignores = [] }) {
  return tseslint.config(
    { ignores },
    ...baseConfig,
    {
      languageOptions: {
        parserOptions: {
          projectService: true,
          tsconfigRootDir,
        },
      },
    },
  );
}
