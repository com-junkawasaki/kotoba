// Login Page Component (Route Group) - Jsonnet Format
// Next.jsのapp/(auth)/login/page.jsに相当

{
  component: 'LoginPage',
  type: 'page',
  environment: 'client',

  props: {
    title: 'Login',
    redirect_to: '{{redirectTo}}', // クエリパラメータ
  },

  children: [
    {
      component: 'LoginForm',
      type: 'client',
      props: {
        onSubmit: 'handleLogin',
        redirect_to: '{{redirectTo}}',
      },
    },
    {
      component: 'SocialLogin',
      type: 'client',
      props: {
        providers: ['google', 'github', 'twitter'],
      },
    },
    {
      component: 'Link',
      type: 'server',
      props: {
        href: '/register',
        text: 'Don\'t have an account? Sign up',
      },
    },
  ],

  imports: [
    {
      module: 'react',
      specifiers: ['useState', 'useRouter'],
    },
    {
      module: '../../../components/LoginForm',
      specifiers: ['LoginForm'],
      is_default: true,
    },
    {
      module: '../../../components/SocialLogin',
      specifiers: ['SocialLogin'],
      is_default: true,
    },
    {
      module: 'next/navigation',
      specifiers: ['useRouter'],
    },
  ],

  state: {
    email: '',
    password: '',
    is_loading: false,
    error: null,
  },

  hooks: [
    {
      type: 'useState',
      variable: 'email',
      initial_value: '""',
    },
    {
      type: 'useState',
      variable: 'password',
      initial_value: '""',
    },
    {
      type: 'useState',
      variable: 'isLoading',
      initial_value: false,
    },
    {
      type: 'useState',
      variable: 'error',
      initial_value: null,
    },
    {
      type: 'custom',
      name: 'useRouter',
      args: [],
      return_variable: 'router',
    },
  ],

  event_handlers: [
    {
      event_type: 'onSubmit',
      handler_function: 'handleLogin',
      prevent_default: true,
    },
  ],

  metadata: {
    title: 'Login | Kotoba App',
    description: 'Sign in to your Kotoba account',
    robots: {
      index: false,
      follow: false,
    },
  },
}
