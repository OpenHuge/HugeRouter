import { type RenderOptions, type RenderResult } from '@testing-library/react';
import { QueryClient } from '@tanstack/react-query';
import type { PropsWithChildren, ReactElement } from 'react';
export declare function createTestQueryClient(): QueryClient;
type TestProvidersProps = PropsWithChildren<{
    queryClient?: QueryClient;
}>;
export declare function TestProviders({ children, queryClient }: TestProvidersProps): import("react/jsx-runtime").JSX.Element;
type ExtendedRenderOptions = Omit<RenderOptions, 'wrapper'> & {
    queryClient?: QueryClient;
};
export declare function renderWithProviders(ui: ReactElement, { queryClient, ...options }?: ExtendedRenderOptions): RenderResult;
export {};
