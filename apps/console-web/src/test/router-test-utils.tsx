import { act, render, type RenderResult } from '@testing-library/react'
import { RouterProvider, createMemoryHistory } from '@tanstack/react-router'
import { authSessionQueryKey, setAnonymousSession } from '../features/auth/auth-queries'
import { getQueryClient } from '../lib/query-client'
import { getRouter } from '../router'

type RenderRouteOptions = {
  waitForLoad?: boolean
}

type RenderRouteResult = {
  history: ReturnType<typeof createMemoryHistory>
  rendered: RenderResult
  router: ReturnType<typeof getRouter>
}

export async function renderRoute(
  path: string,
  { waitForLoad = true }: RenderRouteOptions = {}
): Promise<RenderRouteResult> {
  const queryClient = getQueryClient()

  if (!queryClient.getQueryData(authSessionQueryKey)) {
    setAnonymousSession(queryClient)
  }

  const history = createMemoryHistory({
    initialEntries: [path]
  })
  const router = getRouter({
    history
  })
  const rendered = render(<RouterProvider router={router} />)

  if (waitForLoad) {
    await act(async () => {
      await router.load()
    })
  }

  return {
    history,
    rendered,
    router
  }
}

export function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  let reject!: (reason?: unknown) => void

  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })

  return {
    promise,
    reject,
    resolve
  }
}
