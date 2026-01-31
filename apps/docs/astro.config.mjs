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
						{ label: 'Quick Start', slug: 'quick-start' },
					],
				},
				{
					label: 'Codebase',
					items: [
						{ label: 'Overview', slug: 'codebase' },
						{ label: 'Server', slug: 'codebase/server' },
						{ label: 'Workspace', slug: 'codebase/workspace' },
						{ label: 'Session', slug: 'codebase/session' },
						{ label: 'Applications', slug: 'codebase/application' },
						{ label: 'Tools', slug: 'codebase/tools' },
						{ label: 'Messages', slug: 'codebase/message' },
						{ label: 'Storage', slug: 'codebase/storage' },
						{ label: 'LLM', slug: 'codebase/llm' },
						{ label: 'Bus and Events', slug: 'codebase/bus-events' },
						{ label: 'Desktop App', slug: 'codebase/desktop-app' },
						{ label: 'Tauri Shell', slug: 'codebase/tauri-shell' },
						{ label: 'platform-macos', slug: 'codebase/platform-macos' },
						{ label: 'Extension Host', slug: 'codebase/extension-host' },
					],
				},
			],
		}),
	],
});
