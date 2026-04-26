import {
  UiAlert,
  UiButton,
  UiInline,
  UiStack,
  UiText,
  UiHeading,
} from "../primitives";
import type { ReactNode } from "react";

type EmptyStateProps = {
  title: string;
  description: string;
  action?: ReactNode;
};

export function EmptyState({ action, description, title }: EmptyStateProps) {
  return (
    <UiAlert color="teal" radius="md" variant="light">
      <UiStack gap="sm">
        <UiHeading order={4}>{title}</UiHeading>
        <UiText>{description}</UiText>
        <UiInline>
          {action ?? <UiButton variant="light">Placeholder action</UiButton>}
        </UiInline>
      </UiStack>
    </UiAlert>
  );
}
