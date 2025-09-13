// Root Layout Component - Jsonnet Format
// Next.jsのapp/layout.jsに相当

local component = import '../../lib/component.libsonnet';
local types = import '../../lib/types.libsonnet';

{
  component: 'RootLayout',
  type: 'layout',
  environment: 'server',

  props: {
    title: 'Kotoba App',
    lang: 'ja',
  },

  children: [
    {
      component: 'Navigation',
      type: 'client',
      props: {
        items: [
          { label: 'Home', href: '/' },
          { label: 'Dashboard', href: '/dashboard' },
          { label: 'Blog', href: '/blog' },
        ],
      },
    },
    {
      component: 'Content',
      type: 'layout',
      children: [],  // ページコンポーネントがここに挿入される
    },
    {
      component: 'Footer',
      type: 'server',
      props: {
        copyright: '© 2024 Kotoba Framework',
      },
    },
  ],

  imports: [
    {
      module: 'react',
      specifiers: ['useState', 'useEffect'],
    },
    {
      module: './components/Navigation',
      specifiers: ['Navigation'],
      is_default: true,
    },
    {
      module: './components/Footer',
      specifiers: ['Footer'],
      is_default: true,
    },
  ],

  metadata: {
    description: 'Root layout for the entire application',
  },
}
