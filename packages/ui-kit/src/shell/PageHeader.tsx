import { UiInline, UiStack, UiText, UiHeading } from "../primitives";
import type { ReactNode } from "react";

type PageHeaderProps = {
  title: string;
  description: string;
  actions?: ReactNode;
};

export function PageHeader({ actions, description, title }: PageHeaderProps) {
  return (
    <UiInline align="flex-start" justify="space-between" mb="lg">
      <UiStack gap={4}>
        <UiHeading order={2}>{title}</UiHeading>
        <UiText c="dimmed" maw={720}>
          {description}
        </UiText>
      </UiStack>
      {actions}
    </UiInline>
  );
}
