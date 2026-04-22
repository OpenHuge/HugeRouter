export type ProjectSummary = {
  id: string
  name: string
}

export type ControlPlaneClient = {
  listProjects: () => Promise<ProjectSummary[]>
}

export const createControlPlaneClient = (): ControlPlaneClient => ({
  async listProjects() {
    return Promise.resolve([
      {
        id: 'prj_placeholder',
        name: 'Bootstrap Placeholder'
      }
    ])
  }
})

