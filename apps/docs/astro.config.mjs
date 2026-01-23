// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import logoLight from './src/assets/logo_light.png';
import logoDark from './src/assets/logo_dark.png';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'Cocommand Docs',
			logo: {
				light: logoLight,
				dark: logoDark,
				alt: 'Cocommand',
				replacesTitle: true,
			},
			sidebar: [
				{
					label: 'Start',
					items: [
						{ label: 'Docs Home', slug: '' },
						{ label: 'Spec Overview', slug: 'spec/00-overview' },
						{ label: 'Terminology', slug: 'spec/01-terminology' },
						{ label: 'Execution Model', slug: 'spec/02-execution-model' },
						{ label: 'Workspace', slug: 'spec/03-workspace' },
						{ label: 'Permissions', slug: 'spec/04-permissions' },
						{ label: 'Routing', slug: 'spec/05-routing' },
						{ label: 'Extensions', slug: 'spec/06-extensions' },
						{ label: 'Observability', slug: 'spec/07-observability' },
						{ label: 'UI Shell', slug: 'spec/09-ui-shell' },
						{ label: 'Milestones', slug: 'spec/10-milestones' },
					],
				},
				{
					label: 'Implementation',
					items: [
						{ label: 'Implementation Tasks', slug: 'tasks' },
						{ label: 'Built-in Apps', slug: 'builtins' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Architecture', slug: 'architecture' },
						{ label: 'Backend Server Design', slug: 'backend-server' },
					],
				},
			],
		}),
	],
});
