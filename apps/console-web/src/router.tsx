import {
  createBrowserHistory,
  createMemoryHistory,
  createRouter,
  type RouterHistory
} from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'

type GetRouterOptions = {
  history?: RouterHistory
}

export function getRouter({ history }: GetRouterOptions = {}) {
  return createRouter({
    history: history ?? (typeof window === 'undefined' ? createMemoryHistory() : createBrowserHistory()),
    routeTree,
    defaultPreload: 'intent',
    defaultPreloadStaleTime: 0,
    scrollRestoration: true
  })
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof getRouter>
  }
}
