import type { Preview } from '@storybook/react-vite'
import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import { createAppTheme } from '@huge-router/design-tokens'
import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'

const preview: Preview = {
  decorators: [
    (Story) => (
      <MantineProvider theme={createAppTheme()}>
        <ModalsProvider>
          <Notifications />
          <div style={{ padding: 24 }}>
            <Story />
          </div>
        </ModalsProvider>
      </MantineProvider>
    )
  ]
}

export default preview
