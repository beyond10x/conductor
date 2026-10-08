import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import {withProductSite} from '@beyond10x/docs-system/product-site';

const config: Config = {
  title: 'Conductor',
  tagline: 'A control loop for agent sessions across the repositories of one organization.',
  favicon: 'img/mark.svg',

  future: {
    v4: true,
  },

  url: 'https://beyond10x.github.io',
  baseUrl: '/conductor/',

  organizationName: 'beyond10x',
  projectName: 'conductor',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',

  markdown: {
    format: 'detect',
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: 'https://github.com/beyond10x/conductor/tree/main/website/',
        },
        blog: false,
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'Conductor',
      items: [
        {to: '/docs/', label: 'Documentation', position: 'left'},
        {to: '/docs/getting-started', label: 'Get started', position: 'left'},
        {to: '/docs/reference/cli', label: 'CLI', position: 'left'},
        {to: '/docs/status', label: 'Status', position: 'left'},
        {href: 'https://github.com/beyond10x/conductor', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      links: [
        {
          title: 'Documentation',
          items: [
            {label: 'Overview', to: '/docs/'},
            {label: 'Getting started', to: '/docs/getting-started'},
            {label: 'The loop', to: '/docs/concepts/the-loop'},
            {label: 'The guard', to: '/docs/concepts/the-guard'},
            {label: 'Configure an instance', to: '/docs/guides/configure-an-instance'},
            {label: 'conductor CLI', to: '/docs/reference/cli'},
            {label: 'Config reference', to: '/docs/reference/config'},
            {label: 'Status', to: '/docs/status'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'Source', href: 'https://github.com/beyond10x/conductor'},
            {label: 'Design', href: 'https://github.com/beyond10x/conductor/blob/main/docs/design/conductor.md'},
          ],
        },
      ],
      copyright: 'A beyond10x project. Conductor · Apache-2.0.',
    },
  } satisfies Preset.ThemeConfig,
};

export default withProductSite(config, {landing: './product.json', mark: 'Cd'});
