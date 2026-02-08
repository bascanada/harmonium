import fs from 'fs';
import path from 'path';
import { marked } from 'marked';

export const load = async () => {
	const readmePath = path.resolve('..', 'README.md');
	const readmeContent = fs.readFileSync(readmePath, 'utf-8');

	const renderer = new marked.Renderer();
	renderer.code = ({ text, lang }) => {
		if (lang === 'mermaid') {
			return `<div class="mermaid">${text}</div>`;
		}
		return `<pre><code class="language-${lang}">${text}</code></pre>`;
	};

	const htmlContent = await marked(readmeContent, { renderer });

	return {
		readme: htmlContent
	};
};
