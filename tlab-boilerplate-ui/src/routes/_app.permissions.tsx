import { createFileRoute } from "@tanstack/react-router";
import { PermissionPage } from "../features/access/pages/PermissionPage";
import { pageAccess } from "../features/auth/access";
import { requirePermission } from "../features/auth/routeGuard";

export const Route = createFileRoute("/_app/permissions")({
	beforeLoad: ({ context }) =>
		requirePermission(context.user, pageAccess.permissions),
	component: PermissionPage,
});
