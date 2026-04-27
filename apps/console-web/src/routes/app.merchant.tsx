import { createFileRoute } from "@tanstack/react-router";
import { loadRouteData } from "../features/control-plane/loaders";
import { RouteLoadingState } from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";
import { MerchantCenterPage as MerchantCenterScreen } from "../features/merchant-center/MerchantCenterPage";

export const Route = createFileRoute("/app/merchant")({
  loader: () =>
    loadRouteData(async () => getConsoleDataService().getMerchantWorkspace()),
  pendingComponent: () => (
    <RouteLoadingState label="Loading supplier evidence center" />
  ),
  pendingMs: 0,
  component: MerchantCenterRoute,
});

function MerchantCenterRoute() {
  return <MerchantCenterScreen result={Route.useLoaderData()} />;
}
