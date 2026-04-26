import type { Preview } from "@storybook/react-vite";
import { UiProvider } from "@huge-router/ui-kit";
import "../../console-web/src/styles/app.css";

const preview: Preview = {
  decorators: [
    (Story) => (
      <UiProvider>
        <div style={{ padding: 24 }}>
          <Story />
        </div>
      </UiProvider>
    ),
  ],
};

export default preview;
