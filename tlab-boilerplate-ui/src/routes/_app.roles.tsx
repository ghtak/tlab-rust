import { createFileRoute, Outlet } from "@tanstack/react-router";
import { pageAccess } from "../features/auth/access";
import { requirePermission } from "../features/auth/routeGuard";

export const Route = createFileRoute("/_app/roles")({
	beforeLoad: ({ context }) =>
		requirePermission(context.user, pageAccess.roles),
	component: Outlet,
});
