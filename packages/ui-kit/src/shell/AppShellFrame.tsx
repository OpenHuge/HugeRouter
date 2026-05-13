import { UiBox, UiInline, UiStack, UiText } from "../primitives";
import type { ReactNode } from "react";

export type ShellNavItem = {
  href: string;
  label: string;
  description: string;
  active?: boolean;
};

type AppShellFrameProps = {
  title: string;
  subtitle: string;
  navItems: ShellNavItem[];
  children: ReactNode;
};

export function AppShellFrame({
  children,
  navItems,
  subtitle,
  title,
}: AppShellFrameProps) {
  return (
    <div className="hr-app-shell">
      <header className="hr-app-shell-header">
        <UiInline h="100%" justify="space-between" wrap="nowrap">
          <UiBox>
            <UiText fw={700}>{title}</UiText>
            <UiText c="dimmed" size="sm">
              {subtitle}
            </UiText>
          </UiBox>
          <UiText c="dimmed" size="sm">
            HugeRouter Console
          </UiText>
        </UiInline>
      </header>
      <aside className="hr-app-shell-nav">
        <UiStack gap="xs">
          {navItems.map((item) => (
            <a
              aria-current={item.active ? "page" : undefined}
              className={item.active ? "hr-nav-link active" : "hr-nav-link"}
              href={item.href}
              key={item.href}
            >
              <strong>{item.label}</strong>
              <span>{item.description}</span>
            </a>
          ))}
        </UiStack>
      </aside>
      <main className="hr-app-shell-main">{children}</main>
    </div>
  );
}
