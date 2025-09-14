// Home Page Component - Jsonnet Format
// Next.jsのapp/page.jsに相当

{
  component: 'HomePage',
  type: 'page',
  environment: 'server',

  props: {
    title: 'Welcome to Kotoba',
    subtitle: 'Next.js風App Router Framework',
  },

  children: [
    {
      component: 'Hero',
      type: 'server',
      props: {
        title: 'Build Modern Web Apps',
        description: 'with Kotoba\'s App Router Framework',
        cta_text: 'Get Started',
        cta_link: '/dashboard',
      },
    },
    {
      component: 'Features',
      type: 'server',
      props: {
        features: [
          {
            title: 'File-based Routing',
            description: 'Define routes with file structure',
            icon: '📁',
          },
          {
            title: 'Server Components',
            description: 'Render on server by default',
            icon: '⚡',
          },
          {
            title: 'Client Components',
            description: 'Interactive components when needed',
            icon: '🎨',
          },
          {
            title: 'Code Splitting',
            description: 'Automatic code splitting and lazy loading',
            icon: '✂️',
          },
        ],
      },
    },
  ],

  imports: [
    {
      module: 'react',
      specifiers: ['Suspense'],
    },
    {
      module: './components/Hero',
      specifiers: ['Hero'],
      is_default: true,
    },
    {
      module: './components/Features',
      specifiers: ['Features'],
      is_default: true,
    },
  ],

  hooks: [
    {
      type: 'useEffect',
      dependencies: [],
      effect_code: 'console.log(\'Home page mounted\')',
    },
  ],

  metadata: {
    title: 'Home | Kotoba App',
    description: 'Welcome to the Kotoba App Router Framework',
    keywords: ['kotoba', 'react', 'framework', 'app router'],
  },
}
