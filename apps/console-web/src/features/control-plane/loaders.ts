export type RouteDataResult<T> =
  | {
      state: 'error'
    }
  | {
      data: T
      state: 'success'
    }

export async function loadRouteData<T>(
  load: () => Promise<T>
): Promise<RouteDataResult<T>> {
  try {
    const data = await load()

    return {
      data,
      state: 'success'
    }
  } catch {
    return {
      state: 'error'
    }
  }
}
