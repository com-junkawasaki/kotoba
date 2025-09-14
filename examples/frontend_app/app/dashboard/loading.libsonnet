// Dashboard Loading Component - Jsonnet Format
// Next.jsのapp/dashboard/loading.jsに相当

{
  component: 'DashboardLoading',
  type: 'loading',
  environment: 'client',

  children: [
    {
      component: 'LoadingSkeleton',
      type: 'client',
      props: {
        type: 'dashboard',
        items: [
          {
            type: 'card',
            width: '100%',
            height: '200px',
          },
          {
            type: 'grid',
            columns: 3,
            rows: 2,
            item_height: '120px',
          },
          {
            type: 'list',
            items: 5,
            item_height: '60px',
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
      module: '../../components/LoadingSkeleton',
      specifiers: ['LoadingSkeleton'],
      is_default: true,
    },
  ],

  metadata: {
    fallback: true,
    timeout: 3000,
  },
}
