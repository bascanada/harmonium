import fs from 'fs';
import path from 'path';
import { marked } from 'marked';

export const load = async () => {
    const readmePath = path.resolve('..', 'README.md');
    const readmeContent = fs.readFileSync(readmePath, 'utf-8');
    const htmlContent = await marked(readmeContent);

    return {
        readme: htmlContent
    };
};
