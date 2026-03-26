// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Production (CI): ASTRO_SITE=https://nikolay.pomytkin.com ASTRO_BASE_PATH=/runjucks/
const base = process.env.ASTRO_BASE_PATH ?? '/';

const repoUrl = 'https://github.com/Nikolay-Pomytkin/runjucks';

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
					],
				},
				{
					label: 'Usage',
					items: [
						{ label: 'Template language', slug: 'guides/syntax' },
						{ label: 'JavaScript API', slug: 'guides/javascript-api' },
						{ label: 'Performance', slug: 'guides/performance' },
						{ label: 'Limitations', slug: 'guides/limitations' },
					],
				},
				{
					label: 'Contributing',
					items: [
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
					label: 'Contributing (advanced)',
					items: [
						{ label: 'Overview', slug: 'contributing' },
						{ label: 'Rust API (rustdoc)', slug: 'contributing/rust' },
					],
				},
			],
		}),
	],
});
