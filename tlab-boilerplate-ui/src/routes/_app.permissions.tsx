import { createFileRoute } from "@tanstack/react-router";
import { PermissionPage } from "../features/access/pages/PermissionPage";
import { pageAccess, requirePermission } from "../features/auth/access";

export const Route = createFileRoute("/_app/permissions")({
	beforeLoad: ({ context }) =>
		requirePermission(context.user, pageAccess.permissions),
	component: PermissionPage,
});
