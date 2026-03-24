// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// GitHub Pages project site: set ASTRO_BASE_PATH=/<repo>/ in CI (e.g. /runjucks/).
const base = process.env.ASTRO_BASE_PATH ?? '/';

const repoUrl = 'https://github.com/runjucks/runjucks';

const faviconHref =
	(base === '/' ? '' : base.replace(/\/$/, '')) + '/favicon.svg';

// https://astro.build/config
export default defineConfig({
	base,
	site: process.env.ASTRO_SITE ?? undefined,
	integrations: [
		starlight({
			title: 'Runjucks',
			description:
				'Nunjucks-compatible templates with a Rust rendering core, for Node.js.',
			head: [
				{
					tag: 'link',
					attrs: {
						rel: 'icon',
						href: faviconHref,
						type: 'image/svg+xml',
					},
				},
			],
			customCss: ['./src/styles/custom.css'],
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: repoUrl,
				},
			],
			sidebar: [
				{
					label: 'Start here',
					items: [
						{ label: 'Overview', slug: 'index' },
						{ label: 'Installation', slug: 'guides/installation' },
						{ label: 'Syntax and parity', slug: 'guides/syntax' },
						{ label: 'Development', slug: 'guides/development' },
						{ label: 'Architecture', slug: 'guides/architecture' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Node.js API (TypeDoc)', link: 'api/' },
						{ label: 'Rust API (runjucks_core)', link: 'rustdoc/runjucks_core/' },
						{ label: 'Package managers', slug: 'contributing/package-managers' },
					],
				},
				{
					label: 'Contributing',
					items: [
						{ label: 'Overview', slug: 'contributing' },
						{ label: 'Rust API (rustdoc)', slug: 'contributing/rust' },
					],
				},
			],
		}),
	],
});
