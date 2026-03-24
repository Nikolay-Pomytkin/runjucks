// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// GitHub Pages project site: set ASTRO_BASE_PATH=/<repo>/ in CI (e.g. /runjucks/).
const base = process.env.ASTRO_BASE_PATH ?? '/';

// https://astro.build/config
export default defineConfig({
	base,
	site: process.env.ASTRO_SITE ?? undefined,
	integrations: [
		starlight({
			title: 'Runjucks',
			description:
				'Nunjucks-compatible templates with a Rust rendering core, for Node.js.',
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com',
				},
			],
			sidebar: [
				{
					label: 'Start here',
					items: [
						{ label: 'Overview', slug: 'index' },
						{ label: 'Installation', slug: 'guides/installation' },
						{ label: 'Development', slug: 'guides/development' },
						{ label: 'Architecture', slug: 'guides/architecture' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Node.js API (TypeDoc)', link: 'api/' },
						{ label: 'Rust crate (rustdoc)', link: 'rustdoc/runjucks_core/' },
						{ label: 'Package managers', slug: 'contributing/package-managers' },
					],
				},
			],
		}),
	],
});
