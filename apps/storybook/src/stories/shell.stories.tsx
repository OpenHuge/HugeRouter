import type { Meta, StoryObj } from '@storybook/react-vite'
import { EmptyState, PageHeader } from '@huge-router/ui-kit'

const meta = {
  title: 'Shell/Bootstrap Surfaces'
} satisfies Meta

export default meta

type Story = StoryObj<typeof meta>

export const HeaderAndEmptyState: Story = {
  render: () => (
    <div style={{ display: 'grid', gap: 16 }}>
      <PageHeader
        description="Shared page chrome used by the bootstrap console routes."
        title="Tenant Overview"
      />
      <EmptyState
        description="This Storybook app exists to validate shared shell primitives before feature integration."
        title="No feature modules yet"
      />
    </div>
  )
}
