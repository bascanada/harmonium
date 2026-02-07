module.exports = {
	root: true,
	ignorePatterns: ['build/', 'dist/', 'pkg/', '.svelte-kit/', 'node_modules/'],
	extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended'],
	parser: '@typescript-eslint/parser',
	plugins: ['@typescript-eslint'],
	rules: {
		'@typescript-eslint/no-explicit-any': 'error'
	}
};
