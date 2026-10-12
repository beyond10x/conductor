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
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  themes: ['@docusaurus/theme-mermaid'],

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
        {to: '/docs/', label: 'Docs', position: 'left'},
        {to: '/docs/security/overview', label: 'Security', position: 'left'},
        {to: '/docs/guides/configure-an-instance', label: 'Guides', position: 'left'},
        {to: '/docs/reference/cli', label: 'CLI', position: 'left'},
        {to: '/docs/status', label: 'Status', position: 'left'},
        {href: 'https://github.com/beyond10x/conductor', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      links: [
        {
          title: 'Docs',
          items: [
            {label: 'Overview', to: '/docs/'},
            {label: 'Getting started', to: '/docs/getting-started'},
            {label: 'Roles', to: '/docs/concepts/roles'},
            {label: 'The loop', to: '/docs/concepts/the-loop'},
            {label: 'Status', to: '/docs/status'},
          ],
        },
        {
          title: 'Security',
          items: [
            {label: 'Overview', to: '/docs/security/overview'},
            {label: 'The guard', to: '/docs/security/the-guard'},
            {label: 'Confinement', to: '/docs/security/confinement'},
            {label: 'Limits', to: '/docs/security/limits'},
            {label: 'Hardening', to: '/docs/security/hardening'},
          ],
        },
        {
          title: 'Guides',
          items: [
            {label: 'Configure an instance', to: '/docs/guides/configure-an-instance'},
            {label: 'Initialise the records', to: '/docs/guides/initialise-the-records'},
            {label: 'Start conductor', to: '/docs/guides/start-conductor'},
            {label: 'Troubleshoot', to: '/docs/guides/troubleshoot'},
          ],
        },
        {
          title: 'Reference',
          items: [
            {label: 'conductor CLI', to: '/docs/reference/cli'},
            {label: 'Config reference', to: '/docs/reference/config'},
            {label: 'Crates', to: '/docs/reference/crates'},
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
