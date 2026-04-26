import {
  UiAlert,
  UiSurface,
  UiInline,
  UiStack,
  UiText,
  UiHeading,
} from "@huge-router/ui-kit";
import type { ReactNode } from "react";

type AuthStatusCardProps = {
  title: string;
  description: string;
  tone?: "blue" | "green" | "orange" | "red";
  actions?: ReactNode;
  children?: ReactNode;
};

export function AuthStatusCard({
  actions,
  children,
  description,
  title,
  tone = "blue",
}: AuthStatusCardProps) {
  return (
    <UiSurface maw={480} padding="xl" radius="lg" shadow="md" w="100%">
      <UiStack>
        <UiAlert color={tone} variant="light">
          <UiStack gap="xs">
            <UiHeading order={3}>{title}</UiHeading>
            <UiText>{description}</UiText>
          </UiStack>
        </UiAlert>
        {children}
        {actions ? <UiInline>{actions}</UiInline> : null}
      </UiStack>
    </UiSurface>
  );
}
